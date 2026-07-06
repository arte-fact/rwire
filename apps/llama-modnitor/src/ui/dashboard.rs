//! The running-node mini dashboard: stat tiles, the Logs/Chat/Bench blocks, and
//! the confirm bar for lifecycle actions.

// Rust guideline compliant 2026-02-21

use super::handlers::{
    ask_confirm, cancel_confirm, clear_chat, confirm_action, refresh_logs, run_bench, send_chat,
    toggle_bench, toggle_chat, toggle_logs, update_chat_input,
};
use super::{action_row, effective_ctx, node_model, text_field_w};
use rwire::{El, ElementBuilder, Ev, St, Style, el};
use rwire_components::{Button, ButtonSize, Gap, Stack, StackJustify, Text};

use crate::models::DiscoveredModel;
use crate::snapshot::{LlmNode, NodeStats};

/// The running-node mini dashboard: a serving header, a tab bar (Overview /
/// Logs / Chat), the active tab's content, and the Stop/Configure actions.
pub fn running_dashboard(
    node: &LlmNode,
    item_ref: rwire::ItemRef<LlmNode>,
    models: &[DiscoveredModel],
    stats: Option<&NodeStats>,
    body: &mut Vec<ElementBuilder>,
) {
    // Live stats come from `App.node_stats` (absent until the first scrape).
    let default = NodeStats::default();
    let stats = stats.unwrap_or(&default);
    // Mini dashboard on top, always visible.
    body.push(stat_tiles(node, models, stats));

    // Left: show/hide toggles for the optional blocks. Right: lifecycle actions.
    let toggle = |label: &'static str, on: bool, h| {
        let btn = if on {
            Button::primary(label)
        } else {
            Button::ghost(label)
        };
        btn.size(ButtonSize::Sm)
            .build()
            .on_ref(Ev::Click, h, item_ref)
    };
    body.push(split_row(
        vec![
            toggle("Logs", node.view.logs, toggle_logs()),
            toggle("Chat", node.view.chat, toggle_chat()),
            toggle("Bench", node.view.bench, toggle_bench()),
        ],
        vec![
            confirm_trigger("Configure", "configure", item_ref),
            confirm_trigger("Stop", "stop", item_ref),
        ],
    ));
    if let Some(bar) = confirm_bar(node, item_ref) {
        body.push(bar);
    }

    if node.view.logs {
        logs_block(node, item_ref, body);
    }
    if node.view.chat {
        chat_block(node, item_ref, body);
    }
    if node.view.bench {
        bench_block(node, item_ref, body);
    }
}

/// A row with a left group and a right group, pushed to opposite edges.
pub fn split_row(left: Vec<ElementBuilder>, right: Vec<ElementBuilder>) -> ElementBuilder {
    Stack::row()
        .justify(StackJustify::Between)
        .align_center()
        .children([
            Stack::row()
                .gap(Gap::Sm)
                .align_center()
                .children(left)
                .build(),
            Stack::row()
                .gap(Gap::Sm)
                .align_center()
                .children(right)
                .build(),
        ])
        .build()
}

/// A button that requests confirmation (`data-action`) before its real action.
pub fn confirm_trigger(
    label: &'static str,
    action: &str,
    item_ref: rwire::ItemRef<LlmNode>,
) -> ElementBuilder {
    Button::secondary(label)
        .size(ButtonSize::Sm)
        .build()
        .data("action", action)
        .on_ref(Ev::Click, ask_confirm(), item_ref)
}

/// The confirmation bar shown while a node has a pending action.
pub fn confirm_bar(node: &LlmNode, item_ref: rwire::ItemRef<LlmNode>) -> Option<ElementBuilder> {
    let action = node.pending.as_deref()?;
    Some(
        el(El::Div)
            .st([St::PSm, St::RoundedSm])
            .style(
                Style::new()
                    .border("1px solid var(--A10)")
                    .background("var(--a)"),
            )
            .append([Stack::row()
                .gap(Gap::Sm)
                .align_center()
                .children([
                    Text::body(format!("Confirm {action}?")).build(),
                    Button::primary("Confirm")
                        .size(ButtonSize::Sm)
                        .build()
                        .on_ref(Ev::Click, confirm_action(), item_ref),
                    Button::ghost("Cancel").size(ButtonSize::Sm).build().on_ref(
                        Ev::Click,
                        cancel_confirm(),
                        item_ref,
                    ),
                ])
                .build()]),
    )
}

/// The KV-cache tile value as **used / total in k tokens**. Total is the
/// context window (KV capacity); used comes from the engine's live usage ratio
/// when available (vLLM, or llama.cpp builds that expose it), otherwise "—".
pub fn kv_tile_value(node: &LlmNode, models: &[DiscoveredModel], stats: &NodeStats) -> String {
    let meta = node_model(node, models).and_then(|m| m.meta.as_ref());
    let total = effective_ctx(node, meta);
    if total == 0 {
        return "—".to_string();
    }
    let total_k = f64::from(total) / 1000.0;
    stats.kv_usage.map_or_else(
        || format!("— / {total_k:.0}k tok"),
        |ratio| {
            let used_k = f64::from(ratio * crate::convert::u32_f32(total)) / 1000.0;
            format!("{used_k:.1}k / {total_k:.0}k tok")
        },
    )
}

/// Live serving metrics as a grid of tiles (the always-on mini dashboard).
pub fn stat_tiles(node: &LlmNode, models: &[DiscoveredModel], stats: &NodeStats) -> ElementBuilder {
    let opt_tps = |v: Option<f32>| v.map_or_else(|| "—".into(), |x| format!("{x:.0}"));
    let opt_n = |v: Option<u32>| v.map_or_else(|| "—".into(), |x| x.to_string());
    let tiles = [
        ("Prefill tok/s", opt_tps(stats.prefill_tps)),
        ("Decode tok/s", opt_tps(stats.decode_tps)),
        ("KV cache", kv_tile_value(node, models, stats)),
        ("Running", opt_n(stats.running)),
        ("Queued", opt_n(stats.waiting)),
    ];
    let children: Vec<ElementBuilder> = tiles
        .into_iter()
        .map(|(label, value)| {
            el(El::Div)
                .st([St::PSm, St::RoundedSm])
                .style(
                    Style::new()
                        .background("var(--a)")
                        .set("flex", "1 1 7rem")
                        .min_width("0"),
                )
                .append([
                    Text::caption(label.to_string()).muted().build(),
                    Text::heading3(value).build(),
                ])
        })
        .collect();
    el(El::Div)
        .st([St::DisplayFlex, St::FlexWrap, St::GapSm])
        .append(children)
}

/// Optional server-log block: a label + refresh + the tail.
pub fn logs_block(
    node: &LlmNode,
    item_ref: rwire::ItemRef<LlmNode>,
    body: &mut Vec<ElementBuilder>,
) {
    body.push(action_row(vec![
        Text::label("Server log".to_string()).build(),
        Button::secondary("Refresh")
            .size(ButtonSize::Sm)
            .build()
            .on_ref(Ev::Click, refresh_logs(), item_ref),
    ]));
    let text = if node.log_tail.is_empty() {
        "(no logs yet — press Refresh)"
    } else {
        &node.log_tail
    };
    body.push(mono_block(text, "var(--g)"));
}

/// Optional multi-turn test-chat block: the conversation, an input + Send, Clear.
pub fn chat_block(
    node: &LlmNode,
    item_ref: rwire::ItemRef<LlmNode>,
    body: &mut Vec<ElementBuilder>,
) {
    // Conversation transcript (user right-ish/bright, assistant muted).
    if !node.chat_log.is_empty() {
        let turns: Vec<ElementBuilder> = node
            .chat_log
            .iter()
            .map(|t| {
                let is_user = t.role == "user";
                let who = if is_user { "you" } else { "assistant" };
                el(El::Div)
                    .st([St::PSm, St::RoundedSm])
                    .style(
                        Style::new()
                            .background(if is_user { "var(--a)" } else { "var(--b)" })
                            .border("1px solid var(--g)"),
                    )
                    .append([
                        Text::caption(who.to_string()).muted().build(),
                        el(El::Div)
                            .st([St::WhitespacePreWrap, St::BreakWords])
                            .text(&t.content),
                    ])
            })
            .collect();
        body.push(
            el(El::Div)
                .st([
                    St::DisplayFlex,
                    St::FlexCol,
                    St::GapSm,
                    St::RoundedSm,
                    St::OverflowAuto,
                ])
                .style(Style::new().set("max-height", "22rem"))
                .append(turns),
        );
    }
    body.push(
        Stack::row()
            .gap(Gap::Sm)
            .align_center()
            .children([
                text_field_w(
                    &format!("chat-{}", node.id),
                    "Ask the model…",
                    &node.chat_input,
                    None,
                )
                .on_ref(Ev::Input, update_chat_input(), item_ref),
                Button::primary("Send").size(ButtonSize::Sm).build().on_ref(
                    Ev::Click,
                    send_chat(),
                    item_ref,
                ),
                Button::ghost("Clear").size(ButtonSize::Sm).build().on_ref(
                    Ev::Click,
                    clear_chat(),
                    item_ref,
                ),
            ])
            .build(),
    );
}

/// Optional benchmark block: a Run button + a tile grid of pp/tg results.
pub fn bench_block(
    node: &LlmNode,
    item_ref: rwire::ItemRef<LlmNode>,
    body: &mut Vec<ElementBuilder>,
) {
    let run_label = if node.bench_running {
        "Running…"
    } else {
        "Run benchmark"
    };
    let mut run = Button::primary(run_label).size(ButtonSize::Sm).build();
    if node.bench_running {
        run = run.st([St::Opacity50, St::CursorNotAllowed]);
    } else {
        run = run.on_ref(Ev::Click, run_bench(), item_ref);
    }
    body.push(action_row(vec![
        Text::label("Benchmark (pp / tg, tokens/s)".to_string()).build(),
        run,
    ]));
    if !node.bench_results.is_empty() {
        let tiles: Vec<ElementBuilder> = node
            .bench_results
            .iter()
            .map(|(label, value)| {
                el(El::Div)
                    .st([St::PSm, St::RoundedSm])
                    .style(
                        Style::new()
                            .background("var(--a)")
                            .set("flex", "1 1 6rem")
                            .min_width("0"),
                    )
                    .append([
                        Text::caption(label.clone()).muted().build(),
                        Text::heading3(value.clone()).build(),
                    ])
            })
            .collect();
        body.push(
            el(El::Div)
                .st([St::DisplayFlex, St::FlexWrap, St::GapSm])
                .append(tiles),
        );
    }
}

/// A scrollable monospace text block (logs, chat replies).
pub fn mono_block(text: &str, border: &str) -> ElementBuilder {
    el(El::Pre)
        .st([
            St::FontMono,
            St::RoundedSm,
            St::PSm,
            St::WhitespacePreWrap,
            St::BreakWords,
            St::OverflowAuto,
            St::M0,
        ])
        .style(
            Style::new()
                .background("var(--b)")
                .border(&format!("1px solid {border}"))
                .set("color", "var(--k)")
                .set("font-size", "0.75rem")
                .set("max-height", "16rem"),
        )
        .text(text)
}
