//! Nested-renderer example: a synced region that contains another synced region.
//!
//! [`render_outer`] is a `#[renderer]` whose content embeds [`render_inner`], a
//! second `#[renderer]`. Both read the same `Demo` state but depend on different
//! fields. This exercises the nested renderer runtime path end-to-end:
//!
//! - `outer +` bumps `ticks` -> only the parent re-renders; the nested child keeps
//!   its value and stays updatable afterward.
//! - `inner +` bumps `count` -> only the child region updates.
//! - `row +` / `row -` change `rows`, which BOTH renderers read - so the parent
//!   re-renders AND the nested list grows/shrinks. Shrinking must drop the removed
//!   rows from the DOM: regression cover for the nested-region removal bug (a parent
//!   re-render used to discard the nested child's update, so a shrinking child never
//!   removed its dropped rows).

use std::error::Error;

use async_std::main;
use rwire::capsule_gen::CapsuleConfig;
use rwire::theme::Theme;
use rwire::{ElementBuilder, Server, St, State, handler, renderer, theme};
use rwire_components::{
    Button, ButtonSize, Card, Container, ContainerSize, Gap, Stack, Text, TextColor,
};
use rwire_themes::palettes;

/// Single shared state: the parent renderer depends on `ticks`, the nested one on
/// `count`/`rows`. `rows` is read by both, so changing it re-renders the parent and
/// resizes the nested list.
#[derive(State, Default)]
#[storage(memory)]
struct Demo {
    ticks: i32,
    count: i32,
    /// Number of list rows the nested region renders; shrinking this must remove
    /// the dropped rows from the DOM (the bug under test).
    rows: i32,
}

#[theme]
fn app_theme() -> Theme {
    Theme::dark().palette(palettes::nord())
}

#[main]
async fn main() -> Result<(), Box<dyn Error>> {
    Server::bind("0.0.0.0:7778")?
        .root(app)
        .capsule_config(CapsuleConfig::new())
        .theme(app_theme())
        .run()
        .await
}

fn app() -> ElementBuilder {
    Container::new()
        .size(ContainerSize::Full)
        .padding(true)
        .child(
            Stack::centered()
                .child(
                    Card::new()
                        .child(
                            Stack::column()
                                .gap(Gap::Lg)
                                .align_center()
                                .children([
                                    Text::heading2("Nested renderers").build(),
                                    render_outer(),
                                    Stack::row()
                                        .gap(Gap::Md)
                                        .children([
                                            Button::secondary("row +")
                                                .size(ButtonSize::Lg)
                                                .on_click(add_row()),
                                            Button::secondary("row -")
                                                .size(ButtonSize::Lg)
                                                .on_click(drop_row()),
                                            Button::secondary("inner +")
                                                .size(ButtonSize::Lg)
                                                .on_click(bump_inner()),
                                            Button::primary("outer +")
                                                .size(ButtonSize::Lg)
                                                .on_click(bump_outer()),
                                        ])
                                        .build(),
                                ])
                                .build(),
                        )
                        .build(),
                )
                .build(),
        )
        .build()
        .st([St::BgApp, St::MinHScreen])
}

/// Parent synced region. Reads `ticks` and `rows`, so changing `rows` re-renders
/// the parent too - the case that used to discard the nested child's update.
#[renderer]
fn render_outer(state: &Demo) -> ElementBuilder {
    Stack::column()
        .gap(Gap::Sm)
        .align_center()
        .children([
            Text::body(format!("outer ticks: {} (rows={})", state.ticks, state.rows)).build(),
            // NESTED synced region, bound to the same state but different fields.
            render_inner(),
        ])
        .build()
}

/// Child synced region, nested inside [`render_outer`]. Renders a list whose length
/// tracks `rows`; shrinking it must drop the removed rows from the DOM.
#[renderer]
fn render_inner(state: &Demo) -> ElementBuilder {
    let mut children = vec![
        Text::heading1(format!("inner count: {}", state.count))
            .color(TextColor::Accent)
            .build(),
    ];
    for i in 0..state.rows.max(0) {
        children.push(Text::body(format!("row {i}")).build());
    }
    Stack::column()
        .gap(Gap::Xs)
        .align_center()
        .children(children)
        .build()
}

#[handler]
fn bump_outer(state: &mut Demo) {
    state.ticks += 1;
}

#[handler]
fn bump_inner(state: &mut Demo) {
    state.count += 1;
}

#[handler]
fn add_row(state: &mut Demo) {
    state.rows += 1;
}

#[handler]
fn drop_row(state: &mut Demo) {
    if state.rows > 0 {
        state.rows -= 1;
    }
}
