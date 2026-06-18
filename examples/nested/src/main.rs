//! Nested-renderer example: a synced region that contains another synced region.
//!
//! [`render_outer`] is a `#[renderer]` whose content embeds [`render_inner`], a
//! second `#[renderer]` bound to a *different* state. This exercises the nested
//! renderer runtime path end-to-end:
//!
//! - Bumping the OUTER state re-renders the parent, which must re-emit the nested
//!   child wrapper (`CREATE_SYNCED`) so the child keeps its value and stays
//!   updatable afterward (the historical bug destroyed the child wrapper here).
//! - Bumping the INNER state updates only the child region.

use std::error::Error;

use async_std::main;
use rwire::capsule_gen::CapsuleConfig;
use rwire::theme::Theme;
use rwire::{ElementBuilder, Server, St, State, handler, renderer, theme};
use rwire_components::{
    Button, ButtonSize, Card, Container, ContainerSize, Gap, Stack, Text, TextColor,
};
use rwire_themes::palettes;

#[derive(State, Default)]
#[storage(memory)]
struct Outer {
    ticks: i32,
}

#[derive(State, Default)]
#[storage(memory)]
struct Inner {
    count: i32,
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

/// Parent synced region; its content embeds the nested child region.
#[renderer]
fn render_outer(state: &Outer) -> ElementBuilder {
    Stack::column()
        .gap(Gap::Sm)
        .align_center()
        .children([
            Text::body(format!("outer ticks: {}", state.ticks)).build(),
            // NESTED synced region, bound to a different state.
            render_inner(),
        ])
        .build()
}

/// Child synced region, nested inside [`render_outer`].
#[renderer]
fn render_inner(state: &Inner) -> ElementBuilder {
    Text::heading1(format!("inner count: {}", state.count))
        .color(TextColor::Accent)
        .build()
}

#[handler]
fn bump_outer(state: &mut Outer) {
    state.ticks += 1;
}

#[handler]
fn bump_inner(state: &mut Inner) {
    state.count += 1;
}
