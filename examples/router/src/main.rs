//! Router example: a persistent shell (nav) + an `outlet()` that renders the matched
//! route view. Proves the hybrid-routing contract:
//! - `Link` navigates with no page reload, updating the URL,
//! - deep-link / reload at a path renders the right view,
//! - back/forward work,
//! - the active-nav highlight (a `#[renderer]` over `CurrentRoute`) tracks the URL.
//!
//! Increment A: static views. (Increment B adds a stateful `/counter` view.)

use std::error::Error;

use async_std::main;
use rwire::capsule_gen::CapsuleConfig;
use rwire::theme::Theme;
use rwire::{
    el, handler, outlet, renderer, theme, CurrentRoute, El, ElementBuilder, Link, Router, Server,
    St, State,
};
use rwire_components::Text;
use rwire_themes::palettes;

#[theme]
fn app_theme() -> Theme {
    Theme::dark().palette(palettes::nord())
}

#[main]
async fn main() -> Result<(), Box<dyn Error>> {
    Server::bind("0.0.0.0:7779")?
        .root(shell)
        .routes(
            Router::new()
                .page("/", |_| home())
                .page("/about", |_| about())
                .page("/counter", |_| counter_view())
                .page("/item/:id", |params| item(params.get("id").unwrap_or("?"))),
        )
        .capsule_config(CapsuleConfig::new())
        .theme(app_theme())
        .run()
        .await
}

/// Persistent layout: the nav stays, the `outlet()` swaps per route.
fn shell() -> ElementBuilder {
    el(El::Div)
        .st([
            St::BgApp,
            St::MinHScreen,
            St::TextDefault,
            St::FontMono,
            St::PXl,
            St::DisplayFlex,
            St::FlexCol,
            St::GapLg,
        ])
        .append([nav(), outlet()])
}

/// Active-nav highlight: a renderer over the built-in `CurrentRoute`.
#[renderer]
fn nav(route: &CurrentRoute) -> ElementBuilder {
    let link = |href: &'static str, label: &str| {
        let color = if route.path() == href {
            St::TextAccent
        } else {
            St::TextMuted
        };
        Link::to_with_content(href, el(El::Span).st([color]).text(label))
            .st([St::NoUnderline, St::PrMd])
    };
    el(El::Div).st([St::DisplayFlex, St::GapMd]).append([
        link("/", "Home"),
        link("/about", "About"),
        link("/counter", "Counter"),
        link("/item/42", "Item 42"),
    ])
}

/// A STATEFUL routed view: its own `#[renderer]` + state + handler. After navigating
/// here, the count must update on click — and still work after navigating away and
/// back. That requires the framework to register the view's renderer (increment B).
#[derive(State, Default)]
#[storage(memory)]
struct Counter {
    n: i32,
}

fn counter_view() -> ElementBuilder {
    el(El::Div)
        .st([St::DisplayFlex, St::FlexCol, St::GapSm])
        .append([
            Text::heading1("Counter".to_owned()).build(),
            count_display(),
            rwire_components::Button::primary("+").on_click(bump()),
        ])
}

#[renderer]
fn count_display(state: &Counter) -> ElementBuilder {
    Text::body(format!("count: {}", state.n)).build()
}

#[handler]
fn bump(state: &mut Counter) {
    state.n += 1;
}

fn page(title: &str, body: &str) -> ElementBuilder {
    el(El::Div)
        .st([St::DisplayFlex, St::FlexCol, St::GapSm])
        .append([
            Text::heading1(title.to_owned()).build(),
            Text::body(body.to_owned()).muted().build(),
        ])
}

fn home() -> ElementBuilder {
    page(
        "Home",
        "The home view. Click a link — no reload; the URL updates. Reload on any path works.",
    )
}

fn about() -> ElementBuilder {
    page(
        "About",
        "A second routed view, rendered fresh by the outlet on navigation.",
    )
}

fn item(id: &str) -> ElementBuilder {
    page(&format!("Item {id}"), "A parameterized route (/item/:id).")
}
