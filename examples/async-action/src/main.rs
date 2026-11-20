//! The async-action lifecycle, end to end, with **zero reflection code** — ordinary components are
//! action-aware because their handlers call `ctx.defer()`.
//!
//! Open http://127.0.0.1:9008. Five things to try, each proving one property of the contract:
//!
//! 1. **Deferred button (succeeds)** — spins in place, then clears when the work finishes.
//! 2. **Deferred button (fails)** — spins, then turns destructive with the error on a tooltip.
//! 3. **Immediate action** — mutates and returns without deferring; it *never* spins.
//! 4. **Per-instance rows** — same handler, one `param` each; only the row you click spins.
//! 5. **Form** — its submit button reflects the same way, for free.
//!
//! Plus the opt-in `ActionToasts` notification, which pops on any failure and clears on retry.

use std::error::Error;
use std::thread;
use std::time::Duration;

use rwire::capsule_gen::CapsuleConfig;
use rwire::form::{Field, Form};
use rwire::style_tokens::St;
use rwire::theme::Theme;
use rwire::{handler, renderer, theme, ElementBuilder, Server, State};
use rwire_components::{ActionToasts, Button, Gap, Stack, StackAlign, StackJustify, Text};
use rwire_themes::palettes;

/// Simulated bridge-worker latency.
const WORK_MS: u64 = 1500;

/// Per-connection state. `clicks` backs the immediate-action demo; the deferred handlers don't
/// use it (their whole effect is the deferral), but the `#[handler]` macro requires a state param.
#[derive(State, Default)]
struct Ui {
    clicks: u32,
}

// ============================================================================
// Handlers
// ============================================================================

/// Deferred + succeeds: the button spins, then clears.
#[handler]
fn run_good(_ui: &mut Ui, ctx: &EventContext) {
    let resolver = ctx.defer();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(WORK_MS));
        resolver.ok();
    });
}

/// Deferred + fails: the button spins, then turns destructive with the message on a tooltip.
#[handler]
fn run_bad(_ui: &mut Ui, ctx: &EventContext) {
    let resolver = ctx.defer();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(WORK_MS));
        resolver.err("the working tree is dirty — refusing to merge");
    });
}

/// Immediate: mutates and returns without deferring. No action entry is ever created, so the
/// button never spins — it just does its thing in the same round-trip.
#[handler]
fn bump(ui: &mut Ui) {
    ui.clicks += 1;
}

/// Deferred, one per row: distinguished only by the button's `param`. Clicking a row marks
/// `(handler_id, that param)` Loading, so only that row's button reflects.
#[handler]
fn deliver_row(_ui: &mut Ui, ctx: &EventContext) {
    let resolver = ctx.defer();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(WORK_MS));
        resolver.ok();
    });
}

/// A form submit that defers, so the Save button reflects like any other action-aware control.
#[handler]
fn save_form(_ui: &mut Ui, ctx: &EventContext) {
    let resolver = ctx.defer();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(WORK_MS));
        resolver.ok();
    });
}

// ============================================================================
// UI — every control is plain; the reflection is automatic
// ============================================================================

#[theme]
fn app_theme() -> Theme {
    Theme::dark().palette(palettes::nord())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("async-action on http://0.0.0.0:9008");
    Server::bind("0.0.0.0:9008")?
        .root(app)
        .capsule_config(CapsuleConfig::new())
        .theme(app_theme())
        .run()
        .await
}

fn app() -> ElementBuilder {
    Stack::column()
        .gap(Gap::Lg)
        .align_center()
        .justify(StackJustify::Center)
        .children([
            Text::heading2("async-action — the lifecycle primitive").build(),
            Text::body("Every control below is plain. Because its handler defers, it reflects.")
                .muted()
                .build(),
            Stack::column()
                .gap(Gap::Lg)
                .children([
                    deferred_section(),
                    immediate_section(),
                    per_instance_section(),
                    form_section(),
                ])
                .build()
                .st([St::WFull, St::MaxWMd]),
            // A failed action slides in a toast (fixed, bottom-right); it clears on retry.
            ActionToasts::new().build(),
        ])
        .build()
        .st([St::MinHDvh, St::BgApp, St::PLg])
}

/// Sections 1 & 2 — a deferred action that succeeds and one that fails.
fn deferred_section() -> ElementBuilder {
    section(
        "Deferred",
        [
            action_row(
                "Deliver (succeeds)",
                Button::primary("Deliver").on_click(run_good()),
            ),
            action_row(
                "Deliver (fails)",
                Button::primary("Deliver").on_click(run_bad()),
            ),
        ],
    )
}

/// Section 3 — an immediate action. The button never spins; the count updates in the same click.
fn immediate_section() -> ElementBuilder {
    section(
        "Immediate (never spins)",
        [action_row("Bump the counter", bump_control())],
    )
}

fn bump_control() -> ElementBuilder {
    Stack::row()
        .gap(Gap::Md)
        .align(StackAlign::Center)
        .children([click_count(), Button::secondary("Bump").on_click(bump())])
        .build()
}

#[renderer]
fn click_count(ui: &Ui) -> ElementBuilder {
    Text::body_small(format!("{} clicks", ui.clicks))
        .muted()
        .build()
}

/// Section 4 — N rows of the same handler, each with its own `param`; only the clicked row spins.
fn per_instance_section() -> ElementBuilder {
    let rows = (1..=3).map(|i| {
        let spec = deliver_row().with_param_bytes(vec![i]);
        action_row(
            format!("Request #{i}"),
            Button::primary("Deliver").on_click(spec),
        )
    });
    section_rows(
        "Per-instance (same handler, one param each)",
        rows.collect(),
    )
}

/// Section 5 — a plain `Form`; its submit button reflects its `action()` handler.
fn form_section() -> ElementBuilder {
    section(
        "Form (submit reflects too)",
        [Form::new()
            .action(save_form())
            .fields([
                Field::text("name")
                    .label("Name")
                    .placeholder("Ada Lovelace"),
                Field::submit("Save"),
            ])
            .build()
            .st([St::PMd])],
    )
}

// ── Layout helpers ───────────────────────────────────────────────────────────

fn section(title: &'static str, rows: impl IntoIterator<Item = ElementBuilder>) -> ElementBuilder {
    section_rows(title, rows.into_iter().collect())
}

fn section_rows(title: &'static str, rows: Vec<ElementBuilder>) -> ElementBuilder {
    Stack::column()
        .gap(Gap::Xs)
        .children([
            Text::body_small(title)
                .build()
                .st([St::TextMuted, St::PxSm]),
            Stack::column().children(rows).build().st([
                St::BgSurface,
                St::RoundedMd,
                St::BorderDefault,
            ]),
        ])
        .build()
}

fn action_row(
    label: impl Into<std::borrow::Cow<'static, str>>,
    control: ElementBuilder,
) -> ElementBuilder {
    Stack::row()
        .justify(StackJustify::Between)
        .align(StackAlign::Center)
        .gap(Gap::Md)
        .children([Text::body_small(label).build(), control])
        .build()
        .st([St::PMd, St::BorderB])
}
