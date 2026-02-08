//! rwire Website
//!
//! The official rwire website — landing page + documentation hub.
//! Built with rwire itself, demonstrating the framework's capabilities.

use rwire::capsule_gen::{CapsuleConfig, FontFace};
use rwire::components::*;
use rwire::docs::{parse_markdown, DocPage, DocSite, SearchResult};
use rwire::icons::{icon, Icon};
use rwire::style_tokens::St;
use rwire::theme::{Theme, ThemeMode, ThemeStyle};
use rwire::tokens::ColorPalette;
use rwire::router::{Link, Router};
use rwire::{el, handler, renderer, El, ElementBuilder, Server, State};

// ============================================================================
// State
// ============================================================================

#[derive(State, Default)]
#[storage(memory)]
struct SiteState {
    current_path: String,
    search_query: String,
    searching: bool,
    theme_mode: ThemeMode,
    theme_style: ThemeStyle,
}

// ============================================================================
// Entry Point
// ============================================================================

static DOCS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/docs");

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("rwire Website");
    println!("Open http://127.0.0.1:9000 in your browser");
    println!();

    let router = Router::new()
        .page("/", |_| build_landing_page())
        .page("/docs/*", |_| {
            let site = DocSite::load(DOCS_DIR);
            let path = site.sections().first()
                .and_then(|(_, paths)| paths.first())
                .cloned()
                .unwrap_or_default();
            let sidebar = build_sidebar(&site, &path);
            el(El::Div).append([sidebar, build_doc_page(&site, &path)])
        })
        .page("/search", |_| {
            build_search_results(&DocSite::load(DOCS_DIR), "example")
        });

    let capsule_config = CapsuleConfig::new()
        .theme(Theme::default())
        .palette(ColorPalette::nord())
        .font(FontFace::google("Quicksand", &[300, 400, 600, 700]));

    Server::bind("0.0.0.0:9000")?
        .root(root)
        .on_route(on_route_change())
        .routes(router)
        .capsule_config(capsule_config)
        .run()
        .await
}

// ============================================================================
// Root Layout
// ============================================================================

#[renderer]
fn root(state: &SiteState) -> ElementBuilder {
    let site = DocSite::load(DOCS_DIR);
    let is_landing = state.current_path.is_empty();

    let main_content = if state.searching {
        build_search_results(&site, &state.search_query)
    } else if is_landing {
        build_landing_page()
    } else {
        build_doc_page(&site, &state.current_path)
    };

    let mut root_el = el(El::Div)
        .attr("data-theme", state.theme_mode.as_str())
        .st([St::BgApp, St::TextDefault, St::MinHScreen]);
    if state.theme_style != ThemeStyle::Default {
        root_el = root_el.attr("data-style", state.theme_style.as_str());
    }

    if is_landing {
        // Landing page: no sidebar, full-width layout
        root_el.append([
            build_header(),
            main_content,
            build_footer(),
        ])
    } else {
        // Docs pages: AppShell with sidebar
        let sidebar = build_sidebar(&site, &state.current_path);
        root_el.append([
            AppShell::new()
                .header(build_header())
                .sidebar(sidebar)
                .main(main_content)
                .build(),
            build_footer(),
        ])
    }
}

// ============================================================================
// Header
// ============================================================================

fn build_header() -> ElementBuilder {
    el(El::Header)
        .st([
            St::PositionSticky, St::Top0, St::Z50, St::BgApp,
            St::BorderBDefault, St::DisplayFlex, St::ItemsCenter,
            St::JustifyBetween, St::PxMd,
        ])
        .attr("style", "height:56px")
        .append([
            Link::to("/", "rwire")
                .attr("style", "font-family:'Quicksand',sans-serif;font-weight:300;letter-spacing:0.02em")
                .st([St::TextLg, St::CursorPointer, St::TextDefault, St::NoDecoration]),
            Stack::row()
                .gap(Gap::Sm)
                .align(StackAlign::Center)
                .children([
                    render_search_input(),
                    Link::to("/docs/01-getting-started/install", "Docs")
                        .st([St::TextSm, St::TextMuted, St::NoDecoration, St::CursorPointer])
                        .hover([St::TextDefault]),
                    render_style_switcher(),
                    render_theme_toggle(),
                ])
                .build(),
        ])
}

#[renderer]
fn render_search_input(state: &SiteState) -> ElementBuilder {
    Input::search()
        .placeholder("Search docs...")
        .size(InputSize::Sm)
        .value(state.search_query.clone())
        .on_input(on_search_input())
}

#[renderer]
fn render_theme_toggle(state: &SiteState) -> ElementBuilder {
    ThemeToggle::new()
        .mode(match state.theme_mode {
            ThemeMode::Light => ThemeToggleMode::Light,
            ThemeMode::Dark => ThemeToggleMode::Dark,
        })
        .on_toggle(toggle_theme())
        .build()
}

#[renderer]
fn render_style_switcher(state: &SiteState) -> ElementBuilder {
    let label = match state.theme_style {
        ThemeStyle::Default => "Default",
        ThemeStyle::Soft => "Soft",
        ThemeStyle::Brutalist => "Brutalist",
        ThemeStyle::Minimal => "Minimal",
    };
    Button::ghost(label)
        .size(ButtonSize::Sm)
        .on_click(cycle_theme_style())
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
    el(El::Div)
        .st([St::TextCenter, St::MxAuto, St::MaxW48rem])
        .attr("style", "padding:5rem 1.5rem 4rem")
        .append([
            // Title
            el(El::H1)
                .attr("style", "font-family:'Quicksand',sans-serif;font-weight:300;letter-spacing:0.02em")
                .st([St::Text5xl, St::LeadingTight, St::TextHigh, St::MbMd])
                .text("rwire"),
            // Subtitle
            el(El::P)
                .st([St::TextXl, St::TextMuted, St::LeadingRelaxed, St::MbLg])
                .text("Server-side UI with a binary protocol."),
            // Install command
            el(El::Div)
                .st([
                    St::DisplayInlineFlex, St::ItemsCenter, St::GapSm,
                    St::BgSubtle, St::RoundedMd, St::PxMd, St::PySm, St::MbLg,
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
                    Link::to_with_content(
                        "/docs/01-getting-started/install",
                        Button::primary("Get Started \u{2192}").build(),
                    )
                    .st([St::NoDecoration]),
                    el(El::A)
                        .attr("href", "https://github.com")
                        .attr("target", "_blank")
                        .attr("rel", "noopener noreferrer")
                        .st([St::NoDecoration])
                        .append([
                            Stack::row()
                                .gap(Gap::Xs)
                                .align(StackAlign::Center)
                                .children([
                                    icon(Icon::GitHub),
                                    Button::ghost("GitHub").build(),
                                ])
                                .build(),
                        ]),
                ])
                .build(),
        ])
}

// -- Section 2: Stats ---------------------------------------------------------

fn section_stats() -> ElementBuilder {
    let stats = [
        ("~1.5KB", "JS runtime (tree-shaken)"),
        ("4 bytes", "Per text update"),
        ("51", "Production-ready components"),
        ("200K+", "Connections per GB RAM"),
    ];

    el(El::Div)
        .st([St::MxAuto, St::MaxW56rem, St::PxMd])
        .attr("style", "padding-top:2rem;padding-bottom:3rem")
        .append([
            Card::new()
                .shadow(CardShadow::None)
                .child(
                    el(El::Div)
                        .st([St::DisplayGrid, St::GridCols4, St::GapLg, St::TextCenter])
                        .append(
                            stats.iter().map(|(value, label)| {
                                el(El::Div).append([
                                    el(El::Div)
                                        .st([St::Text3xl, St::FontBold, St::TextAccent])
                                        .text(value),
                                    el(El::Div)
                                        .st([St::TextSm, St::TextMuted])
                                        .text(label),
                                ])
                            }).collect::<Vec<_>>(),
                        ),
                )
                .build(),
        ])
}

// -- Section 3: Code Example --------------------------------------------------

fn section_code_example() -> ElementBuilder {
    let code = r#"use rwire::components::*;
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

fn main() {
    Server::bind("0.0.0.0:9000").unwrap()
        .root(root)
        .run_blocking();
}"#;

    let annotations = [
        ("#[derive(State)]", "State is a Rust struct. Typed, owned, serializable."),
        ("#[handler]", "Handlers are plain functions. No ceremony."),
        ("#[renderer]", "Renderers re-run automatically when state changes."),
    ];

    el(El::Div)
        .st([St::BgSubtle])
        .attr("style", "padding:3rem 1.5rem")
        .append([
            el(El::Div)
                .st([St::MxAuto, St::MaxW56rem])
                .append([
                    // Header
                    el(El::H2)
                        .st([St::Text2xl, St::FontSemibold, St::TextCenter, St::MbSm])
                        .text("A complete app in 20 lines"),
                    el(El::P)
                        .st([St::TextMuted, St::TextCenter, St::MbLg])
                        .text("Define state, write handlers, attach renderers. The macros handle the rest."),
                    // Two-column layout: code + annotations
                    el(El::Div)
                        .st([St::DisplayFlex, St::FlexWrap, St::GapLg])
                        .append([
                            // Code block (left, wider)
                            el(El::Div)
                                .st([St::Flex1, St::MinW0])
                                .attr("style", "min-width:320px")
                                .append([
                                    Code::block(code)
                                        .language("rust")
                                        .build(),
                                ]),
                            // Annotations (right, narrower)
                            el(El::Div)
                                .attr("style", "min-width:200px;max-width:280px")
                                .st([St::DisplayFlex, St::FlexCol, St::GapMd, St::JustifyCenter])
                                .append(
                                    annotations.iter().map(|(macro_name, desc)| {
                                        el(El::Div)
                                            .st([St::PySm])
                                            .append([
                                                Code::inline(macro_name.to_string()).build(),
                                                el(El::P)
                                                    .st([St::TextSm, St::TextMuted, St::MtXs])
                                                    .text(desc),
                                            ])
                                    }).collect::<Vec<_>>(),
                                ),
                        ]),
                ]),
        ])
}

// -- Section 4: Feature Grid --------------------------------------------------

fn section_features() -> ElementBuilder {
    let features: [(Icon, &str, &str); 6] = [
        (
            Icon::Zap,
            "Binary Protocol",
            "DOM updates in 4 bytes. Not JSON, not HTML fragments \u{2014} binary opcodes parsed in microseconds.",
        ),
        (
            Icon::Feather,
            "1.5KB Runtime",
            "Tree-shaken JS runtime. 20x smaller than LiveView, 200x smaller than Vaadin.",
        ),
        (
            Icon::Shield,
            "Fully Typed",
            "State, handlers, renderers, components \u{2014} all Rust. Catch errors at compile time.",
        ),
        (
            Icon::Cpu,
            "200K Connections/GB",
            "Rust async tasks use ~2\u{2013}5KB per connection. No GC pauses, no JVM warmup.",
        ),
        (
            Icon::Palette,
            "580+ Style Tokens",
            "CSS encoded as 1\u{2013}2 byte varint codes. Semantic theming with light/dark mode.",
        ),
        (
            Icon::Leaf,
            "Low Carbon",
            "60x less bandwidth than a React SPA. Fewer bytes = less energy across the stack.",
        ),
    ];

    el(El::Div)
        .attr("style", "padding:3rem 1.5rem")
        .append([
            el(El::Div)
                .st([St::MxAuto, St::MaxW56rem])
                .append([
                    el(El::H2)
                        .st([St::Text2xl, St::FontSemibold, St::TextCenter, St::MbLg])
                        .text("Why rwire"),
                    Grid::new()
                        .columns(GridColumns::Fixed3)
                        .gap(Gap::Lg)
                        .children(
                            features.iter().map(|(ico, title, desc)| {
                                Card::new()
                                    .shadow(CardShadow::None)
                                    .child(
                                        Stack::column()
                                            .gap(Gap::Sm)
                                            .children([
                                                el(El::Div)
                                                    .st([St::TextAccent])
                                                    .append([icon(*ico)]),
                                                el(El::Div)
                                                    .st([St::FontSemibold])
                                                    .text(title),
                                                el(El::P)
                                                    .st([St::TextSm, St::TextMuted])
                                                    .text(desc),
                                            ])
                                            .build(),
                                    )
                                    .build()
                            }).collect::<Vec<_>>(),
                        )
                        .build(),
                ]),
        ])
}

// -- Section 5: Comparison Table ----------------------------------------------

fn section_comparison() -> ElementBuilder {
    el(El::Div)
        .st([St::BgSubtle])
        .attr("style", "padding:3rem 1.5rem")
        .append([
            el(El::Div)
                .st([St::MxAuto, St::MaxW56rem])
                .append([
                    el(El::H2)
                        .st([St::Text2xl, St::FontSemibold, St::TextCenter, St::MbSm])
                        .text("How rwire compares"),
                    el(El::P)
                        .st([St::TextMuted, St::TextCenter, St::MbLg])
                        .text("Real numbers, not marketing claims."),
                    Table::new()
                        .headers(["", "rwire", "LiveView", "Blazor", "htmx"])
                        .row(TableRow::new().cells(["Client runtime", "1.5KB", "30KB", "200KB", "14KB"]))
                        .row(TableRow::new().cells(["Wire format", "Binary", "JSON", "JSON", "HTML"]))
                        .row(TableRow::new().cells(["Update cost", "4 bytes", "25 bytes", "100+ bytes", "100+ bytes"]))
                        .row(TableRow::new().cells(["Memory/conn", "2\u{2013}5KB", "5\u{2013}50KB", "250KB", "N/A"]))
                        .row(TableRow::new().cells(["Language", "Rust", "Elixir", "C#", "Any"]))
                        .striped(true)
                        .build(),
                ]),
        ])
}

// -- Section 6: CTA -----------------------------------------------------------

fn section_cta() -> ElementBuilder {
    el(El::Div)
        .st([St::BgAccentSubtle, St::TextCenter])
        .attr("style", "padding:4rem 1.5rem")
        .append([
            el(El::H2)
                .st([St::Text3xl, St::FontBold, St::MbLg])
                .text("Ready to build?"),
            // Install command
            el(El::Div)
                .st([
                    St::DisplayInlineFlex, St::ItemsCenter, St::GapSm,
                    St::BgApp, St::RoundedMd, St::PxMd, St::PySm, St::MbLg,
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
                    Link::to_with_content(
                        "/docs/01-getting-started/install",
                        Button::primary("Read the docs \u{2192}").build(),
                    )
                    .st([St::NoDecoration]),
                    Link::to_with_content(
                        "/docs/01-getting-started/quick-start",
                        Button::ghost("Browse examples \u{2192}").build(),
                    )
                    .st([St::NoDecoration]),
                ])
                .build(),
        ])
}

// ============================================================================
// Sidebar
// ============================================================================

/// Strip numeric ordering prefix from section names (e.g., "01-getting-started" → "getting started").
fn strip_section_prefix(name: &str) -> String {
    let stripped = name
        .strip_prefix(|c: char| c.is_ascii_digit())
        .and_then(|s| s.strip_prefix(|c: char| c.is_ascii_digit()))
        .and_then(|s| s.strip_prefix('-'))
        .unwrap_or(name);
    stripped.replace('-', " ")
}

fn build_sidebar(site: &DocSite, active_path: &str) -> ElementBuilder {
    let mut nav = el(El::Nav)
        .st([St::DisplayFlex, St::FlexCol, St::GapMd, St::PxSm, St::TextSm]);

    for (section_name, page_paths) in site.sections() {
        let title = el(El::Div)
            .st([
                St::TextXsMuted,
                St::FontSemibold,
                St::TextUppercase,
                St::TrackingWider,
                St::PxSm,
                St::PySm,
            ])
            .text(&strip_section_prefix(section_name));

        let mut link_list = el(El::Div).st([St::DisplayFlex, St::FlexCol]);

        for page_path in page_paths {
            if let Some(page) = site.page(page_path) {
                let is_active = active_path == page_path;

                let tokens = if is_active {
                    vec![
                        St::DisplayBlock, St::PxSm, St::PySm, St::RoundedSm,
                        St::NoDecoration, St::BgAccentSubtle, St::TextAccent12,
                        St::FontMedium, St::CursorPointer,
                    ]
                } else {
                    vec![
                        St::DisplayBlock, St::PxSm, St::PySm, St::RoundedSm,
                        St::NoDecoration, St::TextDefault, St::CursorPointer, St::TransitionColors,
                    ]
                };

                let mut link = Link::to(page_path, &page.title).st(tokens);

                if !is_active {
                    link = link.hover([St::BgHover]);
                }

                link_list = link_list.append([link]);
            }
        }

        nav = nav.append([el(El::Div).append([title, link_list])]);
    }

    nav
}

// ============================================================================
// Documentation Page
// ============================================================================

fn build_doc_page(site: &DocSite, path: &str) -> ElementBuilder {
    let page = match site.page(path) {
        Some(p) => p,
        None => {
            return el(El::Div)
                .st([St::PMd])
                .append([
                    el(El::H1).st([St::Text2xl, St::FontBold, St::MbMd]).text("Page Not Found"),
                    el(El::P).st([St::TextMuted]).text(&format!("No page found at {path}")),
                ]);
        }
    };

    let parsed = parse_markdown(&page.markdown);

    let mut toc = TableOfContents::new();
    for heading in &parsed.headings {
        toc = toc.heading(heading.level, heading.text.clone(), heading.anchor.clone());
    }

    // Find prev/next pages across all sections
    let all_paths: Vec<&str> = site
        .sections()
        .iter()
        .flat_map(|(_, paths)| paths.iter().map(|s| s.as_str()))
        .collect();
    let current_idx = all_paths.iter().position(|p| *p == path);
    let prev_page = current_idx
        .filter(|&i| i > 0)
        .and_then(|i| site.page(all_paths[i - 1]));
    let next_page = current_idx
        .filter(|&i| i + 1 < all_paths.len())
        .and_then(|i| site.page(all_paths[i + 1]));

    let prev_next_nav = build_prev_next_nav(prev_page, next_page);

    Stack::row()
        .gap(Gap::Xl)
        .children([
            el(El::Div)
                .st([St::Flex1, St::MinW0])
                .append([
                    // Breadcrumb
                    el(El::Div)
                        .st([St::DisplayFlex, St::ItemsCenter, St::GapXs, St::TextSm, St::TextMuted, St::MbLg])
                        .append([
                            Link::to("/", "Docs")
                                .st([St::NoDecoration, St::CursorPointer, St::TextMuted])
                                .hover([St::TextDefault]),
                            el(El::Span).text("/"),
                            el(El::Span).text(&strip_section_prefix(&page.section)),
                            el(El::Span).text("/"),
                            el(El::Span).st([St::TextDefault]).text(&page.title),
                        ]),
                    parsed.content,
                    prev_next_nav,
                ]),
            el(El::Div)
                .st([St::FlexShrink0, St::PositionSticky, St::TopHeader])
                .attr("style", "width:220px;align-self:start")
                .append([toc.build()]),
        ])
        .build()
}

fn build_prev_next_nav(prev: Option<&DocPage>, next: Option<&DocPage>) -> ElementBuilder {
    let mut nav = el(El::Nav)
        .st([St::DisplayFlex, St::JustifyBetween, St::ItemsCenter, St::BorderT, St::BorderBDefault])
        .attr("style", "margin-top:3rem;padding-top:1.5rem");

    // Previous link (left-aligned)
    let prev_el = if let Some(p) = prev {
        Link::to(
            &p.path,
            &format!("\u{2190} {}", p.title),
        )
        .st([St::NoDecoration, St::TextMuted, St::TextSm, St::CursorPointer, St::TransitionColors])
        .hover([St::TextAccent])
    } else {
        el(El::Span) // Empty spacer
    };

    // Next link (right-aligned)
    let next_el = if let Some(n) = next {
        Link::to(
            &n.path,
            &format!("{} \u{2192}", n.title),
        )
        .st([St::NoDecoration, St::TextMuted, St::TextSm, St::CursorPointer, St::TransitionColors])
        .hover([St::TextAccent])
    } else {
        el(El::Span)
    };

    nav = nav.append([prev_el, next_el]);
    nav
}

// ============================================================================
// Search Results
// ============================================================================

fn build_search_results(site: &DocSite, query: &str) -> ElementBuilder {
    let results = site.search(query, 20);

    Stack::column()
        .gap(Gap::Md)
        .children([
            el(El::H2)
                .st([St::TextXl, St::FontSemibold])
                .text(&format!("Search results for \"{query}\"")),
            if results.is_empty() {
                EmptyState::new()
                    .title("No results found")
                    .description("Try adjusting your search terms.")
                    .build()
            } else {
                el(El::Div)
                    .st([St::SpaceYSm])
                    .append(
                        results.iter().map(build_search_result_card).collect::<Vec<_>>(),
                    )
            },
        ])
        .build()
}

fn build_search_result_card(result: &SearchResult) -> ElementBuilder {
    let card = Card::new()
        .shadow(CardShadow::None)
        .child(
            Stack::column()
                .gap(Gap::Xs)
                .children([
                    el(El::H3)
                        .st([St::FontMedium, St::TextAccent])
                        .text(&result.title),
                    el(El::P)
                        .st([St::TextXs, St::TextMuted])
                        .text(&result.section),
                    el(El::P)
                        .st([St::TextSm])
                        .text(&result.snippet),
                ])
                .build(),
        )
        .build();

    Link::to_with_content(&result.path, card)
        .st([St::NoDecoration, St::CursorPointer, St::RoundedLg, St::TransitionColors])
        .hover([St::BgHover])
}

// ============================================================================
// Handlers
// ============================================================================

#[handler]
fn on_route_change(state: &mut SiteState, ctx: &rwire::EventContext) {
    if let Some(path) = ctx.text() {
        if path == "/" {
            state.current_path.clear();
        } else {
            state.current_path = path.to_string();
        }
        state.searching = false;
        state.search_query.clear();
    }
}

#[handler]
fn toggle_theme(state: &mut SiteState) {
    state.theme_mode = match state.theme_mode {
        ThemeMode::Light => ThemeMode::Dark,
        ThemeMode::Dark => ThemeMode::Light,
    };
}

#[handler]
fn cycle_theme_style(state: &mut SiteState) {
    state.theme_style = match state.theme_style {
        ThemeStyle::Default => ThemeStyle::Soft,
        ThemeStyle::Soft => ThemeStyle::Brutalist,
        ThemeStyle::Brutalist => ThemeStyle::Minimal,
        ThemeStyle::Minimal => ThemeStyle::Default,
    };
}

#[handler]
fn on_search_input(state: &mut SiteState, ctx: &rwire::EventContext) {
    if let Some(text) = ctx.text() {
        state.search_query = text.to_string();
        state.searching = !state.search_query.is_empty();
    }
}

// ============================================================================
// Footer
// ============================================================================

fn build_footer() -> ElementBuilder {
    Footer::new()
        .logo(
            el(El::Span)
                .attr("style", "font-family:'Quicksand',sans-serif;font-weight:300;letter-spacing:0.02em")
                .st([St::TextLg, St::TextDefault])
                .text("rwire"),
        )
        .tagline("Server-side UI framework with a binary protocol.")
        .column(
            FooterColumn::new("Documentation")
                .link("Getting Started", "/docs/01-getting-started/install")
                .link("Core Concepts", "/docs/02-core-concepts/state")
                .link("Components", "/docs/03-components/overview"),
        )
        .column(
            FooterColumn::new("Community")
                .external_link("GitHub", "https://github.com")
                .external_link("Discord", "https://discord.gg"),
        )
        .copyright("\u{00a9} 2026 rwire contributors. MIT License.")
        .build()
}
