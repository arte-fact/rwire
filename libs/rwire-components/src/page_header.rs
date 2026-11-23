//! PageHeader component.
//!
//! The top bar of a page (the conversation and session screens): a `<header>`
//! at least `--Qh` tall (56 px by default) with a hairline bottom border — on
//! the left an optional menu button (visible below the mobile threshold only),
//! then the title in an `<h1>` (it is the page's single main heading), an
//! optional mono suffix, and a one-line subtitle; on the right, the actions.
//!
//! ```ignore
//! PageHeader::new("projet@machine")
//!     .title_suffix("#12 · claude-flambeau")
//!     .subtitle("claude · /home/benoit/Workspace")
//!     .actions([Badge::status(Tone::Success, "Travaille").build()])
//!     .menu_button(back_to_list)
//!     .build()
//! ```

use std::borrow::Cow;

use rwire::style_tokens::{Bp, St};
use rwire::{el, El, ElementBuilder, HandlerSpec};

/// PageHeader builder.
#[derive(Default)]
pub struct PageHeader {
    title: Option<Cow<'static, str>>,
    title_suffix: Option<Cow<'static, str>>,
    title_suffix_title: Option<Cow<'static, str>>,
    subtitle: Option<Cow<'static, str>>,
    subtitle_mono: bool,
    actions: Vec<ElementBuilder>,
    menu: Option<HandlerSpec>,
    /// The breakpoint from which the menu button is hidden (default `md`:
    /// the button exists on mobile only).
    menu_hidden_from: Option<Bp>,
}

impl PageHeader {
    /// A header titled `title`.
    pub fn new(title: impl Into<Cow<'static, str>>) -> Self {
        Self {
            title: Some(title.into()),
            ..Self::default()
        }
    }

    /// A mono suffix after the title (e.g. the short tmux name), with
    /// `title` as its tooltip (the full name).
    pub fn title_suffix(mut self, suffix: impl Into<Cow<'static, str>>) -> Self {
        self.title_suffix = Some(suffix.into());
        self
    }

    /// The tooltip of the title suffix (the full tmux name).
    pub fn title_suffix_title(mut self, title: impl Into<Cow<'static, str>>) -> Self {
        self.title_suffix_title = Some(title.into());
        self
    }

    /// A one-line subtitle (`runtime · dossier`).
    pub fn subtitle(mut self, subtitle: impl Into<Cow<'static, str>>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// The subtitle in mono (12 px, faint).
    pub fn subtitle_mono(mut self, mono: bool) -> Self {
        self.subtitle_mono = mono;
        self
    }

    /// Action elements on the right (a status badge, an interrupt button…).
    pub fn actions(mut self, actions: impl IntoIterator<Item = ElementBuilder>) -> Self {
        self.actions.extend(actions);
        self
    }

    /// The menu button (back to the list, route `/`): an icon-only button
    /// shown below the mobile threshold only.
    pub fn menu_button(mut self, on_click: HandlerSpec) -> Self {
        self.menu = Some(on_click);
        self
    }

    /// The breakpoint from which the menu button is hidden (default `md`).
    pub fn menu_hidden_from(mut self, bp: Bp) -> Self {
        self.menu_hidden_from = Some(bp);
        self
    }

    /// Build the `<header>`.
    pub fn build(self) -> ElementBuilder {
        let mut left: Vec<ElementBuilder> = Vec::new();
        if let Some(handler) = self.menu {
            let bp = self.menu_hidden_from.unwrap_or(Bp::Md);
            left.push(
                el(El::Button)
                    .st([
                        St::DisplayInlineFlex,
                        St::ItemsCenter,
                        St::JustifyCenter,
                        St::SqIconButton,
                        St::RoundedMd,
                        St::BgTransparent,
                        St::BorderNone,
                        St::CursorPointer,
                        St::hidden_from(bp),
                    ])
                    .at_str(rwire::attr_tokens::At::AriaLabel, "Menu")
                    .data("testid", "page-header-menu")
                    .on(rwire::Ev::Click, handler)
                    .append([rwire::icons::icon_sized(rwire::icons::Icon::Menu, 18)]),
            );
        }
        // Title block: h1 (the page's main heading) + mono suffix, subtitle below.
        let mut title_row: Vec<ElementBuilder> = Vec::new();
        if let Some(title) = &self.title {
            title_row.push(
                el(El::H1)
                    .st([St::Text15, St::FontW650, St::M0, St::Truncate])
                    .data("testid", "page-header-title")
                    .text(title.as_ref()),
            );
        }
        if let Some(suffix) = &self.title_suffix {
            let mut el_suffix = el(El::Span)
                .st([
                    St::FontMono,
                    St::Text11_5,
                    St::TextFaint,
                    St::WhitespaceNowrap,
                ])
                .data("testid", "page-header-title-suffix")
                .text(suffix.as_ref());
            if let Some(t) = &self.title_suffix_title {
                el_suffix = el_suffix.at_str(rwire::attr_tokens::At::Title, t.as_ref());
            }
            title_row.push(el_suffix);
        }
        let mut title_col = el(El::Div)
            .st([St::DisplayFlex, St::ItemsBaseline, St::GapSm, St::MinW0])
            .append(title_row);
        if let Some(subtitle) = &self.subtitle {
            let mut sub = el(El::Div)
                .st([St::TextXs, St::TextFaint, St::Truncate, St::MinW0])
                .data("testid", "page-header-subtitle")
                .text(subtitle.as_ref());
            if self.subtitle_mono {
                sub = sub.st([St::FontMono]);
            }
            title_col = title_col.append([sub]);
        }
        left.push(
            el(El::Div)
                .st([St::DisplayFlex, St::FlexCol, St::MinW0])
                .append([title_col]),
        );

        let mut header = el(El::Header)
            .st([
                St::DisplayFlex,
                St::ItemsCenter,
                St::GapMd,
                St::MinHHeader,
                St::PxGutterPhone,
                St::BorderBDefault,
                St::WFull,
            ])
            .sm([St::PxGutter])
            .data("testid", "page-header")
            .append(left);
        if !self.actions.is_empty() {
            header = header
                .append([el(El::Span).st([St::Flex1])])
                .append([el(El::Div)
                    .st([St::DisplayFlex, St::ItemsCenter, St::GapSm])
                    .data("testid", "page-header-actions")
                    .append(self.actions)]);
        }
        header
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rwire::{ChangeSet, HandlerSpec};

    #[derive(rwire::State, Default)]
    #[storage(memory)]
    struct S;

    fn handler() -> HandlerSpec {
        fn noop(_s: &mut S) {}
        HandlerSpec::from_fn_with_changes::<S>(noop, ChangeSet::all())
            .with_handler_id(rwire::stable_handler_id("page_header_tests", "noop"))
    }

    #[test]
    fn the_header_is_a_56px_header_with_h1_mono_suffix_and_actions() {
        let header = PageHeader::new("projet@machine")
            .title_suffix("#12")
            .title_suffix_title("claude-flambeau")
            .subtitle("claude · /home/benoit/Workspace")
            .subtitle_mono(true)
            .actions([el(El::Span).text("badge")])
            .menu_button(handler())
            .build();
        let html = header.to_static_html();
        assert!(html.contains("<header"), "{html}");
        assert!(html.contains("<h1"), "the title is the page's h1: {html}");
        assert!(
            html.contains("data-testid=\"page-header-title\">projet@machine<"),
            "{html}"
        );
        assert!(html.contains("title=\"claude-flambeau\""), "{html}");
        assert!(
            html.contains("data-testid=\"page-header-subtitle\""),
            "{html}"
        );
        assert!(
            html.contains("data-testid=\"page-header-actions\">"),
            "{html}"
        );
        assert!(html.contains("data-testid=\"page-header-menu\""), "{html}");
        let utils = header.get_style_utils();
        for st in [St::MinHHeader, St::BorderBDefault, St::DisplayFlex] {
            assert!(utils.contains(&st.as_u16()), "{st:?}");
        }
        // The menu button only exists below md.
        let menu = header
            .children()
            .iter()
            .find(|c| {
                c.attributes()
                    .iter()
                    .any(|(k, v)| k == "data-testid" && v == "page-header-menu")
            })
            .expect("menu button");
        assert!(menu
            .get_style_utils()
            .contains(&St::hidden_from(Bp::Md).as_u16()));
        assert!(!html.contains("style="), "no inline style: {html}");
        assert_eq!(St::MinHHeader.css(), "min-height:var(--Qh,3.5rem)");
    }
}
