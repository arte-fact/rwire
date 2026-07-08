//! The async-action lifecycle, end to end, on the real rwire primitive.
//!
//! Open http://127.0.0.1:9008. Each **Deliver** button is bound to a handler that hands its work
//! to a background thread with `ctx.defer()`:
//!
//! - the handler returns immediately, but `defer()` marks the action `Loading`, so the button
//!   spins in place (constant footprint) — pushed to the browser with **no further client event**;
//! - ~1.5s later the background thread calls `resolver.ok()` (the button returns to idle) or
//!   `resolver.err(msg)` (a ⚠ badge appears, the message on a hover tooltip).
//!
//! The buttons carry **no reflection code and no key** — `reflect_action(handler_id, param, …)`
//! keys off the same `(handler_id, param)` the binding already carries. Making an ordinary
//! `Button` do this automatically (no `reflect_action` call at all) is M3.

use std::error::Error;
use std::thread;
use std::time::Duration;

use async_std::main;
use rwire::capsule_gen::CapsuleConfig;
use rwire::style_tokens::St;
use rwire::theme::Theme;
use rwire::{handler, reflect_action, theme, ElementBuilder, HandlerSpec, Server, State};
use rwire_components::{Badge, Button, Gap, Stack, StackAlign, StackJustify, Text, Tooltip};
use rwire_themes::palettes;

/// Simulated bridge-worker latency.
const WORK_MS: u64 = 1500;

/// The handlers need a state param (the `#[handler]` macro requires one); they don't use it —
/// all their effect is the deferral. A real app would pass whatever domain state it mutates.
#[derive(State, Default)]
struct Ui {}

// ============================================================================
// Handlers — defer to a background thread, resolve when it finishes
// ============================================================================

#[handler]
fn run_good(_ui: &mut Ui, ctx: &EventContext) {
    let resolver = ctx.defer();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(WORK_MS));
        resolver.ok();
    });
}

#[handler]
fn run_bad(_ui: &mut Ui, ctx: &EventContext) {
    let resolver = ctx.defer();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(WORK_MS));
        resolver.err("the working tree is dirty — refusing to merge");
    });
}

// ============================================================================
// UI
// ============================================================================

#[theme]
fn app_theme() -> Theme {
    Theme::dark().palette(palettes::nord())
}

#[main]
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
            Text::body("Each Deliver defers its work; the result is pushed back ~1.5s later.")
                .muted()
                .build(),
            demo(),
        ])
        .build()
        .st([St::MinHDvh, St::BgApp, St::PLg])
}

fn demo() -> ElementBuilder {
    Stack::column()
        .gap(Gap::Md)
        .children([
            action_row("Deliver (succeeds)", deliver(run_good)),
            action_row("Deliver (fails)", deliver(run_bad)),
        ])
        .build()
        .st([St::WFull, St::MaxWMd])
}

fn action_row(label: &'static str, control: ElementBuilder) -> ElementBuilder {
    Stack::row()
        .justify(StackJustify::Between)
        .align(StackAlign::Center)
        .gap(Gap::Md)
        .children([Text::body_small(label).build(), control])
        .build()
        .st([St::BgSurface, St::PMd, St::BorderB])
}

/// An action-aware control: reflect the action's phase for `spec`'s `(handler_id, param)`.
///
/// This is the whole integration a caller writes today. In every phase the button keeps a
/// constant footprint — loading spins in place, the error is a fixed-size ⚠ badge in a reserved
/// slot with the message on a tooltip — so the row never reflows. The button stays clickable in
/// the idle and errored phases, so it doubles as retry. M3 folds all of this into `Button` itself.
fn deliver(spec_fn: fn() -> HandlerSpec) -> ElementBuilder {
    let spec = spec_fn();
    let param = spec.param_bytes.clone().unwrap_or_default();
    reflect_action(
        spec.handler_id,
        param,
        move || control_row(None, Button::primary("Deliver").on_click(spec_fn())),
        || {
            control_row(
                None,
                Button::primary("Deliver")
                    .loading(true)
                    .disabled(true)
                    .build(),
            )
        },
        move |msg| {
            control_row(
                Some(warn(msg)),
                Button::primary("Deliver").on_click(spec_fn()),
            )
        },
    )
}

/// A fixed-width status slot (so its content can't move the button) beside the control.
fn control_row(status: Option<ElementBuilder>, button: ElementBuilder) -> ElementBuilder {
    let slot = Stack::row()
        .justify(StackJustify::Center)
        .children(status)
        .build()
        .st([St::W2rem]);
    Stack::row()
        .gap(Gap::Sm)
        .align(StackAlign::Center)
        .children([slot, button])
        .build()
}

/// The failure affordance: a compact ⚠ badge carrying the full message on a hover tooltip.
fn warn(message: &str) -> ElementBuilder {
    Tooltip::new(message.to_string())
        .child(Badge::error("⚠").build())
        .build()
}
