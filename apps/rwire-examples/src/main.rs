//! rwire Examples
//!
//! Gallery website showcasing all rwire example projects.

use rwire::capsule_gen::{CapsuleConfig, FontFace};
use rwire::style_tokens::St;
use rwire::theme::{Theme, ThemeMode, ThemeStyle};
use rwire::{el, handler, renderer, theme, El, ElementBuilder, Server, State};
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
    website_url: String,
    docs_url: String,
    design_system_url: String,
}

impl Config {
    fn from_env() -> Self {
        Self {
            bind_addr: env_or("BIND_ADDR", "0.0.0.0:9003"),
            website_url: env_or("WEBSITE_URL", "http://127.0.0.1:9000"),
            docs_url: env_or("DOCS_URL", "http://127.0.0.1:9001"),
            design_system_url: env_or("DESIGN_SYSTEM_URL", "http://127.0.0.1:9002"),
        }
    }
}

static mut WEBSITE_URL: &str = "";
static mut DOCS_URL: &str = "";
static mut DESIGN_SYSTEM_URL: &str = "";

fn website_url() -> &'static str {
    unsafe { WEBSITE_URL }
}

fn docs_url() -> &'static str {
    unsafe { DOCS_URL }
}

fn design_system_url() -> &'static str {
    unsafe { DESIGN_SYSTEM_URL }
}

// ============================================================================
// Example Data
// ============================================================================

const COUNTER_CODE: &str = include_str!("../../../examples/counter/src/main.rs");
const TODOLIST_CODE: &str = include_str!("../../../examples/todolist/src/main.rs");
const FINE_GRAINED_CODE: &str = include_str!("../../../examples/fine-grained/src/main.rs");
const TODO_COMBINED_CODE: &str = include_str!("../../../examples/todo-combined/src/main.rs");

#[derive(Clone)]
struct Example {
    title: &'static str,
    description: &'static str,
    complexity: Complexity,
    tags: &'static [&'static str],
    run_command: &'static str,
    code: Option<&'static str>,
    external_url: Option<fn() -> &'static str>,
}

#[derive(Clone, Copy, PartialEq)]
enum Complexity {
    Beginner,
    Intermediate,
    Advanced,
}

impl Complexity {
    fn label(self) -> &'static str {
        match self {
            Self::Beginner => "Beginner",
            Self::Intermediate => "Intermediate",
            Self::Advanced => "Advanced",
        }
    }

    fn filter_index(self) -> usize {
        match self {
            Self::Beginner => 1,
            Self::Intermediate => 2,
            Self::Advanced => 3,
        }
    }
}

fn examples() -> Vec<Example> {
    vec![
        Example {
            title: "Counter",
            description: "A minimal counter app demonstrating state, handlers, and renderers. The simplest possible rwire application.",
            complexity: Complexity::Beginner,
            tags: &["State", "Handlers", "Renderers"],
            run_command: "cargo run -p counter",
            code: Some(COUNTER_CODE),
            external_url: None,
        },
        Example {
            title: "Todo List",
            description: "A todo list with add, toggle, and filter functionality. Shows reactive state with collections and form handling.",
            complexity: Complexity::Beginner,
            tags: &["Collections", "Forms", "Filtering"],
            run_command: "cargo run -p todolist",
            code: Some(TODOLIST_CODE),
            external_url: None,
        },
        Example {
            title: "Fine-Grained Reactivity",
            description: "Demonstrates TypeId-based filtering for selective re-renders. Only the renderers that depend on changed state are re-invoked.",
            complexity: Complexity::Intermediate,
            tags: &["TypeId", "Selective Re-render", "Performance"],
            run_command: "cargo run -p fine-grained",
            code: Some(FINE_GRAINED_CODE),
            external_url: None,
        },
        Example {
            title: "Multi-State TodoMVC",
            description: "A full TodoMVC implementation using memory and persisted storage types with ItemRef for type-safe dynamic content.",
            complexity: Complexity::Advanced,
            tags: &["Local Storage", "Persistence", "ItemRef"],
            run_command: "cargo run -p todo-combined",
            code: Some(TODO_COMBINED_CODE),
            external_url: None,
        },
        Example {
            title: "Design System",
            description: "Interactive showcase of 50 rwire components across forms, data display, and navigation categories.",
            complexity: Complexity::Intermediate,
            tags: &["Components", "Interactive", "Showcase"],
            run_command: "cargo run -p rwire-design-system",
            code: None,
            external_url: Some(design_system_url),
        },
    ]
}

// ============================================================================
// State
// ============================================================================

#[derive(State, Default)]
#[storage(memory)]
struct GalleryState {
    active_filter: usize, // 0=all, 1=beginner, 2=intermediate, 3=advanced
}

// ============================================================================
// Entry Point
// ============================================================================

#[theme]
fn site_theme() -> Theme {
    Theme::default().palette(palettes::nord())
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env();

    unsafe {
        WEBSITE_URL = Box::leak(config.website_url.into_boxed_str());
        DOCS_URL = Box::leak(config.docs_url.into_boxed_str());
        DESIGN_SYSTEM_URL = Box::leak(config.design_system_url.into_boxed_str());
    }

    println!("rwire Examples");
    println!("Open http://127.0.0.1:9003 in your browser");
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
        .append([
            build_header(),
            build_hero(),
            render_gallery(),
            build_footer(),
        ])
}

// ============================================================================
// Header
// ============================================================================

fn build_header() -> ElementBuilder {
    let left = el(El::A)
        .attr("href", website_url())
        .attr(
            "style",
            "font-family:'Quicksand',sans-serif;font-weight:300;letter-spacing:0.02em",
        )
        .st([
            St::TextLg,
            St::CursorPointer,
            St::TextDefault,
            St::NoDecoration,
        ])
        .text("rwire");

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
// Hero
// ============================================================================

fn build_hero() -> ElementBuilder {
    el(El::Div)
        .st([St::TextCenter, St::MxAuto, St::MaxW48rem, St::PxSm])
        .st([St::PtXl, St::PbLg])
        .append([
            el(El::H1)
                .attr(
                    "style",
                    "font-family:'Quicksand',sans-serif;font-weight:300;letter-spacing:0.02em",
                )
                .st([St::Text3xl, St::LeadingTight, St::TextHigh, St::MbSm])
                .md([St::Text4xl])
                .text("rwire Examples"),
            el(El::P)
                .st([St::TextBase, St::TextMuted, St::LeadingRelaxed])
                .md([St::TextLg])
                .text("Learn by example. Each project demonstrates a different rwire capability."),
        ])
}

// ============================================================================
// Gallery
// ============================================================================

#[renderer]
fn render_gallery(state: &GalleryState) -> ElementBuilder {
    let all_examples = examples();
    let filtered: Vec<&Example> = all_examples
        .iter()
        .filter(|e| {
            state.active_filter == 0 || e.complexity.filter_index() == state.active_filter
        })
        .collect();

    let f = state.active_filter;

    el(El::Div)
        .st([St::MxAuto, St::MaxW56rem, St::PxMd, St::PbXl])
        .append([
            // Filter bar
            Stack::row()
                .gap(Gap::Sm)
                .justify(StackJustify::Center)
                .children([
                    if f == 0 { Button::primary("All") } else { Button::ghost("All") }
                        .size(ButtonSize::Sm).on_click(set_filter_all()),
                    if f == 1 { Button::primary("Beginner") } else { Button::ghost("Beginner") }
                        .size(ButtonSize::Sm).on_click(set_filter_beginner()),
                    if f == 2 { Button::primary("Intermediate") } else { Button::ghost("Intermediate") }
                        .size(ButtonSize::Sm).on_click(set_filter_intermediate()),
                    if f == 3 { Button::primary("Advanced") } else { Button::ghost("Advanced") }
                        .size(ButtonSize::Sm).on_click(set_filter_advanced()),
                ])
                .build(),
            // Grid
            el(El::Div)
                .st([St::DisplayGrid, St::GridCols1, St::GapLg, St::MtLg])
                .md([St::GridCols2])
                .append(filtered.iter().map(|e| build_example_card(e)).collect::<Vec<_>>()),
        ])
}

fn build_example_card(example: &Example) -> ElementBuilder {
    let badge_intent = match example.complexity {
        Complexity::Beginner => BadgeIntent::Success,
        Complexity::Intermediate => BadgeIntent::Primary,
        Complexity::Advanced => BadgeIntent::Warning,
    };

    let mut content = Stack::column().gap(Gap::Sm).children([
        // Complexity badge + title
        Stack::row()
            .gap(Gap::Sm)
            .align(StackAlign::Center)
            .children([
                Badge::new()
                    .intent(badge_intent)
                    .text(example.complexity.label())
                    .build(),
                el(El::H3)
                    .st([St::FontSemibold, St::TextLg])
                    .text(example.title),
            ])
            .build(),
        // Description
        el(El::P)
            .st([St::TextSm, St::TextMuted])
            .text(example.description),
        // Tags
        Stack::row()
            .gap(Gap::Xs)
            .children(
                example
                    .tags
                    .iter()
                    .map(|tag| Badge::default_badge(tag.to_string()).build())
                    .collect::<Vec<_>>(),
            )
            .build(),
    ]);

    // Code preview (first ~20 lines)
    if let Some(code) = example.code {
        let preview: String = code.lines().take(20).collect::<Vec<_>>().join("\n");
        content = content.children([
            el(El::Div)
                .st([St::OverflowXAuto, St::RoundedMd, St::MtSm])
                .append([Code::block(preview).language("rust").build()]),
        ]);
    }

    // Run command
    content = content.children([
        el(El::Div)
            .st([
                St::DisplayFlex,
                St::ItemsCenter,
                St::GapSm,
                St::BgSubtle,
                St::RoundedMd,
                St::PxSm,
                St::PySm,
                St::MtSm,
            ])
            .append([
                el(El::Span).st([St::TextMuted, St::TextXs]).text("$"),
                Code::inline(example.run_command).build(),
                CopyButton::new(example.run_command).build(),
            ]),
    ]);

    // External link if available
    if let Some(url_fn) = example.external_url {
        content = content.children([
            el(El::A)
                .attr("href", url_fn())
                .st([St::TextSm, St::TextAccent, St::NoDecoration, St::CursorPointer])
                .hover([St::TextDefault])
                .text("View live \u{2192}"),
        ]);
    }

    Card::new()
        .shadow(CardShadow::Sm)
        .child(content.build())
        .build()
}

// ============================================================================
// Footer
// ============================================================================

fn build_footer() -> ElementBuilder {
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
            FooterColumn::new("Ecosystem")
                .external_link("Website", website_url())
                .external_link("Docs", docs_url())
                .external_link("Components", design_system_url()),
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
fn set_filter_all(state: &mut GalleryState) {
    state.active_filter = 0;
}

#[handler]
fn set_filter_beginner(state: &mut GalleryState) {
    state.active_filter = 1;
}

#[handler]
fn set_filter_intermediate(state: &mut GalleryState) {
    state.active_filter = 2;
}

#[handler]
fn set_filter_advanced(state: &mut GalleryState) {
    state.active_filter = 3;
}

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
