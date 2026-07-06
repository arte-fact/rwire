//! rwire Docs
//!
//! The official rwire documentation site.
//! Built with rwire itself, demonstrating the framework's capabilities.

use rwire::capsule_gen::{CapsuleConfig, FontFace};
use rwire::icons::{icon, Icon};
use rwire::router::Link;
use rwire::style_tokens::St;
use rwire::theme::{Theme, ThemeMode, ThemeStyle};
use rwire::{el, handler, renderer, theme, El, ElementBuilder, Ev, Server, State};
use rwire_components::*;
use rwire_markdown::{parse_markdown, DocPage, DocSite, TableOfContents};
use rwire_themes::{palettes, styles};

// ============================================================================
// Configuration
// ============================================================================

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

struct Config {
    bind_addr: String,
    docs_dir: String,
    website_url: String,
    design_system_url: String,
    examples_url: String,
}

impl Config {
    fn from_env() -> Self {
        Self {
            bind_addr: env_or("BIND_ADDR", "0.0.0.0:9001"),
            docs_dir: env_or("DOCS_DIR", concat!(env!("CARGO_MANIFEST_DIR"), "/docs")),
            website_url: env_or("WEBSITE_URL", "http://127.0.0.1:9000"),
            design_system_url: env_or("DESIGN_SYSTEM_URL", "http://127.0.0.1:9002"),
            examples_url: env_or("EXAMPLES_URL", "http://127.0.0.1:9003"),
        }
    }
}

// ============================================================================
// State
// ============================================================================

#[derive(State, Default)]
#[storage(memory)]
struct DocsState {
    current_path: String,
    search_query: String,
    searching: bool,
    sidebar_open: bool,
}

// ============================================================================
// Entry Point
// ============================================================================

// Store docs_dir and cross-site URLs as static strings (set once at startup)
static mut DOCS_DIR: &str = "";
static mut WEBSITE_URL: &str = "";
static mut DESIGN_SYSTEM_URL: &str = "";
static mut EXAMPLES_URL: &str = "";

fn docs_dir() -> &'static str {
    unsafe { DOCS_DIR }
}

fn website_url() -> &'static str {
    unsafe { WEBSITE_URL }
}

fn design_system_url() -> &'static str {
    unsafe { DESIGN_SYSTEM_URL }
}

fn examples_url() -> &'static str {
    unsafe { EXAMPLES_URL }
}

#[theme]
fn site_theme() -> Theme {
    Theme::default().palette(palettes::nord())
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env();

    // Leak strings so they live for 'static — only called once at startup
    unsafe {
        DOCS_DIR = Box::leak(config.docs_dir.into_boxed_str());
        WEBSITE_URL = Box::leak(config.website_url.into_boxed_str());
        DESIGN_SYSTEM_URL = Box::leak(config.design_system_url.into_boxed_str());
        EXAMPLES_URL = Box::leak(config.examples_url.into_boxed_str());
    }

    println!("rwire Docs");
    println!("Open http://127.0.0.1:9001 in your browser");
    println!();

    // Navigation uses the `on_route` + root-rerender model: `Link` clicks send the
    // path, `on_route_change` updates `DocsState.current_path`, and the `root`
    // renderer re-renders the page from it. (No `Router`/`outlet()` here — installing
    // a router would route navigation through the outlet swap instead, which this
    // app has no outlet for, freezing the page.)
    let capsule_config =
        CapsuleConfig::new().font(FontFace::google("Quicksand", &[300, 400, 600, 700]));

    Server::bind(&config.bind_addr)?
        .root(root)
        .on_route(on_route_change())
        .capsule_config(capsule_config)
        .theme(site_theme())
        .run()
        .await
}

// ============================================================================
// Root Layout
// ============================================================================

#[renderer]
fn root(state: &DocsState) -> ElementBuilder {
    let site = DocSite::load(docs_dir());

    // Default to first doc page if path is empty
    let path = if state.current_path.is_empty() {
        site.sections()
            .first()
            .and_then(|(_, paths)| paths.first())
            .cloned()
            .unwrap_or_default()
    } else {
        state.current_path.clone()
    };

    let main_content = build_doc_page(&site, &path);
    let sidebar = build_sidebar(&site, &path);

    // Mobile drawer (visible only when sidebar_open on small screens)
    let mobile_drawer = Drawer::new()
        .title("Navigation")
        .position(DrawerPosition::Left)
        .open(state.sidebar_open)
        .on_close(toggle_sidebar())
        .content(build_sidebar(&site, &path))
        .build();

    // Desktop sidebar: hidden on mobile, visible at md+
    let desktop_sidebar = el(El::Aside)
        .st([
            St::DisplayNone,
            St::BgSidebar,
            St::BorderRDefault,
            St::OverflowYScroll,
            St::PyMd,
        ])
        .md([St::DisplayBlock])
        .attr(
            "style",
            "position:sticky;top:56px;height:calc(100vh - 56px);width:260px;flex-shrink:0",
        )
        .append([sidebar]);

    let body = el(El::Div).st([St::DisplayFlex, St::MinHScreen]).append([
        desktop_sidebar,
        el(El::Main)
            .st([St::Flex1, St::MinW0, St::PMd])
            .append([main_content]),
    ]);

    el(El::Div)
        .st([St::BgApp, St::TextDefault, St::MinHScreen])
        .append([build_header(), mobile_drawer, body, build_footer()])
}

// ============================================================================
// Header
// ============================================================================

fn build_header() -> ElementBuilder {
    // Hamburger button: visible on mobile, hidden at md+
    let hamburger = el(El::Button)
        .st([
            St::DisplayFlex,
            St::ItemsCenter,
            St::JustifyCenter,
            St::BgTransparent,
            St::BorderNone,
            St::CursorPointer,
            St::TextMuted,
            St::P0,
        ])
        .md([St::DisplayNone])
        .on(Ev::Click, toggle_sidebar())
        .append([icon(Icon::Menu)]);

    // Left side: hamburger + logo (logo links to main website)
    let left = Stack::row()
        .gap(Gap::Sm)
        .align(StackAlign::Center)
        .children([
            hamburger,
            el(El::A)
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
                ]),
        ])
        .build();

    // Right side: search + nav
    let right = Stack::row()
        .gap(Gap::Sm)
        .align(StackAlign::Center)
        .children([
            el(El::Div)
                .st([St::DisplayNone])
                .md([St::DisplayBlock])
                .append([render_search_input()]),
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
fn render_search_input(state: &DocsState) -> ElementBuilder {
    let input = Input::search()
        .placeholder("Search docs...")
        .size(InputSize::Sm)
        .id("search-input")
        .value(state.search_query.clone())
        .on_input_debounced(on_search_input(), 300);

    let mut container = el(El::Div).st([St::PositionRelative]);
    container = container.append([input]);

    if state.searching {
        let site = DocSite::load(docs_dir());
        let results = site.search(&state.search_query, 8);

        // Transparent full-screen backdrop for click-outside dismiss
        let backdrop = el(El::Div)
            .st([St::PositionFixed, St::Inset0, St::Z40])
            .on(Ev::Click, close_search());

        // Dropdown panel positioned below input
        let panel = el(El::Div)
            .st([
                St::PositionAbsolute,
                St::Right0,
                St::Z50,
                St::BgApp,
                St::BorderSubtle,
                St::RoundedMd,
                St::ShadowLg,
                St::OverflowYAuto,
                St::PySm,
                St::MaxW360px,
            ])
            .attr("style", "top:100%;margin-top:4px;max-height:400px")
            .append(if results.is_empty() {
                vec![el(El::Div)
                    .st([St::PxMd, St::PySm, St::TextMuted, St::TextSm])
                    .text("No results found")]
            } else {
                results
                    .iter()
                    .map(|r| {
                        Link::to_with_content(
                            &r.path,
                            el(El::Div).append([
                                el(El::Div).st([St::FontMedium, St::TextSm]).text(&r.title),
                                el(El::Div).st([St::TextXs, St::TextMuted]).text(&r.section),
                            ]),
                        )
                        .st([
                            St::DisplayBlock,
                            St::PxMd,
                            St::PySm,
                            St::NoDecoration,
                            St::TextDefault,
                            St::TransitionColors,
                            St::CursorPointer,
                        ])
                        .hover([St::BgHover])
                    })
                    .collect()
            });

        container = container.append([backdrop, panel]);
    }

    container
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
    // The generic TreeView (F8 dogfood): each section is a collapsible native
    // <details> branch, expanded when it holds the active page; leaves carry
    // the router Links so navigation stays a data-route interception.
    use rwire_components::{TreeNode, TreeView};

    let mut roots = Vec::new();
    for (section_name, page_paths) in site.sections() {
        let title = el(El::Span)
            .st([
                St::TextXsMuted,
                St::FontSemibold,
                St::TextUppercase,
                St::TrackingWider,
            ])
            .text(&strip_section_prefix(section_name));

        let mut leaves = Vec::new();
        for page_path in page_paths {
            if let Some(page) = site.page(page_path) {
                let is_active = active_path == page_path;
                let link = Link::to(page_path, &page.title).st(if is_active {
                    vec![St::NoDecoration, St::TextAccent12, St::FontMedium]
                } else {
                    vec![St::NoDecoration, St::TextDefault]
                });
                leaves.push(
                    TreeNode::leaf(page_path.trim_start_matches('/').to_string(), link)
                        .selected(is_active),
                );
            }
        }
        roots.push(
            TreeNode::branch(strip_section_prefix(section_name), title, leaves).expanded(true),
        );
    }

    el(El::Nav)
        .st([St::DisplayFlex, St::FlexCol, St::PxSm, St::TextSm])
        .append([TreeView::new().roots(roots).build()])
}

// ============================================================================
// Documentation Page
// ============================================================================

fn build_doc_page(site: &DocSite, path: &str) -> ElementBuilder {
    let page = match site.page(path) {
        Some(p) => p,
        None => {
            return el(El::Div).st([St::PMd]).append([
                el(El::H1)
                    .st([St::Text2xl, St::FontBold, St::MbMd])
                    .text("Page Not Found"),
                el(El::P)
                    .st([St::TextMuted])
                    .text(&format!("No page found at {path}")),
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

    el(El::Div).st([St::DisplayFlex, St::GapXl]).append([
        el(El::Div).st([St::Flex1, St::MinW0]).append([
            // Breadcrumb (hidden on mobile)
            el(El::Div)
                .st([
                    St::DisplayNone,
                    St::ItemsCenter,
                    St::GapXs,
                    St::TextSm,
                    St::TextMuted,
                    St::MbLg,
                ])
                .md([St::DisplayFlex])
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
        // Table of contents: hidden on mobile, visible at lg+
        el(El::Div)
            .st([
                St::DisplayNone,
                St::FlexShrink0,
                St::PositionSticky,
                St::TopHeader,
            ])
            .lg([St::DisplayBlock])
            .st([St::W220px, St::SelfStart])
            .append([toc.build()]),
    ])
}

fn build_prev_next_nav(prev: Option<&DocPage>, next: Option<&DocPage>) -> ElementBuilder {
    let mut nav = el(El::Nav)
        .st([
            St::DisplayFlex,
            St::JustifyBetween,
            St::ItemsCenter,
            St::BorderT,
            St::BorderBDefault,
        ])
        .st([St::Mt2xl, St::PtLg]);

    let prev_el = if let Some(p) = prev {
        Link::to(&p.path, &format!("\u{2190} {}", p.title))
            .st([
                St::NoDecoration,
                St::TextMuted,
                St::TextSm,
                St::CursorPointer,
                St::TransitionColors,
            ])
            .hover([St::TextAccent])
    } else {
        el(El::Span)
    };

    let next_el = if let Some(n) = next {
        Link::to(&n.path, &format!("{} \u{2192}", n.title))
            .st([
                St::NoDecoration,
                St::TextMuted,
                St::TextSm,
                St::CursorPointer,
                St::TransitionColors,
            ])
            .hover([St::TextAccent])
    } else {
        el(El::Span)
    };

    nav = nav.append([prev_el, next_el]);
    nav
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
            FooterColumn::new("Documentation")
                .link("Getting Started", "/docs/01-getting-started/install")
                .link("Core Concepts", "/docs/02-core-concepts/state")
                .link("Components", "/docs/03-components/overview"),
        )
        .column(
            FooterColumn::new("Ecosystem")
                .external_link("Website", website_url())
                .external_link("Components", design_system_url())
                .external_link("Examples", examples_url()),
        )
        .column(
            FooterColumn::new("Community")
                .external_link("GitHub", "https://github.com")
                .external_link("Discord", "https://discord.gg"),
        )
        .copyright("\u{00a9} 2026 rwire contributors. MIT License.")
        .build()
}

// ============================================================================
// Handlers
// ============================================================================

#[handler]
fn on_route_change(state: &mut DocsState, ctx: &rwire::EventContext) {
    if let Some(path) = ctx.text() {
        if path == "/" {
            state.current_path.clear();
        } else {
            state.current_path = path.to_string();
        }
        state.searching = false;
        state.search_query.clear();
        state.sidebar_open = false;
    }
}

#[handler]
fn toggle_sidebar(state: &mut DocsState) {
    state.sidebar_open = !state.sidebar_open;
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

#[handler]
fn on_search_input(state: &mut DocsState, ctx: &rwire::EventContext) {
    if let Some(text) = ctx.text() {
        state.search_query = text.to_string();
        state.searching = !state.search_query.is_empty();
    }
}

#[handler]
fn close_search(state: &mut DocsState) {
    state.searching = false;
    state.search_query.clear();
}
