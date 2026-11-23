//! `Sidebar`: a full-height column with slots — header, search, filters, a
//! scrolling body and a footer kept at the bottom — on the theme's sidebar
//! ground (`St::BgSidebar`), clear of the device's safe areas.
//!
//! ```ignore
//! Sidebar::new()
//!     .header(brand)
//!     .search(Input::search().size(InputSize::Sm).build())
//!     .filters(chips)
//!     .body(NavList::new().groups(machines).build())
//!     .footer(footer_buttons)
//!     .build()
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// A sidebar column (`<aside>`).
#[derive(Clone, Default)]
pub struct Sidebar {
    header: Option<ElementBuilder>,
    search: Option<ElementBuilder>,
    filters: Option<ElementBuilder>,
    body: Option<ElementBuilder>,
    footer: Option<ElementBuilder>,
    aria_label: Option<Cow<'static, str>>,
    no_border: bool,
}

impl Sidebar {
    /// An empty sidebar.
    pub fn new() -> Self {
        Self::default()
    }

    /// The top slot (a brand), clear of the top safe area.
    pub fn header(mut self, header: ElementBuilder) -> Self {
        self.header = Some(header);
        self
    }

    /// A search field, under the header.
    pub fn search(mut self, search: ElementBuilder) -> Self {
        self.search = Some(search);
        self
    }

    /// Filters (chips), under the search.
    pub fn filters(mut self, filters: ElementBuilder) -> Self {
        self.filters = Some(filters);
        self
    }

    /// The body: it takes the remaining height and scrolls on its own.
    pub fn body(mut self, body: ElementBuilder) -> Self {
        self.body = Some(body);
        self
    }

    /// The footer: kept at the bottom, clear of the bottom safe area.
    pub fn footer(mut self, footer: ElementBuilder) -> Self {
        self.footer = Some(footer);
        self
    }

    /// The 1 px hairline on the right edge (`St::BorderRDefault`), on by
    /// default: the sidebar owns its edge (an `AppShell` column draws none).
    /// Off for a sidebar that sits on another edge or in a drawer.
    pub fn border(mut self, on: bool) -> Self {
        self.no_border = !on;
        self
    }

    /// The landmark's accessible name (`aria-label`).
    pub fn aria_label(mut self, label: impl Into<Cow<'static, str>>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// The column's own tokens.
    pub fn compute_tokens(&self) -> Vec<St> {
        let mut tokens = vec![
            St::DisplayFlex,
            St::FlexCol,
            St::HFull,
            St::MinH0,
            St::BgSidebar,
            St::PlSafe,
        ];
        if !self.no_border {
            tokens.push(St::BorderRDefault);
        }
        tokens
    }

    /// Build the `<aside>`.
    pub fn build(self) -> ElementBuilder {
        let tokens = self.compute_tokens();
        let slot = |content: ElementBuilder, tokens: &[St]| {
            el(El::Div).st(tokens.iter().copied()).append([content])
        };
        let mut children = Vec::new();
        if let Some(header) = self.header {
            // The default brand row: 14 px 14 px 10 px, the top safe area included.
            children.push(slot(
                header,
                &[St::PxSidebarHead, St::PtSidebarHeadSafe, St::PbSidebarHead],
            ));
        }
        if let Some(search) = self.search {
            children.push(slot(search, &[St::PxSp3, St::PbSm]));
        }
        if let Some(filters) = self.filters {
            children.push(slot(filters, &[St::PxSp3, St::PbSm]));
        }
        // The body always takes the remaining height, so the footer sits at
        // the bottom even when the body is short.
        let mut body = el(El::Div).attr("data-sidebar", "body").st([
            St::Flex1,
            St::MinH0,
            St::OverflowYAuto,
            St::PxSm,
            St::PbSm,
        ]);
        if let Some(content) = self.body {
            body = body.append([content]);
        }
        children.push(body);
        if let Some(footer) = self.footer {
            children.push(
                slot(
                    footer,
                    &[
                        St::BorderT,
                        St::PxSp3,
                        St::PtSidebarFoot,
                        St::PbSidebarFootSafe,
                        St::Text12_5,
                    ],
                )
                .attr("data-sidebar", "footer"),
            );
        }
        let mut aside = el(El::Aside).st(tokens);
        if let Some(label) = &self.aria_label {
            aside = aside.attr("aria-label", label);
        }
        aside.append(children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_in_order_with_a_scrolling_body_and_a_bottom_footer() {
        let built = Sidebar::new()
            .header(el(El::Span).text("brand"))
            .search(el(El::Input))
            .filters(el(El::Span).text("chips"))
            .body(el(El::Span).text("list"))
            .footer(el(El::Span).text("foot"))
            .aria_label("Sessions")
            .build();
        let html = built.to_static_html();
        assert!(html.starts_with("<aside "), "{html}");
        assert!(html.contains("aria-label=\"Sessions\""), "{html}");
        let order = ["brand", "<input", "chips", "list", "foot"].map(|s| html.find(s).unwrap());
        assert!(order.windows(2).all(|w| w[0] < w[1]), "{html}");
        let kids = built.children();
        assert_eq!(kids.len(), 5);
        let body = kids[3].get_style_utils();
        for st in [St::Flex1, St::MinH0, St::OverflowYAuto] {
            assert!(body.contains(&st.as_u16()), "body {st:?}");
        }
        assert!(kids[4]
            .get_style_utils()
            .contains(&St::PbSidebarFootSafe.as_u16()));
        assert!(kids[0]
            .get_style_utils()
            .contains(&St::PtSidebarHeadSafe.as_u16()));
        let aside = built.get_style_utils();
        for st in [St::HFull, St::FlexCol, St::BgSidebar, St::PlSafe] {
            assert!(aside.contains(&st.as_u16()), "aside {st:?}");
        }
    }

    /// The default footer: 10 px 12 px, the bottom safe area added.
    #[test]
    fn the_footer_is_padded_like_the_old_front() {
        let built = Sidebar::new().footer(el(El::Span)).build();
        let foot = built.children()[1].get_style_utils();
        for st in [
            St::BorderT,
            St::PxSp3,
            St::PtSidebarFoot,
            St::PbSidebarFootSafe,
        ] {
            assert!(foot.contains(&st.as_u16()), "{st:?}");
        }
        assert_eq!(St::PtSidebarFoot.css(), "padding-top:10px");
        assert_eq!(
            St::PbSidebarFootSafe.css(),
            "padding-bottom:calc(10px + env(safe-area-inset-bottom,0px))"
        );
    }

    /// The default sidebar edge: a 1 px hairline on the right, by default;
    /// `.border(false)` drops it.
    #[test]
    fn the_sidebar_draws_its_right_edge() {
        let built = Sidebar::new().build();
        assert!(built
            .get_style_utils()
            .contains(&St::BorderRDefault.as_u16()));
        let bare = Sidebar::new().border(false).build();
        assert!(!bare
            .get_style_utils()
            .contains(&St::BorderRDefault.as_u16()));
    }

    /// The default header slot: 14 px 14 px 10 px, the top safe area
    /// included; its footer text: 12.5 px.
    #[test]
    fn the_header_and_footer_text_match_the_old_front() {
        let built = Sidebar::new()
            .header(el(El::Span))
            .footer(el(El::Span))
            .build();
        let head = built.children()[0].get_style_utils();
        for st in [St::PtSidebarHeadSafe, St::PxSidebarHead, St::PbSidebarHead] {
            assert!(head.contains(&st.as_u16()), "header {st:?}");
        }
        let foot = built.children()[2].get_style_utils();
        assert!(foot.contains(&St::Text12_5.as_u16()), "footer 12.5 px");
    }

    #[test]
    fn the_body_is_there_even_without_content() {
        let built = Sidebar::new().footer(el(El::Span)).build();
        assert_eq!(built.children().len(), 2, "body + footer");
    }
}
