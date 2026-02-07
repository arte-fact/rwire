//! rwire Documentation Site
//!
//! A full documentation site built with rwire components and the docs module.
//! Demonstrates AppShell, DocsSidebar, TableOfContents, Prose, and search.

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
/// In a real app this would use include_str! or embed at compile time.
static DOCS_DIR: &str = "docs";

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("rwire Documentation Site");
    println!("Open http://127.0.0.1:9000 in your browser");
    println!();

    let capsule_config = CapsuleConfig::new().theme(Theme::default());

    Server::bind("127.0.0.1:9000")?
        .root(root)
        .capsule_config(capsule_config)
        .run()
        .await
}

// ============================================================================
// Root Layout
// ============================================================================

#[renderer]
fn root(state: &DocState) -> ElementBuilder {
    // Load docs on every render (in production, cache this)
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
            el(El::A)
                .st([St::FontBold, St::TextLg, St::NoDecoration, St::TextDefault])
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
// Sidebar
// ============================================================================

fn build_sidebar(site: &DocSite, active_path: &str) -> ElementBuilder {
    let mut sidebar = DocsSidebar::new();

    for (section_name, page_paths) in site.sections() {
        let title = section_name.replace('-', " ");
        let mut section = SidebarSection::new(title);

        for page_path in page_paths {
            if let Some(page) = site.page(page_path) {
                section = section.link(page_path.clone(), page.title.clone());
            }
        }

        sidebar = sidebar.section(section);
    }

    if !active_path.is_empty() {
        sidebar = sidebar.active_path(active_path.to_string());
    }

    sidebar.build()
}

// ============================================================================
// Landing Page
// ============================================================================

fn build_landing_page(site: &DocSite) -> ElementBuilder {
    Stack::column()
        .gap(Gap::Lg)
        .children([
            // Hero section
            el(El::Div)
                .st([St::PyXl, St::TextCenter])
                .append([
                    el(El::H1)
                        .st([St::Text2xl, St::FontBold, St::MbMd])
                        .text("rwire Documentation"),
                    el(El::P)
                        .st([St::TextLg, St::TextMuted, St::MbLg])
                        .text("Server-side UI framework with a binary protocol and ~1.5KB JS runtime."),
                ]),
            // Quick links
            Stack::row()
                .gap(Gap::Md)
                .justify(StackJustify::Center)
                .children(
                    site.sections()
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
                                                    .text(
                                                        page.description
                                                            .as_deref()
                                                            .unwrap_or(""),
                                                    ),
                                            ])
                                            .build(),
                                    )
                                    .build()
                            })
                        })
                        .collect::<Vec<_>>(),
                )
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
        .gap(Gap::Lg)
        .children([
            // Main content
            el(El::Div)
                .st([St::Flex1, St::MinW0])
                .append([
                    // Breadcrumb
                    Breadcrumb::new()
                        .item("Docs", Some("/"))
                        .item(page.section.clone(), None::<&str>)
                        .item(page.title.clone(), None::<&str>)
                        .build(),
                    // Rendered markdown
                    el(El::Div)
                        .st([St::MtMd])
                        .append([parsed.content]),
                ]),
            // TOC (right side)
            el(El::Div)
                .st([St::FlexShrink0])
                .attr("style", "width:200px")
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
                .text(&format!("Search results for \"{}\"", query)),
            if results.is_empty() {
                el(El::P)
                    .st([St::TextMuted])
                    .text("No results found. Try a different search term.")
            } else {
                el(El::Div)
                    .st([St::SpaceYSm])
                    .append(
                        results
                            .iter()
                            .map(build_search_result_card)
                            .collect::<Vec<_>>(),
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
}

// ============================================================================
// Handlers
// ============================================================================

#[handler]
fn navigate_home(state: &mut DocState) {
    state.current_path.clear();
    state.searching = false;
    state.search_query.clear();
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
