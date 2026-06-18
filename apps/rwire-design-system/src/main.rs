//! rwire Design System
//!
//! Auto-generated component documentation site driven by the component catalog.

use rwire::capsule_gen::{CapsuleConfig, FontFace};
use rwire::icons::{icon, Icon};
use rwire::router::{Link, Router};
use rwire::style_tokens::St;
use rwire::theme::{Theme, ThemeMode, ThemeStyle};
use rwire::{el, handler, renderer, theme, At, El, ElementBuilder, Ev, Server, State};
use rwire_components::catalog::{self, Category, ComponentEntry};
use rwire_components::*;
use rwire_markdown::Markdown;
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
    examples_url: String,
}

impl Config {
    fn from_env() -> Self {
        Self {
            bind_addr: env_or("BIND_ADDR", "0.0.0.0:9002"),
            website_url: env_or("WEBSITE_URL", "http://127.0.0.1:9000"),
            docs_url: env_or("DOCS_URL", "http://127.0.0.1:9001"),
            examples_url: env_or("EXAMPLES_URL", "http://127.0.0.1:9003"),
        }
    }
}

static mut WEBSITE_URL: &str = "";
static mut DOCS_URL: &str = "";
static mut EXAMPLES_URL: &str = "";
static mut DOCS_DIR: &str = "";

fn website_url() -> &'static str {
    unsafe { WEBSITE_URL }
}
fn docs_url() -> &'static str {
    unsafe { DOCS_URL }
}
fn examples_url() -> &'static str {
    unsafe { EXAMPLES_URL }
}
fn docs_dir() -> &'static str {
    unsafe { DOCS_DIR }
}

// ============================================================================
// State
// ============================================================================

#[derive(State, Default)]
#[storage(memory)]
struct DesignSystemState {
    current_slug: String,
    sidebar_open: bool,
    // Playground state (reset on component navigation)
    variant_selections: Vec<usize>,
    bool_states: Vec<bool>,
}

// ============================================================================
// Entry Point
// ============================================================================

#[theme]
fn app_theme() -> Theme {
    Theme::default().palette(palettes::indigo())
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env();

    unsafe {
        WEBSITE_URL = Box::leak(config.website_url.into_boxed_str());
        DOCS_URL = Box::leak(config.docs_url.into_boxed_str());
        EXAMPLES_URL = Box::leak(config.examples_url.into_boxed_str());
        DOCS_DIR = Box::leak(
            env_or(
                "DOCS_DIR",
                concat!(env!("CARGO_MANIFEST_DIR"), "/../../libs/rwire-components/docs"),
            )
            .into_boxed_str(),
        );
    }

    println!("rwire Design System");
    println!("Open http://127.0.0.1:9002 in your browser");
    println!();

    let router = Router::new()
        .page("/", |_| build_landing())
        .page("/components/:slug", |params| {
            let slug = params.get("slug").unwrap_or("");
            build_component_page(slug)
        });

    // Element maps are shipped whole now — markdown elements need no declaring.
    let capsule_config = CapsuleConfig::new()
        .font(FontFace::google("Quicksand", &[300, 400, 600, 700]));

    Server::bind(&config.bind_addr)?
        .root(root)
        .on_route(on_route_change())
        .routes(router)
        .capsule_config(capsule_config)
        .theme(app_theme())
        .run()
        .await
}

// ============================================================================
// Root Layout
// ============================================================================

#[renderer]
fn root(state: &DesignSystemState) -> ElementBuilder {
    let slug_str = &state.current_slug;
    let on_landing = slug_str.is_empty();

    // Mobile drawer: full sidebar on component pages, empty placeholder on landing
    let mobile_drawer = if on_landing {
        el(El::Div)
    } else {
        Drawer::new()
            .title("Components")
            .position(DrawerPosition::Left)
            .open(state.sidebar_open)
            .on_close(toggle_sidebar())
            .content(build_sidebar(slug_str))
            .build()
    };

    // Body content: landing page vs component page with sidebar
    let body = if on_landing {
        el(El::Div)
            .st([St::DisplayFlex, St::MinHScreen])
            .append([
                el(El::Main).st([St::Flex1, St::MinW0]).append([build_landing()]),
            ])
    } else {
        let entry = catalog::find(slug_str);

        // Desktop sidebar
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
                "position:sticky;top:56px;height:calc(100vh - 56px);width:240px;flex-shrink:0",
            )
            .append([build_sidebar(slug_str)]);

        // Main content
        let main_content = if let Some(entry) = entry {
            build_component_content(entry, state)
        } else {
            el(El::Div).st([St::PMd]).append([
                el(El::H1)
                    .st([St::Text2xl, St::FontBold, St::MbMd])
                    .text("Component Not Found"),
                el(El::P)
                    .st([St::TextMuted])
                    .text(&format!("No component found with slug \"{slug_str}\"")),
            ])
        };

        el(El::Div)
            .st([St::DisplayFlex, St::MinHScreen])
            .append([
                desktop_sidebar,
                el(El::Main)
                    .st([St::Flex1, St::MinW0, St::PMd, St::PbXl])
                    .append([main_content]),
            ])
    };

    // Always: Div > [Header, Drawer, Body, Footer] — stable 4-child structure
    el(El::Div)
        .st([St::BgApp, St::TextDefault, St::MinHScreen])
        .append([build_header(), mobile_drawer, body, build_footer()])
}

// ============================================================================
// Header
// ============================================================================

fn build_header() -> ElementBuilder {
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
        .at_str(At::AriaLabel, "Open menu")
        .on(Ev::Click, toggle_sidebar())
        .append([icon(Icon::Menu)]);

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
                .st([St::TextLg, St::CursorPointer, St::TextDefault, St::NoDecoration])
                .text("rwire"),
            Badge::primary("Design System").build(),
        ])
        .build();

    let right = Stack::row()
        .gap(Gap::Sm)
        .align(StackAlign::Center)
        .children([
            el(El::A)
                .attr("href", docs_url())
                .st([St::TextSm, St::TextMuted, St::NoDecoration, St::CursorPointer])
                .hover([St::TextDefault])
                .text("Docs"),
            el(El::A)
                .attr("href", examples_url())
                .st([St::TextSm, St::TextMuted, St::NoDecoration, St::CursorPointer])
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
            Button::ghost({
                let is_nord = theme.palette_ref().is_some_and(|p| p.accent.step(8) == "#5E81AC");
                if is_nord { "Nord" } else if theme.palette_ref().is_some() { "Indigo" } else { "Default" }
            })
            .size(ButtonSize::Sm)
            .on_click(cycle_palette()),
        ])
        .build()
}

// ============================================================================
// Sidebar
// ============================================================================

fn build_sidebar(active_slug: &str) -> ElementBuilder {
    let catalog = catalog::catalog();
    let mut nav = el(El::Nav).st([
        St::DisplayFlex,
        St::FlexCol,
        St::GapMd,
        St::PxSm,
        St::TextSm,
    ]);

    for category in Category::ALL {
        let category_entries: Vec<&ComponentEntry> = catalog
            .iter()
            .filter(|e| e.category == *category)
            .collect();

        if category_entries.is_empty() {
            continue;
        }

        let title = el(El::Div)
            .st([
                St::TextXsMuted,
                St::FontSemibold,
                St::TextUppercase,
                St::TrackingWider,
                St::PxSm,
                St::PySm,
            ])
            .text(category.label());

        let mut link_list = el(El::Div).st([St::DisplayFlex, St::FlexCol]);

        for entry in category_entries {
            let is_active = active_slug == entry.slug;
            let href = format!("/components/{}", entry.slug);

            let tokens = if is_active {
                vec![
                    St::DisplayBlock,
                    St::PxSm,
                    St::PySm,
                    St::RoundedSm,
                    St::NoDecoration,
                    St::BgAccentSubtle,
                    St::TextAccent12,
                    St::FontMedium,
                    St::CursorPointer,
                ]
            } else {
                vec![
                    St::DisplayBlock,
                    St::PxSm,
                    St::PySm,
                    St::RoundedSm,
                    St::NoDecoration,
                    St::TextDefault,
                    St::CursorPointer,
                    St::TransitionColors,
                ]
            };

            let mut link = Link::to(&href, entry.name).st(tokens);
            if !is_active {
                link = link.hover([St::BgHover]);
            }
            link_list = link_list.append([link]);
        }

        nav = nav.append([el(El::Div).append([title, link_list])]);
    }

    nav
}

// ============================================================================
// Landing Page
// ============================================================================

fn build_landing() -> ElementBuilder {
    let catalog = catalog::catalog();

    let hero = Stack::column()
        .gap(Gap::Md)
        .align(StackAlign::Center)
        .children([
            el(El::H1)
                .st([St::Text3xl, St::FontBold, St::TextCenter])
                .text("rwire Design System"),
            el(El::P)
                .st([St::TextLg, St::TextMuted, St::TextCenter, St::MaxWLg])
                .text("50 production-ready components with type-safe variants, design tokens, and theme support."),
        ])
        .build();

    let mut cards = Vec::new();
    for category in Category::ALL {
        let entries: Vec<&ComponentEntry> = catalog
            .iter()
            .filter(|e| e.category == *category)
            .collect();

        let count = entries.len();
        let names: Vec<&str> = entries.iter().take(5).map(|e| e.name).collect();
        let first_slug = entries.first().map(|e| e.slug).unwrap_or("button");

        let card = Link::to_with_content(
            &format!("/components/{first_slug}"),
            Card::new()
                .child(
                    Stack::column()
                        .gap(Gap::Sm)
                        .children([
                            el(El::H3).st([St::FontSemibold, St::TextLg]).text(category.label()),
                            el(El::P)
                                .st([St::TextSm, St::TextMuted])
                                .text(&format!("{count} components")),
                            el(El::P)
                                .st([St::TextSm, St::TextMuted])
                                .text(&names.join(", ")),
                        ])
                        .build(),
                )
                .build(),
        )
        .st([St::NoDecoration, St::CursorPointer])
        .hover([St::BgHover]);

        cards.push(card);
    }

    Container::new()
        .centered(true)
        .padding(true)
        .child(
            Stack::column()
                .gap(Gap::Xl)
                .children([
                    el(El::Div).st([St::PtXl, St::PbLg]).append([hero]),
                    Grid::new()
                        .columns(GridColumns::Fixed3)
                        .gap(Gap::Md)
                        .children(cards)
                        .build(),
                ])
                .build(),
        )
        .build()
}

// ============================================================================
// Component Page (for router tree-shaking)
// ============================================================================

fn build_component_page(slug: &str) -> ElementBuilder {
    // When called with empty slug (router tree-shaking), build ALL demos
    // so every component's tokens get discovered.
    let demo = if slug.is_empty() {
        let demos: Vec<ElementBuilder> = catalog::catalog()
            .iter()
            .map(|e| (e.build_demo)(&[], &[]))
            .collect();
        el(El::Div).append(demos)
    } else {
        catalog::find(slug)
            .map(|e| (e.build_demo)(&[], &[]))
            .unwrap_or_else(|| el(El::Div))
    };

    // Return a representative tree with all element types used in the full page
    // layout so they get tree-shaken into the capsule.
    el(El::Div).append([
        // Sidebar: needs md:DisplayBlock for responsive visibility
        el(El::Aside)
            .st([St::DisplayNone])
            .md([St::DisplayBlock])
            .append([el(El::Nav).append([
                Link::to("/", "x").hover([St::BgHover]),
            ])]),
        // Main content wrapper
        el(El::Main).append([
            // Breadcrumb
            Breadcrumb::new()
                .item("x", Some("/"))
                .item("y", None::<&str>)
                .build(),
            // Playground controls (Input, Label via Checkbox)
            Checkbox::new().label("x").build(),
            // Code example
            Code::block("x").language("rust").build(),
            // Props table
            Table::new().headers(["a"]).build(),
            // Variant selector button (captures hover pseudo pair)
            el(El::Button)
                .st([St::BgSecondary])
                .hover([St::BgSecondaryHover]),
            // Component demo
            demo,
        ]),
    ])
}

// ============================================================================
// Component Content (rendered at runtime via root renderer)
// ============================================================================

fn build_component_content(entry: &ComponentEntry, state: &DesignSystemState) -> ElementBuilder {
    // Breadcrumb
    let breadcrumb = Breadcrumb::new()
        .item("Components", Some("/"))
        .item(entry.category.label(), None::<&str>)
        .item(entry.name, None::<&str>)
        .build();

    // Title + description
    let header = Stack::column()
        .gap(Gap::Xs)
        .children([
            el(El::H1)
                .st([St::Text3xl, St::FontBold])
                .text(entry.name),
            el(El::P)
                .st([St::TextLg, St::TextMuted])
                .text(entry.description),
        ])
        .build();

    // Playground
    let playground = build_playground(entry, state);

    // Code example
    let code_example = build_code_example(entry, state);

    // Props table
    let props_table = build_props_table(entry);

    // Markdown docs
    let markdown_docs = build_markdown_docs(entry);

    Stack::column()
        .gap(Gap::Lg)
        .children([breadcrumb, header, playground, code_example, props_table, markdown_docs])
        .build()
}

// ============================================================================
// Playground
// ============================================================================

fn build_playground(entry: &ComponentEntry, state: &DesignSystemState) -> ElementBuilder {
    let mut controls = Vec::new();

    // Variant selectors
    for (axis_idx, axis) in entry.variants.iter().enumerate() {
        let mut buttons = Vec::new();
        for (val_idx, val) in axis.values.iter().enumerate() {
            let selected = state
                .variant_selections
                .get(axis_idx)
                .copied()
                .unwrap_or(axis.default_index)
                == val_idx;

            let btn = el(El::Button)
                .st(if selected {
                    vec![
                        St::BgPrimary, St::TextOnPrimary, St::PxSm, St::PySm,
                        St::RoundedMd, St::TextXs, St::FontMedium, St::BorderNone,
                        St::CursorPointer,
                    ]
                } else {
                    vec![
                        St::BgSecondary, St::TextOnSecondary, St::PxSm, St::PySm,
                        St::RoundedMd, St::TextXs, St::FontMedium, St::BorderNone,
                        St::CursorPointer, St::TransitionColors,
                    ]
                })
                .hover(if selected { vec![] } else { vec![St::BgSecondaryHover] })
                .data("axis", &axis_idx.to_string())
                .data("val", &val_idx.to_string())
                .on(Ev::Click, select_variant())
                .text(val.label);

            buttons.push(btn);
        }

        let row = Stack::row()
            .gap(Gap::Xs)
            .align(StackAlign::Center)
            .children([
                el(El::Span)
                    .st([St::TextXs, St::TextMuted, St::FontMedium])
                    .attr("style", "min-width:64px")
                    .text(axis.display_name),
                Stack::row()
                    .gap(Gap::Xs)
                    .children(buttons)
                    .build(),
            ])
            .build();
        controls.push(row);
    }

    // Bool toggles
    if !entry.bool_props.is_empty() {
        let mut toggles = Vec::new();
        for (bool_idx, prop) in entry.bool_props.iter().enumerate() {
            let checked = state
                .bool_states
                .get(bool_idx)
                .copied()
                .unwrap_or(prop.default);

            // Use click event (not change) so data-* attributes are sent to the server.
            // The JS runtime only collects data-* on click events.
            let toggle = Checkbox::new()
                .label(prop.name)
                .checked(checked)
                .build()
                .data("idx", &bool_idx.to_string())
                .on(Ev::Click, toggle_bool_prop());

            toggles.push(toggle);
        }
        controls.push(
            Stack::row()
                .gap(Gap::Md)
                .children(toggles)
                .build(),
        );
    }

    // Build the live demo
    let variants: Vec<usize> = entry
        .variants
        .iter()
        .enumerate()
        .map(|(i, axis)| {
            state
                .variant_selections
                .get(i)
                .copied()
                .unwrap_or(axis.default_index)
        })
        .collect();
    let bools: Vec<bool> = entry
        .bool_props
        .iter()
        .enumerate()
        .map(|(i, prop)| {
            state
                .bool_states
                .get(i)
                .copied()
                .unwrap_or(prop.default)
        })
        .collect();

    let demo = (entry.build_demo)(&variants, &bools);

    // Assemble playground card
    let mut playground_children = Vec::new();

    if !controls.is_empty() {
        playground_children.push(
            el(El::Div)
                .st([St::DisplayFlex, St::FlexCol, St::GapSm, St::PbMd, St::BorderBDefault])
                .append(controls),
        );
    }

    playground_children.push(
        el(El::Div)
            .st([
                St::DisplayFlex,
                St::ItemsCenter,
                St::JustifyCenter,
                St::PMd,
                St::MinH6rem,
            ])
            .append([demo]),
    );

    Card::new()
        .children(playground_children)
        .build()
}

// ============================================================================
// Code Example
// ============================================================================

fn build_code_example(entry: &ComponentEntry, state: &DesignSystemState) -> ElementBuilder {
    let parts = [format!("{}::new()", entry.name)];
    let mut chain = Vec::new();

    for (i, axis) in entry.variants.iter().enumerate() {
        let selected = state
            .variant_selections
            .get(i)
            .copied()
            .unwrap_or(axis.default_index);
        if selected != axis.default_index {
            if let Some(val) = axis.values.get(selected) {
                chain.push(format!("    .{}({})", axis.name, val.rust_expr));
            }
        }
    }

    for (i, prop) in entry.bool_props.iter().enumerate() {
        let checked = state
            .bool_states
            .get(i)
            .copied()
            .unwrap_or(prop.default);
        if checked != prop.default {
            chain.push(format!("    .{}({})", prop.name, checked));
        }
    }

    let code = if chain.is_empty() {
        format!("{}\n    .build()", parts[0])
    } else {
        format!("{}\n{}\n    .build()", parts[0], chain.join("\n"))
    };

    Code::block(code).language("rust").build()
}

// ============================================================================
// Props Table
// ============================================================================

fn build_props_table(entry: &ComponentEntry) -> ElementBuilder {
    if entry.variants.is_empty() && entry.bool_props.is_empty() {
        return el(El::Div);
    }

    let mut table = Table::new().headers(["Prop", "Type", "Default"]);

    for axis in entry.variants {
        let default_val = axis
            .values
            .get(axis.default_index)
            .map(|v| v.label)
            .unwrap_or("—");
        table = table.row(
            TableRow::new()
                .cell(axis.name)
                .cell(axis.rust_type)
                .cell(default_val),
        );
    }

    for prop in entry.bool_props {
        table = table.row(
            TableRow::new()
                .cell(prop.name)
                .cell("bool")
                .cell(if prop.default { "true" } else { "false" }),
        );
    }

    Stack::column()
        .gap(Gap::Sm)
        .children([
            el(El::H2).st([St::TextXl, St::FontSemibold]).text("Props"),
            table.build(),
        ])
        .build()
}

// ============================================================================
// Markdown Documentation
// ============================================================================

fn build_markdown_docs(entry: &ComponentEntry) -> ElementBuilder {
    let docs_path = format!("{}/{}.md", docs_dir(), entry.slug);
    match std::fs::read_to_string(&docs_path) {
        Ok(content) => {
            // Strip frontmatter before rendering
            let markdown = if content.trim_start().starts_with("---") {
                if let Some(end) = content[3..].find("\n---") {
                    content[3 + end + 4..].trim_start().to_string()
                } else {
                    content
                }
            } else {
                content
            };
            Markdown::new(markdown).build()
        }
        Err(_) => el(El::Div),
    }
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
fn on_route_change(state: &mut DesignSystemState, ctx: &rwire::EventContext) {
    if let Some(path) = ctx.text() {
        if path == "/" {
            state.current_slug.clear();
        } else if let Some(slug) = path.strip_prefix("/components/") {
            state.current_slug = slug.to_string();
            // Reset playground state for new component
            if let Some(entry) = catalog::find(slug) {
                state.variant_selections = entry
                    .variants
                    .iter()
                    .map(|axis| axis.default_index)
                    .collect();
                state.bool_states = entry.bool_props.iter().map(|p| p.default).collect();
            }
        }
        state.sidebar_open = false;
    }
}

#[handler]
fn toggle_sidebar(state: &mut DesignSystemState) {
    state.sidebar_open = !state.sidebar_open;
}

#[handler]
fn select_variant(state: &mut DesignSystemState, ctx: &rwire::EventContext) {
    if let (Some(axis), Some(val)) = (ctx.data("axis"), ctx.data("val")) {
        if let (Ok(a), Ok(v)) = (axis.parse::<usize>(), val.parse::<usize>()) {
            // Ensure vec is large enough
            while state.variant_selections.len() <= a {
                state.variant_selections.push(0);
            }
            state.variant_selections[a] = v;
        }
    }
}

#[handler]
fn toggle_bool_prop(state: &mut DesignSystemState, ctx: &rwire::EventContext) {
    if let Some(idx) = ctx.data("idx") {
        if let Ok(i) = idx.parse::<usize>() {
            while state.bool_states.len() <= i {
                state.bool_states.push(false);
            }
            state.bool_states[i] = !state.bool_states[i];
        }
    }
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
    // Cycle: Indigo → Nord → default (no palette)
    let is_nord = theme.palette_ref().is_some_and(|p| p.accent.step(8) == "#5E81AC");
    let is_indigo = theme.palette_ref().is_some_and(|p| p.accent.step(8) == "#3730A3");
    if is_indigo {
        theme.set_palette(palettes::nord());
    } else if is_nord {
        theme.clear_palette();
    } else {
        theme.set_palette(palettes::indigo());
    }
}
