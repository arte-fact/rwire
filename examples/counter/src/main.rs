//! Counter example - demonstrates rwire reactive state API.
//!
//! A simple counter component with increment/decrement buttons.
//! Only the count display re-renders when state changes.
//! Uses typed style tokens for compact wire representation.
//! Styled with Nord theme colors via configurable palette.

use rwire::capsule_gen::CapsuleConfig;
use rwire::theme::Theme;
use rwire::{el, handler, renderer, ColorPalette, El, ElementBuilder, Ev, Server, St, State};

#[derive(State, Default)]
#[storage(memory)]
struct Counter {
    count: i32,
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("rwire Server - Counter Example");
    println!("Open http://127.0.0.1:9000 in your browser");
    println!();

    // Use Nord palette with dark theme
    let capsule_config = CapsuleConfig::new()
        .theme(Theme::dark())
        .palette(ColorPalette::nord());

    Server::bind("127.0.0.1:9000")?
        .root(build_counter)
        .capsule_config(capsule_config)
        .run()
        .await
}

fn build_counter() -> ElementBuilder {
    // Full-page container using typed semantic tokens
    // BgApp maps to neutral-12 in dark mode (Nord Polar Night)
    el(El::Div)
        .st([St::BgApp, St::MinHScreen, St::FlexCenter])
        .append([
            el(El::Div)
                .class("counter")
                .st([St::DisplayFlex, St::ItemsCenter, St::GapLg])
                .append([
                    styled_button("-", decrement()),
                    render_count(),
                    styled_button("+", increment()),
                ]),
        ])
}

fn styled_button(text: &str, handler: rwire::HandlerSpec) -> ElementBuilder {
    // Typed semantic tokens:
    // BgAccent = accent-9 (Nord Frost primary)
    // TextHigh = high contrast text
    el(El::Button)
        .text(text)
        .st([
            St::BgAccent,
            St::TextHigh,
            St::BorderNone,
            St::PxMd,
            St::PySm,
            St::TextXl,
            St::RoundedMd,
            St::CursorPointer,
            St::TransitionColors,
        ])
        .on(Ev::Click, handler)
}

#[renderer]
fn render_count(state: &Counter) -> ElementBuilder {
    // Typed semantic tokens:
    // TextDefault = default text color (light in dark mode)
    el(El::Span)
        .text(&state.count.to_string())
        .st([St::TextDefault, St::Text2xl, St::FontBold, St::TextCenter])
}

#[handler]
fn increment(state: &mut Counter) {
    state.count += 1;
}

#[handler]
fn decrement(state: &mut Counter) {
    state.count -= 1;
}
