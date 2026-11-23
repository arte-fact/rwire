//! AppShell component.
//!
//! Full-page layout with header, optional sidebar, and main content area.
//!
//! # Three columns (sidebar, main, panel)
//!
//! With a panel, a mobile view, resize handles, a collapsed sidebar or its own
//! thresholds, the shell lays out three columns in a full-height row:
//!
//! ```ignore
//! AppShell::new()
//!     .sidebar(sidebar)                 // Sidebar::new()…build()
//!     .main(conversation)
//!     .panel(Some(terminal))            // None: no panel
//!     .sidebar_width(264)
//!     .panel_width(340)
//!     .collapsed(false)
//!     .resizable(on_resize())           // handler reading ctx.data("pane") / ctx.data("w")
//!     .mobile(MobilePane::Main)         // the view below the one-column threshold
//!     .build()
//! ```
//!
//! - Below 1100 px the panel becomes a drawer over the main column; below
//!   760 px (or the theme's `touch_below`, when set) one column shows, the one
//!   `mobile(…)` names, and the panel covers the screen. `.breakpoints(one,
//!   drawer)` sets the shell's own thresholds; the `sm` breakpoint does not
//!   move.
//! - Resize handles (`BIND_RESIZE`) drag a column's width client-side; on
//!   release the handler gets `pane` (`sidebar` / `panel`) and `w` (px; empty
//!   after a double-click, which restores the default width). A plain click on
//!   a handle sends nothing.
//! - Without a visible sidebar (none, or `collapsed`), a phone shows the main
//!   column whatever `mobile(…)` says.
//! - With `.collapsible_sidebar(true)` the visitor can fold the sidebar away:
//!   a native checkbox (visually hidden, still keyboard reachable) plus two
//!   labels — "fold" on the sidebar's top-right corner, "unfold" on the main
//!   column's top-left, shown only once folded. Pure CSS (`:has`), so folding
//!   costs no server round-trip and the shell keeps its own layout rules.
//! - **The drawer is the app's to manage.** Below the drawer threshold the
//!   panel overlays the main column; the app closes it by passing
//!   `panel(None)` (a close button in the panel, Escape) and moves the focus
//!   into it and back. The shell cannot mark it `aria-modal` or make the main
//!   column `inert`: whether the panel is a drawer depends on the viewport,
//!   which the server does not know, and `inert` on a desktop's main column
//!   would make it unusable.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::AppShell;
//!
//! AppShell::new()
//!     .header(header_content)
//!     .sidebar(sidebar_content)
//!     .main(main_content)
//!     .build()
//! ```

use rwire::style::Style;
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder, Ev, HandlerSpec};
use std::borrow::Cow;

/// Which column a one-column (phone) layout shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MobilePane {
    /// The sidebar column.
    #[default]
    Sidebar,
    /// The main column.
    Main,
}

/// AppShell builder.
#[derive(Clone, Default)]
pub struct AppShell {
    header: Option<ElementBuilder>,
    sidebar: Option<ElementBuilder>,
    main_content: Option<ElementBuilder>,
    sidebar_width: u16,
    header_height: u16,
    extra_class: Option<Cow<'static, str>>,
    panel: Option<ElementBuilder>,
    panel_width: Option<u16>,
    collapsed: bool,
    on_resize: Option<HandlerSpec>,
    mobile: Option<MobilePane>,
    breakpoints: Option<(u16, u16)>,
    three_columns: bool,
    no_landmarks: bool,
    collapsible: bool,
}

#[rwire::component]
impl AppShell {
    /// Create a new AppShell with default dimensions.
    pub fn new() -> Self {
        Self {
            sidebar_width: 264,
            header_height: 56,
            ..Self::default()
        }
    }

    /// Set the header content.
    pub fn header(mut self, content: ElementBuilder) -> Self {
        self.header = Some(content);
        self
    }

    /// Set the sidebar content.
    pub fn sidebar(mut self, content: ElementBuilder) -> Self {
        self.sidebar = Some(content);
        self
    }

    /// Set the main content.
    pub fn main(mut self, content: ElementBuilder) -> Self {
        self.main_content = Some(content);
        self
    }

    /// Set the sidebar width in pixels.
    pub fn sidebar_width(mut self, px: u16) -> Self {
        self.sidebar_width = px;
        self
    }

    /// Set the header height in pixels.
    pub fn header_height(mut self, px: u16) -> Self {
        self.header_height = px;
        self
    }

    /// A right-hand panel (three-column layout); `None` for none.
    pub fn panel(mut self, panel: Option<ElementBuilder>) -> Self {
        self.panel = panel;
        self.three_columns = true;
        self
    }

    /// The panel's width in pixels (340 by default).
    pub fn panel_width(mut self, px: u16) -> Self {
        self.panel_width = Some(px);
        self.three_columns = true;
        self
    }

    /// Hide the sidebar (the app keeps its own menu button to bring it back).
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self.three_columns = true;
        self
    }

    /// Resize handles on the sidebar's and the panel's edges. `handler` gets
    /// `ctx.data("pane")` (`sidebar` / `panel`) and `ctx.data("w")` (the width
    /// in px, or empty after a double-click: the default width).
    pub fn resizable(mut self, handler: HandlerSpec) -> Self {
        self.on_resize = Some(handler);
        self.three_columns = true;
        self
    }

    /// The column a one-column layout shows.
    pub fn mobile(mut self, pane: MobilePane) -> Self {
        self.mobile = Some(pane);
        self.three_columns = true;
        self
    }

    /// The shell's own thresholds: one column below `one_column` px, the panel
    /// as a drawer below `drawer` px (the theme's: `touch_below` or 760, and
    /// 1100).
    pub fn breakpoints(mut self, one_column: u16, drawer: u16) -> Self {
        self.breakpoints = Some((one_column, drawer));
        self.three_columns = true;
        self
    }

    /// Render the columns as landmarks (`<main>`, `<aside>`), the default. Turn
    /// it off for a shell embedded in a page that has its own `<main>` (a demo).
    pub fn landmarks(mut self, on: bool) -> Self {
        self.no_landmarks = !on;
        self.three_columns = true;
        self
    }

    /// Let the visitor fold the sidebar away (see the module docs). The shell
    /// renders the checkbox and both labels; the app's own sidebar content is
    /// untouched.
    pub fn collapsible_sidebar(mut self, on: bool) -> Self {
        self.collapsible = on;
        self.three_columns = true;
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for the shell container.
    pub fn compute_tokens() -> Vec<St> {
        vec![St::GridTemplateShell]
    }

    /// Compute style tokens for the header.
    pub fn compute_header_tokens() -> Vec<St> {
        vec![
            St::GridColFull,
            St::PositionSticky,
            St::Top0,
            St::Z50,
            St::BgApp,
            St::BorderBDefault,
            St::DisplayFlex,
            St::ItemsCenter,
            St::PxMd,
        ]
    }

    /// Compute style tokens for the sidebar.
    pub fn compute_sidebar_tokens() -> Vec<St> {
        vec![
            St::BgSidebar,
            St::BorderRDefault,
            St::OverflowYScroll,
            St::PositionSticky,
            St::PyMd,
        ]
    }

    /// Compute style tokens for the main area.
    pub fn compute_main_tokens() -> Vec<St> {
        vec![St::OverflowYScroll, St::PMd, St::Flex1]
    }

    /// Build the app shell into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        if self.three_columns {
            return self.build_three_columns();
        }
        let has_sidebar = self.sidebar.is_some();
        let grid_cols = if has_sidebar {
            format!("grid-template-columns:{}px 1fr", self.sidebar_width)
        } else {
            "grid-template-columns:1fr".to_string()
        };

        let mut shell = el(El::Div)
            .st(Self::compute_tokens())
            .attr("style", &grid_cols);

        if let Some(ref extra) = self.extra_class {
            shell = shell.class(extra.as_ref());
        }

        // Header
        if let Some(header) = self.header {
            let header_style = format!("height:{}px", self.header_height);
            let header_el = el(El::Header)
                .st(Self::compute_header_tokens())
                .attr("style", &header_style)
                .append([header]);
            shell = shell.append([header_el]);
        }

        // Sidebar
        if let Some(sidebar) = self.sidebar {
            let sidebar_style = format!(
                "top:{}px;height:calc(100vh - {}px)",
                self.header_height, self.header_height
            );
            let sidebar_el = el(El::Aside)
                .st(Self::compute_sidebar_tokens())
                .attr("style", &sidebar_style)
                .append([sidebar]);
            shell = shell.append([sidebar_el]);
        }

        // Main content
        if let Some(content) = self.main_content {
            let main_el = el(El::Main)
                .st(Self::compute_main_tokens())
                .append([content]);
            shell = shell.append([main_el]);
        }

        shell
    }
}

impl AppShell {
    /// A resize handle for `pane`, on the edge of the column before it
    /// (`sidebar`) or after it (`panel`, `data-rz="n"`).
    fn handle(pane: &str, handler: &HandlerSpec) -> ElementBuilder {
        let mut h = el(El::Div)
            .attr("data-shell", "handle")
            .attr("data-pane", pane)
            .attr("role", "separator")
            .attr("aria-orientation", "vertical")
            .attr(
                "aria-label",
                if pane == "panel" {
                    "Resize the panel"
                } else {
                    "Resize the sidebar"
                },
            )
            .st([St::ShellHandle])
            .hover([St::ShellHandleLine])
            .resize_handle()
            .on(Ev::Click, handler.clone());
        if pane == "panel" {
            h = h.attr("data-rz", "n");
        }
        h
    }

    /// A fold/unfold label for [`Self::collapsible_sidebar`]: same checkbox,
    /// absolutely placed in the column's top corner.
    fn fold_label(marker: &'static str, title: &'static str, fold: bool) -> ElementBuilder {
        let corner = if fold {
            ["right", "0.4rem"]
        } else {
            ["left", "0.4rem"]
        };
        el(El::Label)
            .attr("for", "rw-side-toggle")
            .attr("data-shell", marker)
            .attr("title", title)
            .attr("aria-label", title)
            .attr("role", "button")
            .attr("tabindex", "0")
            .st([
                St::PositionAbsolute,
                St::Z50,
                St::DisplayInlineFlex,
                St::ItemsCenter,
                St::JustifyCenter,
                St::RoundedSm,
                St::TextMuted,
                St::CursorPointer,
                St::BgApp,
                St::BorderDefault,
            ])
            .style(
                Style::new()
                    .set("top", "0.35rem")
                    .set(corner[0], corner[1])
                    .set("width", "1.6rem")
                    .set("height", "1.6rem"),
            )
            .append([rwire::icon(if fold {
                rwire::Icon::SidebarCollapse
            } else {
                rwire::Icon::PanelRight
            })])
    }

    /// The three-column layout (see the module docs).
    fn build_three_columns(self) -> ElementBuilder {
        let mut vars = format!("--Qsw:{}px", self.sidebar_width);
        if let Some(pw) = self.panel_width {
            vars.push_str(&format!(";--Qpw:{pw}px"));
        }
        // Without a visible sidebar, a phone shows the main column whatever the
        // app asked: the list would be an empty screen.
        let sidebar_shown = self.sidebar.is_some() && !self.collapsed;
        let pane = if sidebar_shown {
            self.mobile.unwrap_or_default()
        } else {
            MobilePane::Main
        };
        let (main_el, aside_el) = if self.no_landmarks {
            (El::Div, El::Div)
        } else {
            (El::Main, El::Aside)
        };
        let mut root = el(El::Div)
            .attr("data-shell-root", "")
            .attr(
                "data-mobile",
                match pane {
                    // Wire value stays `list`: neutral, and existing markup selectors use it.
                    MobilePane::Sidebar => "list",
                    MobilePane::Main => "main",
                },
            )
            .attr("style", &vars)
            .st([
                St::DisplayFlex,
                St::FlexCol,
                St::HDvh,
                St::MinH0,
                St::OverflowHidden,
                St::BgApp,
            ]);
        if let Some(ref extra) = self.extra_class {
            root = root.class(extra.as_ref());
        }
        if self.collapsible {
            // Fold control, pure CSS: the checkbox lives on the root, the two
            // labels flip it, and `:has` hides the column, its handle and the
            // fold label, while revealing the unfold one.
            root = root.append([
                el(El::Input)
                    .attr("type", "checkbox")
                    .attr("id", "rw-side-toggle")
                    .attr("data-shell", "side-toggle")
                    .attr("aria-label", "Replier la liste des sessions")
                    .st([St::SrOnly]),
                el(El::Style).text(
                    "[data-shell-root]:has(> [data-shell=side-toggle]:checked)>div>[data-shell=sidebar],\
                     [data-shell-root]:has(> [data-shell=side-toggle]:checked)>div>[data-shell=handle][data-pane=sidebar],\
                     [data-shell-root]:has(> [data-shell=side-toggle]:checked) [data-shell=side-collapse]{display:none}\
                     [data-shell-root] [data-shell=side-expand]{display:none}\
                     [data-shell-root]:has(> [data-shell=side-toggle]:checked) [data-shell=side-expand]{display:inline-flex}",
                ),
            ]);
        }
        if let Some((one, drawer)) = self.breakpoints {
            // One column never starts above the drawer threshold.
            let one = one.min(drawer);
            let key = format!("{one}-{drawer}");
            let scope = format!("[data-shell-root][data-shell-bp=\"{key}\"]");
            root = root
                .attr("data-shell-bp", &key)
                .append([el(El::Style).text(&rwire::theme::shell_layout_css(&scope, one, drawer))]);
        }
        if let Some(header) = self.header {
            root = root.append([el(El::Header)
                .st([
                    St::BgApp,
                    St::BorderBDefault,
                    St::DisplayFlex,
                    St::ItemsCenter,
                    St::PxMd,
                ])
                .attr("style", &format!("height:{}px", self.header_height))
                .append([header])]);
        }
        let mut row = vec![];
        if let (Some(sidebar), false) = (self.sidebar, self.collapsed) {
            let mut column = el(El::Div)
                .attr("data-shell", "sidebar")
                // The column draws no edge: its content does (`Sidebar`).
                .st([St::WShellSidebar, St::MinH0, St::DisplayFlex, St::FlexCol]);
            if self.collapsible {
                column = column.st([St::PositionRelative]).append([Self::fold_label(
                    "side-collapse",
                    "Replier la liste",
                    true,
                )]);
            }
            row.push(column.append([sidebar]));
            if let Some(ref h) = self.on_resize {
                row.push(Self::handle("sidebar", h));
            }
        }
        let mut main_column = el(main_el).attr("data-shell", "main").st([
            St::Flex1,
            St::MinW0,
            St::MinH0,
            St::OverflowYAuto,
        ]);
        if self.collapsible {
            main_column = main_column.st([St::PositionRelative]).append([
                // The unfold control sits above the main column while the list is
                // folded; the app leaves that corner free (its top bar is padded).
                Self::fold_label("side-expand", "Déplier la liste", false),
            ]);
        }
        row.push(main_column.append(self.main_content));
        if let Some(panel) = self.panel {
            if let Some(ref h) = self.on_resize {
                row.push(Self::handle("panel", h));
            }
            row.push(
                el(aside_el)
                    .attr("data-shell", "panel")
                    .attr("aria-label", "Panel")
                    .st([
                        St::WShellPanel,
                        St::MinH0,
                        St::OverflowYAuto,
                        St::BorderL,
                        St::BgApp,
                    ])
                    .append([panel]),
            );
        }
        root.append([el(El::Div)
            .st([St::DisplayFlex, St::Flex1, St::MinH0])
            .append(row)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_shell_defaults() {
        let shell = AppShell::new();
        assert_eq!(shell.sidebar_width, 264);
        assert_eq!(shell.header_height, 56);
        assert!(shell.header.is_none());
        assert!(shell.sidebar.is_none());
        assert!(shell.main_content.is_none());
    }

    #[test]
    fn test_app_shell_custom_dimensions() {
        let shell = AppShell::new().sidebar_width(280).header_height(64);
        assert_eq!(shell.sidebar_width, 280);
        assert_eq!(shell.header_height, 64);
    }

    #[test]
    fn a_collapsible_shell_ships_the_checkbox_the_two_labels_and_the_rules() {
        let html = AppShell::new()
            .sidebar(el(El::Div).text("liste"))
            .main(el(El::Div).text("fil"))
            .collapsible_sidebar(true)
            .build()
            .to_static_html();
        assert!(html.contains("data-shell=\"side-toggle\""), "{html}");
        assert!(html.contains("id=\"rw-side-toggle\""), "{html}");
        assert!(html.contains("data-shell=\"side-collapse\""), "{html}");
        assert!(html.contains("data-shell=\"side-expand\""), "{html}");
        assert!(
            html.contains(":has(> [data-shell=side-toggle]:checked)"),
            "{html}"
        );
        assert!(
            html.contains("for=\"rw-side-toggle\""),
            "les deux libellés visent la même case : {html}"
        );
    }

    #[test]
    fn a_plain_shell_has_no_fold_control() {
        let html = AppShell::new()
            .sidebar(el(El::Div).text("liste"))
            .main(el(El::Div).text("fil"))
            .build()
            .to_static_html();
        assert!(!html.contains("side-toggle"), "{html}");
    }

    #[test]
    fn test_shell_container_tokens() {
        let tokens = AppShell::compute_tokens();
        assert!(tokens.contains(&St::GridTemplateShell));
    }

    #[test]
    fn test_shell_header_tokens() {
        let tokens = AppShell::compute_header_tokens();
        assert!(tokens.contains(&St::GridColFull));
        assert!(tokens.contains(&St::PositionSticky));
        assert!(tokens.contains(&St::Top0));
        assert!(tokens.contains(&St::Z50));
        assert!(tokens.contains(&St::BgApp));
        assert!(tokens.contains(&St::BorderBDefault));
    }

    #[test]
    fn test_shell_sidebar_tokens() {
        let tokens = AppShell::compute_sidebar_tokens();
        assert!(tokens.contains(&St::BgSidebar));
        assert!(tokens.contains(&St::BorderRDefault));
        assert!(tokens.contains(&St::OverflowYScroll));
        assert!(tokens.contains(&St::PositionSticky));
    }

    fn handler() -> HandlerSpec {
        #[derive(Default)]
        struct S;
        impl rwire::MemoryState for S {}
        fn noop(_: &mut S) {}
        HandlerSpec::memory::<S>(noop)
    }

    #[test]
    fn the_legacy_shell_is_unchanged_without_the_new_options() {
        let html = AppShell::new()
            .header(el(El::Span))
            .sidebar(el(El::Span))
            .main(el(El::Span))
            .build()
            .to_static_html();
        assert!(html.contains("grid-template-columns:264px 1fr"), "{html}");
        assert!(!html.contains("data-shell"), "{html}");
    }

    #[test]
    fn three_columns_with_handles_widths_and_a_mobile_view() {
        let html = AppShell::new()
            .sidebar(el(El::Span).text("list"))
            .main(el(El::Span).text("thread"))
            .panel(Some(el(El::Span).text("terminal")))
            .sidebar_width(264)
            .panel_width(340)
            .resizable(handler())
            .mobile(MobilePane::Main)
            .build()
            .to_static_html();
        assert!(html.contains("data-shell-root"), "{html}");
        assert!(html.contains("data-mobile=\"main\""), "{html}");
        assert!(html.contains("--Qsw:264px;--Qpw:340px"), "{html}");
        let order = [
            "data-shell=\"sidebar\"",
            "data-pane=\"sidebar\"",
            "data-shell=\"main\"",
            "data-pane=\"panel\"",
            "data-shell=\"panel\"",
        ]
        .map(|s| html.find(s).unwrap_or_else(|| panic!("{s}: {html}")));
        assert!(order.windows(2).all(|w| w[0] < w[1]), "{html}");
        assert!(
            html.contains("data-rz=\"n\""),
            "the panel handle sizes the next column"
        );
        assert_eq!(html.matches("role=\"separator\"").count(), 2);
    }

    #[test]
    fn collapsed_drops_the_sidebar_and_its_handle() {
        let html = AppShell::new()
            .sidebar(el(El::Span).text("list"))
            .main(el(El::Span))
            .collapsed(true)
            .resizable(handler())
            .build()
            .to_static_html();
        assert!(!html.contains("data-shell=\"sidebar\""), "{html}");
        assert!(!html.contains("data-pane"), "{html}");
        assert!(html.contains("data-shell=\"main\""), "{html}");
    }

    #[test]
    fn the_theme_lays_out_shells_at_its_thresholds_and_a_shell_at_its_own() {
        use rwire::theme::{generate_theme_css, Theme};
        let css = generate_theme_css(&Theme::new());
        assert!(css.contains("@media (max-width:1099.98px){[data-shell-root]:not([data-shell-bp])>div>[data-shell=panel]{position:fixed;"), "{css}");
        assert!(css.contains("@media (max-width:759.98px){[data-shell-root]:not([data-shell-bp])>div>[data-shell=handle]{display:none}"), "{css}");
        assert!(css.contains("[data-shell-root]:not([data-shell-bp])[data-mobile=main]>div>[data-shell=sidebar]{display:none}"));
        // The one-column threshold follows touch_below.
        let phone = generate_theme_css(&Theme::new().touch_below(800));
        assert!(
            phone.contains("@media (max-width:799.98px){[data-shell-root]:not([data-shell-bp])"),
            "{phone}"
        );
        // A shell with its own thresholds carries its rules.
        let html = AppShell::new()
            .main(el(El::Span))
            .breakpoints(700, 1200)
            .build()
            .to_static_html();
        assert!(html.contains("data-shell-bp=\"700-1200\""), "{html}");
        assert!(html.contains("@media (max-width:1199.98px){[data-shell-root][data-shell-bp=\"700-1200\"]>div>[data-shell=panel]"), "{html}");
    }

    #[test]
    fn without_a_visible_sidebar_a_phone_shows_main() {
        // No sidebar at all.
        let html = AppShell::new()
            .main(el(El::Span))
            .mobile(MobilePane::Sidebar)
            .build()
            .to_static_html();
        assert!(html.contains("data-mobile=\"main\""), "{html}");
        // A collapsed sidebar.
        let html = AppShell::new()
            .sidebar(el(El::Span))
            .main(el(El::Span))
            .collapsed(true)
            .build()
            .to_static_html();
        assert!(html.contains("data-mobile=\"main\""), "{html}");
        // A visible sidebar keeps the app's choice (List by default).
        let html = AppShell::new()
            .sidebar(el(El::Span))
            .main(el(El::Span))
            .resizable(handler())
            .build()
            .to_static_html();
        assert!(html.contains("data-mobile=\"list\""), "{html}");
    }

    #[test]
    fn inverted_thresholds_are_clamped_to_the_drawer() {
        let html = AppShell::new()
            .main(el(El::Span))
            .breakpoints(1200, 1100)
            .build()
            .to_static_html();
        assert!(!html.contains("1199.98"), "{html}");
        assert_eq!(
            html.matches("@media (max-width:1099.98px)").count(),
            2,
            "{html}"
        );
        let css = rwire::theme::generate_theme_css(&rwire::theme::Theme::new().touch_below(1200));
        assert!(
            !css.contains("(max-width:1199.98px){[data-shell-root]"),
            "{css}"
        );
    }

    #[test]
    fn rules_reach_a_shells_own_columns_only() {
        let css = rwire::theme::shell_layout_css("S", 760, 1100);
        assert!(
            !css.contains(" [data-shell="),
            "descendant selectors: {css}"
        );
        assert!(css.contains("S>div>[data-shell=panel]"), "{css}");
        assert!(
            css.contains("S[data-mobile=list]>div>[data-shell=main]"),
            "{css}"
        );
        // A shell inside another's main column: the outer rules stop at its row.
        let inner = AppShell::new()
            .main(el(El::Span))
            .panel(Some(el(El::Span)))
            .build();
        let outer = AppShell::new()
            .main(inner)
            .panel(Some(el(El::Span)))
            .build();
        let html = outer.to_static_html();
        assert_eq!(html.matches("data-shell-root").count(), 2, "{html}");
    }

    #[test]
    fn the_panel_is_a_labelled_landmark_and_landmarks_can_be_turned_off() {
        let html = AppShell::new()
            .main(el(El::Span))
            .panel(Some(el(El::Span)))
            .build()
            .to_static_html();
        assert!(html.contains("<aside "), "{html}");
        assert!(html.contains("aria-label=\"Panel\""), "{html}");
        // Embedded (a gallery demo inside a page's own <main>): no landmarks.
        let html = AppShell::new()
            .main(el(El::Span))
            .panel(Some(el(El::Span)))
            .landmarks(false)
            .build()
            .to_static_html();
        assert!(
            !html.contains("<main") && !html.contains("<aside"),
            "{html}"
        );
    }

    #[test]
    fn the_default_sidebar_width_is_264() {
        assert_eq!(AppShell::new().sidebar_width, 264);
    }

    /// The sidebar column leaves the edge to its content (`Sidebar` draws it):
    /// no double hairline.
    #[test]
    fn the_sidebar_column_draws_no_edge_of_its_own() {
        let built = AppShell::new()
            .sidebar(el(El::Span))
            .main(el(El::Span))
            .panel(None)
            .build();
        let html = built.to_static_html();
        let col = html.split("data-shell=\"sidebar\"").next().unwrap();
        let class = col.rsplit("class=\"").next().unwrap();
        let border = St::BorderRDefault.as_u16();
        assert!(
            !class
                .split('"')
                .next()
                .unwrap()
                .split(' ')
                .any(|c| c == format!("u{border}")),
            "{html}"
        );
    }

    #[test]
    fn test_shell_main_tokens() {
        let tokens = AppShell::compute_main_tokens();
        assert!(tokens.contains(&St::OverflowYScroll));
        assert!(tokens.contains(&St::PMd));
        assert!(tokens.contains(&St::Flex1));
    }
}
