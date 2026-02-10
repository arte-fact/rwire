//! Counter example - minimal reactive UI with rwire components.

use std::error::Error;

use async_std::main;
use rwire::capsule_gen::CapsuleConfig;
use rwire::theme::Theme;
use rwire::{handler, renderer, theme, ElementBuilder, Server, St, State};
use rwire_components::{
    Button, ButtonSize, Card, Container, ContainerSize, Gap, Stack, Text, TextColor,
};
use rwire_themes::palettes;

#[derive(State, Default)]
#[storage(memory)]
struct Counter {
    count: i32,
}

#[theme]
fn app_theme() -> Theme {
    Theme::dark().palette(palettes::nord())
}

#[main]
async fn main() -> Result<(), Box<dyn Error>> {
    Server::bind("127.0.0.1:9000")?
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
                                    Text::heading2("Counter").build(),
                                    render_count(),
                                    Stack::row()
                                        .gap(Gap::Md)
                                        .children([
                                            Button::secondary("-")
                                                .size(ButtonSize::Lg)
                                                .on_click(decrement()),
                                            Button::primary("+")
                                                .size(ButtonSize::Lg)
                                                .on_click(increment()),
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

#[renderer]
fn render_count(state: &Counter) -> ElementBuilder {
    Text::heading1(state.count.to_string())
        .color(TextColor::Accent)
        .build()
}

#[handler]
fn increment(state: &mut Counter) {
    state.count += 1;
}

#[handler]
fn decrement(state: &mut Counter) {
    state.count -= 1;
}
