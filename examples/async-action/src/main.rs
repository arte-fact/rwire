//! The async-action lifecycle with **zero reflection code** — ordinary `Button`s are action-aware.
//!
//! Open http://127.0.0.1:9008. Each button is just `Button::primary("Deliver").on_click(handler())`.
//! Its handler hands the work to a background thread with `ctx.defer()` and returns immediately;
//! `defer()` marks the action `Loading`, so the button spins in place (constant footprint) —
//! pushed to the browser with no further client event. ~1.5s later the worker calls
//! `resolver.ok()` (button returns to idle) or `resolver.err(msg)` (button turns destructive,
//! message on a hover tooltip, still clickable as retry).
//!
//! No `reflect_action`, no key, no loading/error markup here: because the handler defers, the
//! `Button` reflects the whole lifecycle automatically. A handler that never defers leaves its
//! button a plain button — the reflection is free and complete.

use std::error::Error;
use std::thread;
use std::time::Duration;

use async_std::main;
use rwire::capsule_gen::CapsuleConfig;
use rwire::style_tokens::St;
use rwire::theme::Theme;
use rwire::{handler, theme, ElementBuilder, Server, State};
use rwire_components::{ActionErrors, Button, Gap, Stack, StackAlign, StackJustify, Text};
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
// UI — plain buttons; the reflection is automatic
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
            Text::body(
                "Plain buttons. Each defers its work; the result is pushed back ~1.5s later.",
            )
            .muted()
            .build(),
            demo(),
            // Opt-in page-level slot: lists any failed action, clears on retry.
            Stack::column()
                .child(ActionErrors::new().build())
                .build()
                .st([St::WFull, St::MaxWMd]),
        ])
        .build()
        .st([St::MinHDvh, St::BgApp, St::PLg])
}

fn demo() -> ElementBuilder {
    Stack::column()
        .gap(Gap::Md)
        .children([
            action_row(
                "Deliver (succeeds)",
                Button::primary("Deliver").on_click(run_good()),
            ),
            action_row(
                "Deliver (fails)",
                Button::primary("Deliver").on_click(run_bad()),
            ),
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
