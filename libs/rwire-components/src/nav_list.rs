//! Dense, grouped navigation: `NavList` → `NavGroup` → `NavRow`.
//!
//! A sidebar list of groups (machines) holding two-line rows (sessions): a
//! foldable group header in spaced capitals with a chevron, a count and a
//! badge; rows on a `14px | 1fr | auto` grid with a leading status dot, a
//! title (strong part and muted suffix), a trailing badge and a meta line.
//! The current row is tinted and marked `aria-current="page"`; a folded group
//! renders its header only, `aria-expanded="false"`. Sizes follow the theme's
//! density (`St::PadRow`).
//!
//! ```ignore
//! NavList::new()
//!     .aria_label("Sessions")
//!     .group(
//!         NavGroup::new("home")
//!             .count(3)
//!             .leading(StatusDot::new().intent(StatusDotIntent::Success).build())
//!             .on_toggle(toggle_machine())
//!             .rows([
//!                 NavRow::new("/s/abc")
//!                     .leading(StatusDot::new().intent(StatusDotIntent::Success).ring(true).build())
//!                     .title("mcp-agents", "@home")
//!                     .trailing(runtime_badge)
//!                     .meta("Refactoring the hub…", "a1b2")
//!                     .current(true),
//!             ]),
//!     )
//!     .build()
//! ```

use rwire::router::Link;
use rwire::style_tokens::St;
use rwire::{el, icons, El, ElementBuilder, Ev, HandlerSpec, Icon};
use std::borrow::Cow;

/// A list of navigation groups (`<nav>`).
#[derive(Clone, Default)]
pub struct NavList {
    id: Option<Cow<'static, str>>,
    groups: Vec<NavGroup>,
    aria_label: Option<Cow<'static, str>>,
}

impl NavList {
    /// An empty list.
    pub fn new() -> Self {
        Self::default()
    }

    /// The prefix of the groups' ids (`<prefix>-g0`, `-g1`…; default `nav`),
    /// so two lists on a page, or two groups with the same label, never share one.
    pub fn id(mut self, prefix: impl Into<Cow<'static, str>>) -> Self {
        self.id = Some(prefix.into());
        self
    }

    /// Add a group.
    pub fn group(mut self, group: NavGroup) -> Self {
        self.groups.push(group);
        self
    }

    /// Add groups.
    pub fn groups(mut self, groups: impl IntoIterator<Item = NavGroup>) -> Self {
        self.groups.extend(groups);
        self
    }

    /// The navigation's accessible name (`aria-label`).
    pub fn aria_label(mut self, label: impl Into<Cow<'static, str>>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Build the `<nav>`.
    pub fn build(self) -> ElementBuilder {
        let mut nav = el(El::Nav).st([St::DisplayFlex, St::FlexCol, St::GapSm]);
        if let Some(label) = &self.aria_label {
            nav = nav.attr("aria-label", label);
        }
        let prefix = self.id.unwrap_or(Cow::Borrowed("nav"));
        nav.append(self.groups.into_iter().enumerate().map(|(i, group)| {
            let group = if group.id.is_none() {
                group.id(format!("{prefix}-g{i}"))
            } else {
                group
            };
            group.build()
        }))
    }
}

/// A foldable group of rows, under a header in spaced capitals.
#[derive(Clone)]
pub struct NavGroup {
    label: Cow<'static, str>,
    id: Option<Cow<'static, str>>,
    count: Option<u32>,
    badge: Option<ElementBuilder>,
    leading: Option<ElementBuilder>,
    folded: bool,
    on_toggle: Option<HandlerSpec>,
    rows: Vec<NavRow>,
}

impl NavGroup {
    /// A group named `label` (shown in capitals).
    pub fn new(label: impl Into<Cow<'static, str>>) -> Self {
        Self {
            label: label.into(),
            id: None,
            count: None,
            badge: None,
            leading: None,
            folded: false,
            on_toggle: None,
            rows: Vec::new(),
        }
    }

    /// The id of the rows' container, which the header's `aria-controls`
    /// names (default: `nav-` and the label's letters and digits).
    pub fn id(mut self, id: impl Into<Cow<'static, str>>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// A count, at the header's end (tabular figures, faint ink).
    pub fn count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }

    /// A badge after the label (an attention count, say).
    pub fn badge(mut self, badge: ElementBuilder) -> Self {
        self.badge = Some(badge);
        self
    }

    /// An element before the label (a `StatusDot`: online, offline).
    pub fn leading(mut self, leading: ElementBuilder) -> Self {
        self.leading = Some(leading);
        self
    }

    /// Fold the group: the header only, `aria-expanded="false"`.
    pub fn folded(mut self, folded: bool) -> Self {
        self.folded = folded;
        self
    }

    /// The header's click handler (the app flips its `folded` state).
    pub fn on_toggle(mut self, handler: HandlerSpec) -> Self {
        self.on_toggle = Some(handler);
        self
    }

    /// Add a row.
    pub fn row(mut self, row: NavRow) -> Self {
        self.rows.push(row);
        self
    }

    /// Add rows.
    pub fn rows(mut self, rows: impl IntoIterator<Item = NavRow>) -> Self {
        self.rows.extend(rows);
        self
    }

    fn rows_id(&self) -> String {
        match &self.id {
            Some(id) => id.to_string(),
            None => {
                let slug: String = self
                    .label
                    .chars()
                    .map(|c| {
                        if c.is_ascii_alphanumeric() {
                            c.to_ascii_lowercase()
                        } else {
                            '-'
                        }
                    })
                    .collect();
                format!("nav-{slug}")
            }
        }
    }

    /// Build the group: a header button, then (unfolded) its rows.
    pub fn build(self) -> ElementBuilder {
        let rows_id = self.rows_id();
        let mut chevron = icons::icon_sized(Icon::ChevronDown, 12).st([St::ChevronFold]);
        if self.folded {
            chevron = chevron.st([St::RotateNeg90]);
        }
        let mut head_children = vec![chevron.attr("aria-hidden", "true")];
        if let Some(leading) = self.leading {
            head_children.push(leading);
        }
        head_children.push(
            el(El::Span)
                .st([St::Truncate, St::MinW0])
                .text(self.label.as_ref()),
        );
        if let Some(badge) = self.badge {
            head_children.push(badge);
        }
        if let Some(count) = self.count {
            head_children.push(
                el(El::Span)
                    .st([
                        St::MlAuto,
                        St::TextFaint,
                        St::TabularNums,
                        St::FontMedium,
                        St::TrackingNormal,
                    ])
                    .text(&count.to_string()),
            );
        }
        let head_tokens = [
            St::DisplayFlex,
            St::ItemsCenter,
            St::GapSm,
            St::WFull,
            St::PxSm,
            St::PyXs,
            St::TextMuted,
            St::NavGroupHead,
        ];
        // A header that folds is a button naming its rows (when shown); one
        // that does not is a plain title.
        let head = match self.on_toggle {
            Some(handler) => {
                let mut head = el(El::Button)
                    .attr("type", "button")
                    .attr("aria-expanded", if self.folded { "false" } else { "true" })
                    .st(head_tokens)
                    .st([
                        St::BgTransparent,
                        St::BorderNone,
                        St::RoundedMd,
                        St::CursorPointer,
                    ])
                    .hover([St::BgSurfaceRaised])
                    .focus_visible([St::RingFocus])
                    .on(Ev::Click, handler)
                    .append(head_children);
                if !self.folded {
                    head = head.attr("aria-controls", &rows_id);
                }
                head
            }
            None => el(El::Div)
                .st(head_tokens)
                .append(head_children.into_iter().skip(1)),
        };

        let mut group = el(El::Div)
            .st([St::DisplayFlex, St::FlexCol, St::GapXs])
            .append([head]);
        if !self.folded {
            group = group.append([el(El::Div)
                .attr("id", &rows_id)
                .attr("role", "group")
                .attr("aria-label", self.label.as_ref())
                .st([St::DisplayFlex, St::FlexCol])
                .append(self.rows.into_iter().map(NavRow::build))]);
        }
        group
    }
}

/// A two-line navigation row: a client-routed link.
#[derive(Clone)]
pub struct NavRow {
    href: Cow<'static, str>,
    leading: Option<ElementBuilder>,
    title: Cow<'static, str>,
    title_suffix: Option<Cow<'static, str>>,
    trailing: Option<ElementBuilder>,
    meta: Option<(Cow<'static, str>, Option<Cow<'static, str>>)>,
    current: bool,
    attention: bool,
}

impl NavRow {
    /// A row linking to `href` (client-side routing).
    pub fn new(href: impl Into<Cow<'static, str>>) -> Self {
        Self {
            href: href.into(),
            leading: None,
            title: Cow::Borrowed(""),
            title_suffix: None,
            trailing: None,
            meta: None,
            current: false,
            attention: false,
        }
    }

    /// The 14 px leading cell (a `StatusDot`).
    pub fn leading(mut self, leading: ElementBuilder) -> Self {
        self.leading = Some(leading);
        self
    }

    /// The title: a strong part and a muted suffix (`"project"`, `"@machine"`).
    pub fn title(
        mut self,
        strong: impl Into<Cow<'static, str>>,
        muted_suffix: impl Into<Cow<'static, str>>,
    ) -> Self {
        self.title = strong.into();
        let suffix = muted_suffix.into();
        self.title_suffix = (!suffix.is_empty()).then_some(suffix);
        self
    }

    /// The trailing cell (a badge).
    pub fn trailing(mut self, trailing: ElementBuilder) -> Self {
        self.trailing = Some(trailing);
        self
    }

    /// The meta line under the title: a text (ellipsed) and a mono suffix.
    pub fn meta(
        mut self,
        text: impl Into<Cow<'static, str>>,
        mono_suffix: impl Into<Cow<'static, str>>,
    ) -> Self {
        let suffix = mono_suffix.into();
        self.meta = Some((text.into(), (!suffix.is_empty()).then_some(suffix)));
        self
    }

    /// The current row: tinted, an accent hairline, `aria-current="page"`.
    pub fn current(mut self, current: bool) -> Self {
        self.current = current;
        self
    }

    /// Needs attention: the title in bold.
    pub fn attention(mut self, attention: bool) -> Self {
        self.attention = attention;
        self
    }

    /// The row's own tokens.
    pub fn compute_tokens(&self) -> Vec<St> {
        let mut tokens = vec![
            St::GridNavRow,
            St::PadRow,
            St::RoundedNavRow,
            St::WFull,
            St::TextDefault,
            St::NoDecoration,
            St::CursorPointer,
        ];
        if self.current {
            tokens.extend([St::BgAccentSubtle, St::NavCurrent]);
        }
        tokens
    }

    /// Build the row.
    pub fn build(self) -> ElementBuilder {
        // `data-touch`: 44 px tall under a finger or below `touch_below`, like a
        // button (the theme's touch rule, at zero specificity).
        let mut row = Link::route(&self.href)
            .attr("data-touch", "")
            .st(self.compute_tokens());
        if !self.current {
            row = row.hover([St::BgSurfaceRaised]);
        }
        row = row.focus_visible([St::RingFocus]);
        if self.current {
            row = row.attr("aria-current", "page");
        }
        let leading = el(El::Span)
            .st([St::DisplayFlex, St::ItemsCenter, St::JustifyCenter])
            .append(self.leading);
        let mut title = vec![el(El::B)
            .st([if self.attention {
                St::FontBold
            } else {
                St::FontSemibold
            }])
            .text(self.title.as_ref())];
        if let Some(suffix) = &self.title_suffix {
            title.push(el(El::Span).st([St::TextFaint]).text(suffix.as_ref()));
        }
        let mut cells = vec![
            leading,
            el(El::Span)
                .st([St::Truncate, St::MinW0, St::TextNavTitle, St::LeadingNormal])
                .append(title),
            // A flex cell: the badge adds no line strut, so the row is as tall
            // as its title's line.
            el(El::Span)
                .st([St::DisplayFlex, St::ItemsCenter])
                .append(self.trailing),
        ];
        if let Some((text, suffix)) = &self.meta {
            let mut meta = vec![el(El::Span)
                .st([St::Truncate, St::MinW0, St::Flex1])
                .text(text.as_ref())];
            if let Some(suffix) = suffix {
                meta.push(
                    el(El::Span)
                        .st([
                            St::FontMono,
                            St::TextNavHash,
                            St::Opacity75,
                            St::FlexShrink0,
                        ])
                        .text(suffix.as_ref()),
                );
            }
            cells.push(
                el(El::Span)
                    .st([
                        St::ColNavMeta,
                        St::DisplayFlex,
                        St::ItemsBaseline,
                        St::GapXs,
                        St::MinW0,
                        St::TextNavMeta,
                        St::TextFaint,
                    ])
                    .append(meta),
            );
        }
        row.append(cells)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StatusDot;

    fn toggle() -> HandlerSpec {
        #[derive(Default)]
        struct S;
        impl rwire::MemoryState for S {}
        fn noop(_: &mut S) {}
        HandlerSpec::memory::<S>(noop)
    }

    fn row() -> NavRow {
        NavRow::new("/s/abc")
            .leading(StatusDot::new().build())
            .title("mcp-agents", "@home")
            .meta("Refactoring the hub", "a1b2")
    }

    #[test]
    fn a_row_is_a_routed_link_on_the_dense_grid() {
        let html = row().build().to_static_html();
        assert!(html.starts_with("<a "), "{html}");
        assert!(html.contains("href=\"/s/abc\""), "{html}");
        assert!(html.contains("data-route"), "{html}");
        assert!(html.contains("<b "), "{html}");
        assert!(html.contains(">@home<"), "{html}");
        assert!(html.contains(">a1b2<"), "{html}");
        assert!(!html.contains("aria-current"), "{html}");
        let tokens = row().compute_tokens();
        for st in [St::GridNavRow, St::PadRow, St::RoundedNavRow] {
            assert!(tokens.contains(&st), "{st:?}");
        }
    }

    /// Under a finger (or below `touch_below`) a one-line row gets the 44 px
    /// touch minimum, like a button; at zero specificity, so a two-line row
    /// (taller already) and a desktop row with a fine pointer are unchanged.
    #[test]
    fn a_row_takes_the_touch_minimum_like_a_button() {
        let html = NavRow::new("/s/x")
            .title("one", "")
            .build()
            .to_static_html();
        assert!(html.contains("data-touch"), "{html}");
        for theme in [
            rwire::theme::Theme::new(),
            rwire::theme::Theme::new().touch_below(760),
        ] {
            let css = rwire::theme::generate_theme_css(&theme);
            let block = css.split_once("@media (pointer:coarse)").unwrap().1;
            let rule = block.split("{min-height:44px}").next().unwrap();
            assert!(rule.contains("[data-touch]"), "{rule}");
            assert!(rule.contains(":where("), "zero specificity: {rule}");
        }
    }

    /// Nothing in a one-line row is taller than its title's line (1.5): the
    /// trailing badge sits in a flex cell (no line strut), the title sets its
    /// line height; the compact padding is the old front's (.38rem .55rem at
    /// its 16 px root).
    #[test]
    fn a_one_line_row_is_as_tall_as_its_title_line() {
        let built = NavRow::new("/s/x")
            .title("one", "")
            .trailing(el(El::Span).text("claude"))
            .build();
        let cells = built.children();
        let trailing = cells[2].get_style_utils();
        assert!(trailing.contains(&St::DisplayFlex.as_u16()), "{trailing:?}");
        assert!(trailing.contains(&St::ItemsCenter.as_u16()));
        assert!(cells[1]
            .get_style_utils()
            .contains(&St::LeadingNormal.as_u16()));
        let css = rwire::theme::generate_theme_css(
            &rwire::theme::Theme::new().density(rwire::theme::Density::Compact),
        );
        assert!(css.contains("--Qzp:6.08px 8.8px;"), "{css}");
    }

    #[test]
    fn the_current_row_is_marked_and_tinted() {
        let built = row().current(true).build();
        let html = built.to_static_html();
        assert!(html.contains("aria-current=\"page\""), "{html}");
        let utils = built.get_style_utils();
        assert!(utils.contains(&St::BgAccentSubtle.as_u16()));
        assert!(utils.contains(&St::NavCurrent.as_u16()));
    }

    #[test]
    fn attention_makes_the_title_bold() {
        let built = row().attention(true).build();
        let title = &built.children()[1].children()[0];
        assert!(title.get_style_utils().contains(&St::FontBold.as_u16()));
    }

    #[test]
    fn a_group_header_controls_its_rows_and_folds() {
        let open = NavGroup::new("Home Lab")
            .on_toggle(toggle())
            .count(3)
            .rows([row(), row()])
            .build();
        let html = open.to_static_html();
        assert!(html.contains("aria-expanded=\"true\""), "{html}");
        assert!(html.contains("aria-controls=\"nav-home-lab\""), "{html}");
        assert!(html.contains("id=\"nav-home-lab\""), "{html}");
        assert!(html.contains("role=\"group\""), "{html}");
        assert_eq!(html.matches("href=\"/s/abc\"").count(), 2);
        assert!(html.contains(">3<"), "{html}");
        let head = &open.children()[0];
        assert!(head.get_style_utils().contains(&St::NavGroupHead.as_u16()));

        let folded = NavGroup::new("Home Lab")
            .on_toggle(toggle())
            .folded(true)
            .rows([row()])
            .build();
        let html = folded.to_static_html();
        assert!(html.contains("aria-expanded=\"false\""), "{html}");
        assert!(
            !html.contains("href="),
            "a folded group renders no rows: {html}"
        );
        let chevron = &folded.children()[0].children()[0];
        assert!(chevron
            .get_style_utils()
            .contains(&St::RotateNeg90.as_u16()));
        assert!(chevron
            .get_style_utils()
            .contains(&St::ChevronFold.as_u16()));
    }

    #[test]
    fn the_chevron_turn_stops_under_reduced_motion() {
        assert!(St::ChevronFold
            .css()
            .contains("@media (prefers-reduced-motion:reduce){transition:none}"));
        assert_eq!(
            St::NavGroupHead.css(),
            "font-size:11px;font-weight:600;letter-spacing:.06em;text-transform:uppercase"
        );
    }

    #[test]
    fn a_group_without_a_toggle_has_a_plain_title() {
        let html = NavGroup::new("home").rows([row()]).build().to_static_html();
        assert!(!html.contains("<button"), "{html}");
        assert!(!html.contains("aria-expanded"), "{html}");
        assert!(!html.contains("aria-controls"), "{html}");
    }

    #[test]
    fn a_folded_group_names_no_absent_id() {
        let html = NavGroup::new("home")
            .on_toggle(toggle())
            .folded(true)
            .rows([row()])
            .build()
            .to_static_html();
        assert!(html.contains("aria-expanded=\"false\""), "{html}");
        assert!(!html.contains("aria-controls"), "{html}");
    }

    #[test]
    fn a_lists_group_ids_are_unique() {
        let html = NavList::new()
            .id("side")
            .group(NavGroup::new("home").on_toggle(toggle()).rows([row()]))
            .group(NavGroup::new("home").on_toggle(toggle()).rows([row()]))
            .build()
            .to_static_html();
        assert!(html.contains("id=\"side-g0\""), "{html}");
        assert!(html.contains("id=\"side-g1\""), "{html}");
        assert!(html.contains("aria-controls=\"side-g1\""), "{html}");
    }

    #[test]
    fn a_list_is_a_labelled_nav() {
        let html = NavList::new()
            .aria_label("Sessions")
            .group(NavGroup::new("a").on_toggle(toggle()).rows([row()]))
            .group(NavGroup::new("b").on_toggle(toggle()))
            .build()
            .to_static_html();
        assert!(html.starts_with("<nav "), "{html}");
        assert!(html.contains("aria-label=\"Sessions\""), "{html}");
        assert_eq!(html.matches("aria-expanded").count(), 2);
    }
}
