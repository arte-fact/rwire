//! Dashboard UI: a left column of selectable hardware cards (CPU + GPUs) and a
//! right column for LLM nodes (Phase 1: just the "+" card).
//!
//! The whole layout is a single synced region ([`render_dashboard`]) bound to the
//! shared [`App`] state, so it re-reads live hardware *and* the current selection
//! together. Clicking a card toggles selection (single material class at a time);
//! unchanged renders are dropped by the framework's content-hash dedup.

// Rust guideline compliant 2026-02-21

use rwire::{El, ElementBuilder, Ev, IterWithRef, St, Style, el, renderer};
use rwire_components::{
    Badge, Button, ButtonSize, Card, Container, ContainerSize, Gap, Stack, StackJustify, Text,
};

mod handlers;
use handlers::{
    add_node, delete_node, duplicate_node, kill_orphan, launch_node, load_profile, toggle_select,
};

mod cards;
use cards::{cpu_tile, gpu_tile};

mod dashboard;
use dashboard::{confirm_bar, confirm_trigger, mono_block, running_dashboard};

mod form;
pub use form::*;

use crate::models::DiscoveredModel;
use crate::snapshot::{
    App, CPU_KEY, DeviceInfo, LlmNode, MaterialClass, NodePhase, NodeStats, OrphanProc, Selection,
};

/// Sentinel option value meaning "the user wants to type a custom value/flag".
pub const CUSTOM: &str = "__custom__";

/// Build the static page shell wrapping the two synced columns.
///
/// The hardware and node columns are **separate** synced regions so the 1 s
/// hardware poll (which broadcasts only the `hw` field) re-renders the left
/// column without disturbing the node column or its inputs.
pub fn app() -> ElementBuilder {
    Container::new()
        .size(ContainerSize::Full)
        .padding(true)
        .child(
            Stack::column()
                .gap(Gap::Lg)
                .children([
                    // Vertical stack: title, a compact full-width hardware status
                    // strip, then the node card grid. Each region wraps internally
                    // (flex-wrap), so the whole page reflows from multi-column on
                    // desktop to single-column on mobile with no fixed side rail.
                    Text::heading1("Node Monitor").build(),
                    render_hardware(),
                    render_nodes(),
                ])
                .build(),
        )
        .build()
}

/// Top hardware status strip: one compact, selectable tile per CPU/GPU that
/// flexes across the row and wraps on narrow screens. Depends on `hw` +
/// `selection`, so it re-renders on each poll tick and on selection changes.
#[renderer]
fn render_hardware(app: &App) -> ElementBuilder {
    let hw = &app.hw;
    let sel = &app.selection;

    let mut tiles = Vec::with_capacity(1 + hw.gpus.len());
    tiles.push(selectable(cpu_tile(hw), CPU_KEY, MaterialClass::Cpu, sel));
    for gpu in &hw.gpus {
        tiles.push(selectable(gpu_tile(gpu), &gpu.key(), gpu.class(), sel));
    }
    el(El::Div)
        .st([St::DisplayFlex, St::FlexWrap, St::GapMd])
        .append(tiles)
}

/// Node workspace: full-width controls (orphans, profile loader) then a
/// responsive card grid — the "+" card and compact node cards tile 2-3 across
/// on desktop and stack 1-wide on mobile, while a node being configured spans
/// the full row so its form has room. Reads `node_stats` for live tiles; DOM
/// morphing keeps inputs stable across the metrics poll.
#[renderer]
fn render_nodes(app: &App) -> ElementBuilder {
    let sel = &app.selection;
    let mut sections = Vec::new();
    if !app.orphans.is_empty() {
        sections.push(orphan_card(&app.orphans));
    }
    if let Some(bar) = load_profile_bar(&app.profiles) {
        sections.push(bar);
    }

    let mut grid = vec![grid_item(plus_card(sel.locked_class()), false)];
    for (item_ref, node) in app.nodes.iter_with_ref() {
        let stats = app.node_stats.get(&node.id);
        let full = matches!(node.phase, NodePhase::Configuring);
        grid.push(grid_item(
            node_view(node, item_ref, &app.models, &app.devices, stats),
            full,
        ));
    }
    sections.push(
        el(El::Div)
            .st([St::DisplayFlex, St::FlexWrap, St::ItemsStart, St::GapMd])
            .append(grid),
    );

    Stack::column().gap(Gap::Md).children(sections).build()
}

/// Wrap a card as a grid cell. `full` cards take their own full-width row (the
/// config form); others flex to ~26rem so 2-3 tile per row, 1 on mobile.
fn grid_item(card: ElementBuilder, full: bool) -> ElementBuilder {
    let basis = if full { "1 1 100%" } else { "1 1 26rem" };
    el(El::Div)
        .style(Style::new().set("flex", basis).min_width("0"))
        .append([card])
}

/// A warning card listing engine processes the UI doesn't manage, each with a
/// Kill control to free their VRAM. Shown only when orphans are detected.
fn orphan_card(orphans: &[OrphanProc]) -> ElementBuilder {
    let mut body = vec![
        Stack::row()
            .gap(Gap::Sm)
            .align_center()
            .children([
                Text::heading3("Orphan processes".to_string()).build(),
                Badge::error(orphans.len().to_string()).build(),
            ])
            .build(),
        Text::caption("Engine processes not managed by any node — kill to free VRAM.".to_string())
            .muted()
            .build(),
    ];
    for o in orphans {
        body.push(
            Stack::row()
                .justify(StackJustify::Between)
                .align_center()
                .children([
                    el(El::Span)
                        .st([St::FontMono])
                        .style(Style::new().set("font-size", "0.8rem"))
                        .text(&format!("PID {} · {}", o.pid, o.label)),
                    Button::secondary("Kill")
                        .size(ButtonSize::Sm)
                        .build()
                        .data("pid", &o.pid.to_string())
                        .on(Ev::Click, kill_orphan()),
                ])
                .build(),
        );
    }
    el(El::Div)
        .st([St::PMd, St::RoundedLg])
        .style(
            Style::new()
                .border("1px solid var(--O10)")
                .background("var(--a)"),
        )
        .append([Stack::column().gap(Gap::Sm).children(body).build()])
}

/// A "Load profile" dropdown that spawns a node from a saved profile. Returns
/// `None` when no profiles exist yet.
fn load_profile_bar(profiles: &[String]) -> Option<ElementBuilder> {
    if profiles.is_empty() {
        return None;
    }
    let mut opts = vec![el(El::Option).attr("value", "").text("Load profile…")];
    for name in profiles {
        opts.push(el(El::Option).attr("value", name).text(name));
    }
    Some(
        Stack::row()
            .gap(Gap::Sm)
            .align_center()
            .children([
                Text::label("Saved".to_string()).build(),
                input_box("load-profile", El::Select)
                    .append(opts)
                    .on(Ev::Change, load_profile()),
            ])
            .build(),
    )
}

/// Wrap a hardware card with selection chrome: a bright ring when selected, a
/// dimmed/disabled look when another class is locked, and click-to-toggle.
///
/// The ring/dim are data-driven per-card visual state, so they use the inline
/// `Style` builder (like [`cards::core_bars`]) rather than utility classes.
fn selectable(
    card: ElementBuilder,
    key: &str,
    class: MaterialClass,
    sel: &Selection,
) -> ElementBuilder {
    let selected = sel.contains(key);
    let disabled = matches!(sel.locked_class(), Some(locked) if locked != class);

    let mut card = card.data("key", key);
    if selected {
        // Bright frost ring on the selected card.
        card = card.style(
            Style::new()
                .set("box-shadow", "0 0 0 3px var(--n9)")
                .set("border-radius", "var(--R3)"),
        );
    } else if disabled {
        card = card.style(Style::new().opacity("0.4"));
    }

    if disabled {
        card.st([St::PointerEventsNone, St::CursorNotAllowed])
    } else {
        card.st([St::CursorPointer]).on(Ev::Click, toggle_select())
    }
}

/// The hollow "+" card that creates an LLM node. Active only once a material
/// class is selected; inert (dimmed) otherwise. Click wiring lands in Phase 2.
fn plus_card(locked: Option<MaterialClass>) -> ElementBuilder {
    let hint = match locked {
        Some(MaterialClass::Nvidia) => "New NVIDIA node",
        Some(MaterialClass::Amd) => "New AMD node",
        Some(MaterialClass::Cpu) => "New CPU node",
        None => "Select hardware to start",
    };
    // Hollow dashed border via the typed Style builder (brighter when active).
    let border = if locked.is_some() {
        "2px dashed var(--n7)"
    } else {
        "2px dashed var(--i)"
    };
    let card = el(El::Div)
        .st([
            St::DisplayFlex,
            St::FlexCol,
            St::ItemsCenter,
            St::JustifyCenter,
            St::GapSm,
            St::PLg,
            St::RoundedLg,
            St::MinH6rem,
            St::WFull,
            St::TextCenter,
        ])
        .style(Style::new().border(border))
        .append([
            Text::heading2("+").build(),
            Text::caption(hint.to_string()).muted().build(),
        ]);

    if locked.is_some() {
        card.attr("id", "plus-card")
            .st([St::CursorPointer])
            .on(Ev::Click, add_node())
    } else {
        card.st([St::Opacity50])
    }
}

/// Shared visual style for form controls: subtle border, padded, rounded, with
/// an optional max-width so short values don't stretch across the whole card.
fn field_style(max_width: Option<&str>) -> Style {
    let mut s = Style::new()
        .border("1px solid var(--g)")
        .background("var(--b)")
        .set("color", "var(--k)")
        .set("padding", "0.45rem 0.6rem")
        .set("font-size", "0.85rem")
        .set("line-height", "1.2");
    if let Some(w) = max_width {
        s = s.max_width(w);
    }
    s
}

/// A styled form control with a stable `id` (rwire restores focus/cursor across
/// re-renders by id): monospace (values are CLI tokens), a focus ring, and a
/// hover-brightened border. `max_width` caps short inputs.
fn input_box(id: &str, el_type: El) -> ElementBuilder {
    input_box_w(id, el_type, None)
}

/// [`input_box`] with an explicit max-width cap.
fn input_box_w(id: &str, el_type: El, max_width: Option<&str>) -> ElementBuilder {
    el(el_type)
        .attr("id", id)
        .st([St::WFull, St::FontMono, St::RoundedSm, St::TransitionColors])
        .style(field_style(max_width))
        .focus([St::RingFocus])
        .hover([St::BorderColorAccent])
}

/// A text input with an explicit max-width cap.
fn text_field_w(
    id: &str,
    placeholder: &str,
    value: &str,
    max_width: Option<&str>,
) -> ElementBuilder {
    input_box_w(id, El::Input, max_width)
        .attr("type", "text")
        .attr("placeholder", placeholder)
        .attr("value", value)
}

/// A node card. The header (title, phase badge, delete) is always shown; the body
/// depends on the lifecycle phase: a config form while `Configuring`, status +
/// stop while `Starting`/`Running`, and a message + retry/delete otherwise.
fn node_view(
    node: &LlmNode,
    item_ref: rwire::ItemRef<LlmNode>,
    models: &[DiscoveredModel],
    devices: &[DeviceInfo],
    stats: Option<&NodeStats>,
) -> ElementBuilder {
    // Title block: "Node #N · CLASS" + phase badge, with a compact subtitle
    // line (model · :port · engine · devices) so the body stays uncluttered.
    let mut title_rows = vec![
        Stack::row()
            .gap(Gap::Sm)
            .align_center()
            .children([
                Text::heading3(format!("Node #{} · {}", node.id, node.material.label())).build(),
                phase_badge(&node.phase),
            ])
            .build(),
    ];
    if let Some(sub) = node_subtitle(node) {
        title_rows.push(Text::caption(sub).muted().build());
    }
    let header = Stack::row()
        .justify(StackJustify::Between)
        .align_center()
        .children([
            Stack::column().gap(Gap::Xs).children(title_rows).build(),
            Stack::row()
                .gap(Gap::Sm)
                .children([
                    Button::ghost("Duplicate")
                        .size(ButtonSize::Sm)
                        .build()
                        .on_ref(Ev::Click, duplicate_node(), item_ref),
                    Button::ghost("Delete").size(ButtonSize::Sm).build().on_ref(
                        Ev::Click,
                        delete_node(),
                        item_ref,
                    ),
                ])
                .build(),
        ])
        .build();

    let mut body = vec![header];

    match &node.phase {
        NodePhase::Configuring => configuring_body(node, item_ref, models, devices, &mut body),
        NodePhase::Starting => {
            body.push(Text::body("Starting… loading model".to_string()).build());
            body.push(action_row(vec![
                confirm_trigger("Configure", "configure", item_ref),
                confirm_trigger("Stop", "stop", item_ref),
            ]));
        }
        NodePhase::Running => running_dashboard(node, item_ref, models, stats, &mut body),
        NodePhase::Stopped => {
            body.push(Text::body("Stopped.".to_string()).muted().build());
            body.push(action_row(vec![
                relaunch_button(item_ref, "Relaunch"),
                confirm_trigger("Configure", "configure", item_ref),
            ]));
        }
        NodePhase::Error(msg) => {
            body.push(error_view(msg));
            body.push(action_row(vec![
                relaunch_button(item_ref, "Retry"),
                confirm_trigger("Configure", "configure", item_ref),
            ]));
        }
    }
    // Confirmation bar for any pending stop/configure (Starting/Stopped/Error;
    // the running dashboard renders its own inline above the blocks).
    if !matches!(node.phase, NodePhase::Running | NodePhase::Configuring)
        && let Some(bar) = confirm_bar(node, item_ref)
    {
        body.push(bar);
    }

    Card::new()
        .child(Stack::column().gap(Gap::Sm).children(body).build())
        .build()
}

/// The compact header subtitle for a launched node: model · :port · engine ·
/// devices. `None` while configuring (the form shows those fields).
fn node_subtitle(node: &LlmNode) -> Option<String> {
    if matches!(node.phase, NodePhase::Configuring) {
        return None;
    }
    let mut parts = Vec::new();
    parts.push(if node.model.is_empty() {
        "no model".to_string()
    } else {
        short_path(&node.model).to_string()
    });
    if node.port != 0 {
        parts.push(format!(":{}", node.port));
    }
    if let Some(e) = node.engine {
        parts.push(e.label().to_string());
    }
    parts.push(if node.material == MaterialClass::Cpu {
        "host CPU".to_string()
    } else {
        node.devices.join(", ")
    });
    Some(parts.join(" · "))
}

/// A small horizontal row of action buttons.
fn action_row(buttons: Vec<ElementBuilder>) -> ElementBuilder {
    Stack::row()
        .gap(Gap::Sm)
        .align_center()
        .children(buttons)
        .build()
}

/// Render an error message (with any child-log tail) in a scrollable mono block.
fn error_view(msg: &str) -> ElementBuilder {
    mono_block(msg, "var(--O10)")
}

/// A colored badge for a node's lifecycle phase.
fn phase_badge(phase: &NodePhase) -> ElementBuilder {
    match phase {
        NodePhase::Configuring => Badge::default_badge("configuring".to_string()).build(),
        NodePhase::Starting => Badge::warning("starting".to_string()).build(),
        NodePhase::Running => Badge::success("running".to_string()).build(),
        NodePhase::Stopped => Badge::default_badge("stopped".to_string()).build(),
        NodePhase::Error(_) => Badge::error("error".to_string()).build(),
    }
}

/// A relaunch/retry control bound to the node.
fn relaunch_button(item_ref: rwire::ItemRef<LlmNode>, label: &'static str) -> ElementBuilder {
    Button::primary(label)
        .size(ButtonSize::Sm)
        .build()
        .on_ref(Ev::Click, launch_node(), item_ref)
}
