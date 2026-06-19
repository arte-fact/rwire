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
    el, outlet, renderer, theme, CurrentRoute, El, ElementBuilder, Link, Router, Server, St,
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
        link("/item/42", "Item 42"),
    ])
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
