//! rwire Documentation Site
//!
//! A full documentation site built with rwire components and the docs module.
//! Demonstrates AppShell, TableOfContents, Prose, and search.

use rwire::capsule_gen::CapsuleConfig;
use rwire::components::*;
use rwire::docs::{parse_markdown, DocSite, SearchResult};
use rwire::style_tokens::St;
use rwire::theme::{Theme, ThemeMode};
use rwire::{el, handler, renderer, El, ElementBuilder, Ev, Server, State};

// ============================================================================
// State
// ============================================================================

#[derive(State, Default)]
#[storage(memory)]
struct DocState {
    /// Current page path (e.g., "/docs/getting-started/install").
    current_path: String,
    /// Search query.
    search_query: String,
    /// Whether search results are shown.
    searching: bool,
    /// Theme mode (light/dark).
    theme_mode: ThemeMode,
}

// ============================================================================
// Entry Point
// ============================================================================

/// Load docs from the docs/ directory at startup.
/// Uses CARGO_MANIFEST_DIR to resolve relative to the crate, not the CWD.
static DOCS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/docs");

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("rwire Documentation Site");
    println!("Open http://127.0.0.1:9000 in your browser");
    println!();

    let capsule_config = CapsuleConfig::new().theme(Theme::default());

    Server::bind("0.0.0.0:9000")?
        .root(root)
        .on_route(on_route_change())
        .capsule_config(capsule_config)
        .run()
        .await
}

// ============================================================================
// Root Layout
// ============================================================================

#[renderer]
fn root(state: &DocState) -> ElementBuilder {
    let site = DocSite::load(DOCS_DIR);

    let sidebar = build_sidebar(&site, &state.current_path);
    let main_content = if state.searching {
        build_search_results(&site, &state.search_query)
    } else if state.current_path.is_empty() {
        build_landing_page(&site)
    } else {
        build_doc_page(&site, &state.current_path)
    };

    el(El::Div)
        .attr("data-theme", state.theme_mode.as_str())
        .st([St::BgApp, St::TextDefault, St::MinHScreen])
        .append([
            AppShell::new()
                .header(build_header())
                .sidebar(sidebar)
                .main(main_content)
                .build(),
        ])
}

// ============================================================================
// Header
// ============================================================================

fn build_header() -> ElementBuilder {
    Stack::row()
        .gap(Gap::Md)
        .justify(StackJustify::Between)
        .align(StackAlign::Center)
        .children([
            // Logo / title
            el(El::Span)
                .st([St::FontBold, St::TextLg, St::CursorPointer, St::TextDefault])
                .text("rwire Docs")
                .on(Ev::Click, navigate_home()),
            // Right side: search + theme toggle
            Stack::row()
                .gap(Gap::Sm)
                .align(StackAlign::Center)
                .children([
                    render_search_input(),
                    render_theme_toggle(),
                ])
                .build(),
        ])
        .build()
}

#[renderer]
fn render_search_input(state: &DocState) -> ElementBuilder {
    Input::search()
        .placeholder("Search docs...")
        .size(InputSize::Sm)
        .value(state.search_query.clone())
        .on_input(on_search_input())
}

#[renderer]
fn render_theme_toggle(state: &DocState) -> ElementBuilder {
    ThemeToggle::new()
        .mode(match state.theme_mode {
            ThemeMode::Light => ThemeToggleMode::Light,
            ThemeMode::Dark => ThemeToggleMode::Dark,
        })
        .on_toggle(toggle_theme())
        .build()
}

// ============================================================================
// Sidebar (click-handler based, not <a href>)
// ============================================================================

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
            .text(&section_name.replace('-', " "));

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
                        St::TextDefault, St::CursorPointer, St::TransitionColors,
                    ]
                };

                let mut link = el(El::Div)
                    .st(tokens)
                    .text(&page.title)
                    .data("path", page_path)
                    .on(Ev::Click, navigate_to());

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
// Landing Page
// ============================================================================

fn build_landing_page(site: &DocSite) -> ElementBuilder {
    let cards: Vec<ElementBuilder> = site
        .sections()
        .iter()
        .flat_map(|(_, paths)| paths.first().cloned())
        .filter_map(|path| {
            site.page(&path).map(|page| {
                Card::new()
                    .child(
                        Stack::column()
                            .gap(Gap::Sm)
                            .children([
                                el(El::H3)
                                    .st([St::FontSemibold])
                                    .text(&page.title),
                                el(El::P)
                                    .st([St::TextSm, St::TextMuted])
                                    .text(page.description.as_deref().unwrap_or("")),
                            ])
                            .build(),
                    )
                    .build()
                    .st([St::CursorPointer])
                    .data("path", &path)
                    .on(Ev::Click, navigate_to())
                    .hover([St::BgHover])
            })
        })
        .collect();

    Stack::column()
        .gap(Gap::Xl)
        .children([
            // Hero section
            el(El::Div)
                .st([St::PyXl, St::TextCenter])
                .append([
                    el(El::H1)
                        .st([St::Text3xl, St::FontBold, St::MbMd])
                        .text("rwire Documentation"),
                    el(El::P)
                        .st([St::TextLg, St::TextMuted, St::MbLg, St::LeadingRelaxed])
                        .text("Server-side UI framework with a binary protocol and ~1.5KB JS runtime."),
                ]),
            // Quick links
            Stack::row()
                .gap(Gap::Lg)
                .justify(StackJustify::Center)
                .children(cards)
                .build(),
            // Stats
            el(El::P)
                .st([St::TextCenter, St::TextSm, St::TextMuted])
                .text(&format!(
                    "{} pages across {} sections",
                    site.page_count(),
                    site.sections().len()
                )),
        ])
        .build()
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

    // Build table of contents from headings
    let mut toc = TableOfContents::new();
    for heading in &parsed.headings {
        toc = toc.heading(heading.level, heading.text.clone(), heading.anchor.clone());
    }

    // Layout: content + TOC sidebar
    Stack::row()
        .gap(Gap::Xl)
        .children([
            // Main content
            el(El::Div)
                .st([St::Flex1, St::MinW0])
                .append([
                    // Breadcrumb navigation (clickable)
                    el(El::Div)
                        .st([St::DisplayFlex, St::ItemsCenter, St::GapXs, St::TextSm, St::TextMuted, St::MbLg])
                        .append([
                            el(El::Span)
                                .st([St::CursorPointer])
                                .hover([St::TextDefault])
                                .text("Docs")
                                .on(Ev::Click, navigate_home()),
                            el(El::Span).text("/"),
                            el(El::Span).text(&page.section.replace('-', " ")),
                            el(El::Span).text("/"),
                            el(El::Span).st([St::TextDefault]).text(&page.title),
                        ]),
                    // Rendered markdown
                    parsed.content,
                ]),
            // TOC (right side, sticky below header)
            el(El::Div)
                .st([St::FlexShrink0, St::PositionSticky, St::TopHeader])
                .attr("style", "width:220px;align-self:start")
                .append([toc.build()]),
        ])
        .build()
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
    Card::new()
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
        .build()
        .st([St::CursorPointer])
        .data("path", &result.path)
        .on(Ev::Click, navigate_to())
        .hover([St::BgHover])
}

// ============================================================================
// Handlers
// ============================================================================

#[handler]
fn on_route_change(state: &mut DocState, ctx: &rwire::EventContext) {
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
fn navigate_home(state: &mut DocState) {
    state.current_path.clear();
    state.searching = false;
    state.search_query.clear();
}

#[handler]
fn navigate_to(state: &mut DocState, ctx: &rwire::EventContext) {
    if let Some(path) = ctx.data("path") {
        state.current_path = path.to_string();
        state.searching = false;
        state.search_query.clear();
    }
}

#[handler]
fn toggle_theme(state: &mut DocState) {
    state.theme_mode = match state.theme_mode {
        ThemeMode::Light => ThemeMode::Dark,
        ThemeMode::Dark => ThemeMode::Light,
    };
}

#[handler]
fn on_search_input(state: &mut DocState, ctx: &rwire::EventContext) {
    if let Some(text) = ctx.text() {
        state.search_query = text.to_string();
        state.searching = !state.search_query.is_empty();
    }
}
