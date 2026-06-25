//! rwire Website
//!
//! The official rwire marketing landing page.
//! Built with rwire itself, demonstrating the framework's capabilities.

use rwire::capsule_gen::{CapsuleConfig, FontFace};
use rwire::icons::{icon, Icon};
use rwire::style_tokens::St;
use rwire::theme::{Theme, ThemeMode, ThemeStyle};
use rwire::{el, handler, renderer, theme, El, ElementBuilder, Server};
use rwire_components::*;
use rwire_themes::{palettes, styles};

// ============================================================================
// Configuration
// ============================================================================

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

struct Config {
    bind_addr: String,
    docs_url: String,
    design_system_url: String,
    examples_url: String,
}

impl Config {
    fn from_env() -> Self {
        Self {
            bind_addr: env_or("BIND_ADDR", "0.0.0.0:9000"),
            docs_url: env_or("DOCS_URL", "http://0.0.0.0:9001"),
            design_system_url: env_or("DESIGN_SYSTEM_URL", "http://0.0.0.0:9002"),
            examples_url: env_or("EXAMPLES_URL", "http://0.0.0.0:9003"),
        }
    }
}

// Store cross-site URLs as static strings (set once at startup)
static mut DOCS_URL: &str = "";
static mut DESIGN_SYSTEM_URL: &str = "";
static mut EXAMPLES_URL: &str = "";

fn docs_url() -> &'static str {
    unsafe { DOCS_URL }
}

fn design_system_url() -> &'static str {
    unsafe { DESIGN_SYSTEM_URL }
}

fn examples_url() -> &'static str {
    unsafe { EXAMPLES_URL }
}

// ============================================================================
// Entry Point
// ============================================================================

#[theme]
fn site_theme() -> Theme {
    Theme::default().accent("#5e81ac")
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env();

    // Leak strings so they live for 'static — only called once at startup
    unsafe {
        DOCS_URL = Box::leak(config.docs_url.into_boxed_str());
        DESIGN_SYSTEM_URL = Box::leak(config.design_system_url.into_boxed_str());
        EXAMPLES_URL = Box::leak(config.examples_url.into_boxed_str());
    }

    println!("rwire Website");
    println!("Open http://127.0.0.1:9000 in your browser");
    println!();

    let capsule_config =
        CapsuleConfig::new().font(FontFace::google("Quicksand", &[300, 400, 600, 700]));

    Server::bind(&config.bind_addr)?
        .root(root)
        .capsule_config(capsule_config)
        .theme(site_theme())
        .run()
        .await
}

// ============================================================================
// Root Layout
// ============================================================================

fn root() -> ElementBuilder {
    el(El::Div)
        .st([St::BgApp, St::TextDefault, St::MinHScreen])
        .append([build_header(), build_landing_page(), build_footer()])
}

// ============================================================================
// Header
// ============================================================================

fn build_header() -> ElementBuilder {
    // Left side: logo
    let left = el(El::Span)
        .attr(
            "style",
            "font-family:'Quicksand',sans-serif;font-weight:300;letter-spacing:0.02em",
        )
        .st([St::TextLg, St::TextDefault])
        .text("rwire");

    // Right side: nav links + theme controls
    let right = Stack::row()
        .gap(Gap::Sm)
        .align(StackAlign::Center)
        .children([
            el(El::A)
                .attr("href", docs_url())
                .st([
                    St::TextSm,
                    St::TextMuted,
                    St::NoDecoration,
                    St::CursorPointer,
                ])
                .hover([St::TextDefault])
                .text("Docs"),
            el(El::A)
                .attr("href", design_system_url())
                .st([
                    St::TextSm,
                    St::TextMuted,
                    St::NoDecoration,
                    St::CursorPointer,
                ])
                .hover([St::TextDefault])
                .text("Components"),
            el(El::A)
                .attr("href", examples_url())
                .st([
                    St::TextSm,
                    St::TextMuted,
                    St::NoDecoration,
                    St::CursorPointer,
                ])
                .hover([St::TextDefault])
                .text("Examples"),
            el(El::Div)
                .st([St::DisplayNone])
                .md([St::DisplayFlex])
                .append([render_style_switcher()]),
            render_theme_toggle(),
        ])
        .build();

    el(El::Header)
        .st([
            St::PositionSticky,
            St::Top0,
            St::Z50,
            St::BgApp,
            St::BorderBDefault,
            St::DisplayFlex,
            St::ItemsCenter,
            St::JustifyBetween,
            St::PxMd,
        ])
        .st([St::HHeader])
        .append([left, right])
}

#[renderer]
fn render_theme_toggle(theme: &Theme) -> ElementBuilder {
    ThemeToggle::new()
        .mode(match theme.mode {
            ThemeMode::Light => ThemeToggleMode::Light,
            ThemeMode::Dark => ThemeToggleMode::Dark,
        })
        .on_toggle(toggle_theme())
        .build()
}

#[renderer]
fn render_style_switcher(theme: &Theme) -> ElementBuilder {
    Stack::row()
        .gap(Gap::Xs)
        .align(StackAlign::Center)
        .children([
            Button::ghost(theme.style.label())
                .size(ButtonSize::Sm)
                .on_click(cycle_theme_style()),
            Button::ghost(if theme.palette_ref().is_some() {
                "Nord"
            } else {
                "Default"
            })
            .size(ButtonSize::Sm)
            .on_click(cycle_palette()),
        ])
        .build()
}

// ============================================================================
// Landing Page
// ============================================================================

fn build_landing_page() -> ElementBuilder {
    el(El::Div).append([
        section_hero(),
        section_stats(),
        section_code_example(),
        section_features(),
        section_comparison(),
        section_cta(),
    ])
}

// -- Section 1: Hero ----------------------------------------------------------

fn section_hero() -> ElementBuilder {
    let get_started_url = format!(
        "{}/docs/01-getting-started/install",
        docs_url().trim_end_matches('/')
    );

    el(El::Div)
        .st([St::TextCenter, St::MxAuto, St::MaxW48rem, St::PxSm])
        .st([St::Pt3xl, St::PbXl])
        .append([
            // Title
            el(El::H1)
                .attr(
                    "style",
                    "font-family:'Quicksand',sans-serif;font-weight:300;letter-spacing:0.02em",
                )
                .st([St::Text3xl, St::LeadingTight, St::TextHigh, St::MbMd])
                .md([St::Text5xl])
                .text("rwire"),
            // Subtitle
            el(El::P)
                .st([St::TextBase, St::TextMuted, St::LeadingRelaxed, St::MbLg])
                .md([St::TextXl])
                .text("Server-side UI with a binary protocol."),
            // Install command
            el(El::Div)
                .st([
                    St::DisplayInlineFlex,
                    St::ItemsCenter,
                    St::GapSm,
                    St::BgSubtle,
                    St::RoundedMd,
                    St::PxMd,
                    St::PySm,
                    St::MbLg,
                ])
                .append([
                    el(El::Span).st([St::TextMuted]).text("$"),
                    Code::inline("cargo add rwire").build(),
                    CopyButton::new("cargo add rwire").build(),
                ]),
            // CTAs
            Stack::row()
                .gap(Gap::Md)
                .justify(StackJustify::Center)
                .children([
                    el(El::A)
                        .attr("href", &get_started_url)
                        .st([St::NoDecoration])
                        .append([Button::primary("Get Started \u{2192}").build()]),
                ])
                .build(),
        ])
}

// -- Section 2: Stats ---------------------------------------------------------

fn section_stats() -> ElementBuilder {
    let stats = [
        ("~17KB", "Client payload (minimal app)"),
        ("~30 bytes", "Per UI update"),
        ("50", "Production-ready components"),
        ("200K+", "Connections per GB RAM"),
    ];

    el(El::Div)
        .st([St::MxAuto, St::MaxW56rem, St::PxMd])
        .st([St::PtXl, St::Pb3xl])
        .append([Card::new()
            .shadow(CardShadow::None)
            .child(
                el(El::Div)
                    .st([
                        St::DisplayGrid,
                        St::GridCols2,
                        St::GapLg,
                        St::TextCenter,
                    ])
                    .md([St::GridCols4])
                    .append(
                        stats
                            .iter()
                            .map(|(value, label)| {
                                el(El::Div).append([
                                    el(El::Div)
                                        .st([St::Text3xl, St::FontBold, St::TextAccent])
                                        .text(value),
                                    el(El::Div)
                                        .st([St::TextSm, St::TextMuted])
                                        .text(label),
                                ])
                            })
                            .collect::<Vec<_>>(),
                    ),
            )
            .build()])
}

// -- Section 3: Code Example --------------------------------------------------

fn section_code_example() -> ElementBuilder {
    let code = r#"use rwire_components::*;
use rwire::{el, handler, renderer, El, Server, State};

#[derive(State, Default)]
#[storage(memory)]
struct Counter { count: i32 }

#[handler]
fn increment(state: &mut Counter) {
    state.count += 1;
}

#[renderer]
fn render_count(state: &Counter) -> ElementBuilder {
    Text::heading1(state.count.to_string()).build()
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Server::bind("0.0.0.0:9000")?
        .root(root)
        .run()
        .await
}"#;

    let annotations = [
        (
            "#[derive(State)]",
            "State is a Rust struct. Typed, owned, serializable.",
        ),
        ("#[handler]", "Handlers are plain functions. No ceremony."),
        (
            "#[renderer]",
            "Renderers re-run automatically when state changes.",
        ),
    ];

    el(El::Div)
        .st([St::BgSubtle, St::Py3xl, St::PxLg])
        .append([el(El::Div).st([St::MxAuto, St::MaxW56rem]).append([
            // Header
            el(El::H2)
                .st([St::Text2xl, St::FontSemibold, St::TextCenter, St::MbSm])
                .text("A complete app in 20 lines"),
            el(El::P)
                .st([St::TextMuted, St::TextCenter, St::MbLg])
                .text("Define state, write handlers, attach renderers. The macros handle the rest."),
            // Two-column layout: code + annotations
            el(El::Div)
                .st([St::DisplayFlex, St::FlexCol, St::GapLg])
                .md([St::FlexRow])
                .append([
                    // Code block (left, wider)
                    el(El::Div)
                        .st([St::Flex1, St::MinW0, St::OverflowXAuto])
                        .append([Code::block(code).language("rust").build()]),
                    // Annotations (right, narrower)
                    el(El::Div)
                        .st([
                            St::MaxW280px,
                            St::DisplayFlex,
                            St::FlexCol,
                            St::GapMd,
                            St::JustifyCenter,
                        ])
                        .append(
                            annotations
                                .iter()
                                .map(|(macro_name, desc)| {
                                    el(El::Div).st([St::PySm]).append([
                                        Code::inline(macro_name.to_string()).build(),
                                        el(El::P)
                                            .st([St::TextSm, St::TextMuted, St::MtXs])
                                            .text(desc),
                                    ])
                                })
                                .collect::<Vec<_>>(),
                        ),
                ]),
        ])])
}

// -- Section 4: Feature Grid --------------------------------------------------

fn section_features() -> ElementBuilder {
    let features: [(Icon, &str, &str); 6] = [
        (
            Icon::Zap,
            "Binary Protocol",
            "A UI update is tens of bytes of binary opcodes \u{2014} not JSON, not HTML fragments \u{2014} parsed in microseconds.",
        ),
        (
            Icon::Feather,
            "~17KB Client",
            "The whole client \u{2014} runtime plus styles \u{2014} in one small document. Element/event maps and CSS stream lazily over the wire, so you only receive what your app uses.",
        ),
        (
            Icon::Shield,
            "Fully Typed",
            "State, handlers, renderers, components \u{2014} all Rust. Catch errors at compile time.",
        ),
        (
            Icon::Cpu,
            "Tiny Connections",
            "Sub-KB per connection: a minimal server held 5,000 live WebSocket connections in under 4 MB of RAM. No GC pauses, no JVM warmup.",
        ),
        (
            Icon::Palette,
            "700+ Style Tokens",
            "CSS encoded as 1\u{2013}2 byte varint codes. Semantic theming with light/dark mode.",
        ),
        (
            Icon::Leaf,
            "Low Carbon",
            "Tiny binary diffs instead of re-rendered HTML or JSON. Fewer bytes = less energy across the stack.",
        ),
    ];

    el(El::Div).st([St::Py3xl, St::PxLg]).append([el(El::Div)
        .st([St::MxAuto, St::MaxW56rem])
        .append([
            el(El::H2)
                .st([St::Text2xl, St::FontSemibold, St::TextCenter, St::MbLg])
                .text("Why rwire"),
            el(El::Div)
                .st([St::DisplayGrid, St::GridCols1, St::GapLg])
                .md([St::GridCols2])
                .lg([St::GridCols3])
                .append(
                    features
                        .iter()
                        .map(|(ico, title, desc)| {
                            Card::new()
                                .shadow(CardShadow::None)
                                .child(
                                    Stack::column()
                                        .gap(Gap::Sm)
                                        .children([
                                            el(El::Div)
                                                .st([St::TextAccent])
                                                .append([icon(*ico)]),
                                            el(El::Div).st([St::FontSemibold]).text(title),
                                            el(El::P)
                                                .st([St::TextSm, St::TextMuted])
                                                .text(desc),
                                        ])
                                        .build(),
                                )
                                .build()
                        })
                        .collect::<Vec<_>>(),
                ),
        ])])
}

// -- Section 5: Comparison Table ----------------------------------------------

fn section_comparison() -> ElementBuilder {
    el(El::Div)
        .st([St::BgSubtle, St::Py3xl, St::PxLg])
        .append([el(El::Div).st([St::MxAuto, St::MaxW56rem]).append([
            el(El::H2)
                .st([St::Text2xl, St::FontSemibold, St::TextCenter, St::MbSm])
                .text("How rwire compares"),
            el(El::P)
                .st([St::TextMuted, St::TextCenter, St::MbLg])
                .text("Architecture and ballpark sizes; competitor figures are approximate."),
            el(El::Div).st([St::OverflowXAuto]).append([Table::new()
                .headers(["", "rwire", "LiveView", "Blazor", "htmx"])
                .row(
                    TableRow::new()
                        .cells(["Client runtime", "~17KB", "30KB", "200KB", "14KB"]),
                )
                .row(
                    TableRow::new()
                        .cells(["Wire format", "Binary", "JSON", "JSON", "HTML"]),
                )
                .row(
                    TableRow::new().cells([
                        "Update cost",
                        "~30 bytes",
                        "25 bytes",
                        "100+ bytes",
                        "100+ bytes",
                    ]),
                )
                .row(
                    TableRow::new().cells([
                        "Memory/conn",
                        "2\u{2013}5KB",
                        "5\u{2013}50KB",
                        "250KB",
                        "N/A",
                    ]),
                )
                .row(
                    TableRow::new()
                        .cells(["Language", "Rust", "Elixir", "C#", "Any"]),
                )
                .striped(true)
                .build()]),
        ])])
}

// -- Section 6: CTA -----------------------------------------------------------

fn section_cta() -> ElementBuilder {
    let docs_base = docs_url().trim_end_matches('/');
    let read_docs_url = format!("{docs_base}/docs/01-getting-started/install");
    let quick_start_url = format!("{docs_base}/docs/01-getting-started/quick-start");

    el(El::Div)
        .st([St::BgAccentSubtle, St::TextCenter, St::Py4xl, St::PxLg])
        .append([
            el(El::H2)
                .st([St::Text3xl, St::FontBold, St::MbLg])
                .text("Ready to build?"),
            // Install command
            el(El::Div)
                .st([
                    St::DisplayInlineFlex,
                    St::ItemsCenter,
                    St::GapSm,
                    St::BgApp,
                    St::RoundedMd,
                    St::PxMd,
                    St::PySm,
                    St::MbLg,
                ])
                .append([
                    el(El::Span).st([St::TextMuted]).text("$"),
                    Code::inline("cargo add rwire").build(),
                    CopyButton::new("cargo add rwire").build(),
                ]),
            // CTAs
            Stack::row()
                .gap(Gap::Md)
                .justify(StackJustify::Center)
                .children([
                    el(El::A)
                        .attr("href", &read_docs_url)
                        .st([St::NoDecoration])
                        .append([Button::primary("Read the docs \u{2192}").build()]),
                    el(El::A)
                        .attr("href", &quick_start_url)
                        .st([St::NoDecoration])
                        .append([Button::ghost("Browse examples \u{2192}").build()]),
                ])
                .build(),
        ])
}

// ============================================================================
// Footer
// ============================================================================

fn build_footer() -> ElementBuilder {
    let docs_base = docs_url().trim_end_matches('/');

    Footer::new()
        .logo(
            el(El::Span)
                .attr(
                    "style",
                    "font-family:'Quicksand',sans-serif;font-weight:300;letter-spacing:0.02em",
                )
                .st([St::TextLg, St::TextDefault])
                .text("rwire"),
        )
        .tagline("Server-side UI framework with a binary protocol.")
        .column(
            FooterColumn::new("Documentation")
                .external_link(
                    "Getting Started",
                    &format!("{docs_base}/docs/01-getting-started/install"),
                )
                .external_link(
                    "Core Concepts",
                    &format!("{docs_base}/docs/02-core-concepts/state"),
                )
                .external_link(
                    "Components",
                    &format!("{docs_base}/docs/03-components/overview"),
                ),
        )
        .column(
            FooterColumn::new("Ecosystem")
                .external_link("Docs", docs_url())
                .external_link("Components", design_system_url())
                .external_link("Examples", examples_url()),
        )
        .column(
            FooterColumn::new("Community")
                .external_link("GitHub", "https://github.com/arte-fact/rwire"),
        )
        .copyright("\u{00a9} 2026 rwire contributors. MIT License.")
        .build()
}

// ============================================================================
// Handlers
// ============================================================================

#[handler]
fn toggle_theme(theme: &mut Theme) {
    theme.mode = theme.mode.toggle();
}

#[handler]
fn cycle_theme_style(theme: &mut Theme) {
    let mut all = vec![ThemeStyle::soft()];
    all.extend(styles::ALL.iter().map(|f| f()));
    let idx = all.iter().position(|s| *s == theme.style).unwrap_or(0);
    theme.style = all[(idx + 1) % all.len()];
}

#[handler]
fn cycle_palette(theme: &mut Theme) {
    if theme.palette_ref().is_some() {
        theme.clear_palette();
    } else {
        theme.set_palette(palettes::nord());
    }
}
