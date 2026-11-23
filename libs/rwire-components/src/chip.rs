//! Chip component.
//!
//! A selectable chip for filters, view toggles, and inline tab pickers — the middle ground
//! between a `Button` (an action) and `Tabs` (a full navigation bar with content panes).
//! Accent-filled when active; muted with a hover lift otherwise. Corners follow the
//! theme's radius scale (square under `RadiusScale::None`).
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{Chip, ChipGroup};
//!
//! Chip::new("All").active(true).build()
//! Chip::new("Failed").count(3).on_click(set_filter()).build().data("filter", "failed")
//! Chip::new("Docs").href("/docs").active(true).build()
//! ChipGroup::new("Filters").chips([Chip::new("All").active(true).build()]).build()
//! ```

use rwire::style_tokens::St;
use rwire::{el, At, Av, El, ElementBuilder, Ev, HandlerSpec, Tone};
use std::borrow::Cow;

/// Chip builder.
#[derive(Debug, Default)]
pub struct Chip {
    label: Cow<'static, str>,
    /// `Some` when set explicitly: a button chip is then a toggle (`aria-pressed`).
    active: Option<bool>,
    on_click: Option<HandlerSpec>,
    extra_class: Option<Cow<'static, str>>,
    count: Option<u64>,
    href: Option<Cow<'static, str>>,
}

impl Chip {
    /// Create a chip with a label.
    pub fn new(label: impl Into<Cow<'static, str>>) -> Self {
        Self {
            label: label.into(),
            ..Self::default()
        }
    }

    /// Mark the chip as the active selection (accent-filled). Setting it makes
    /// a button chip a toggle (`aria-pressed`); a link chip (`href`) is marked
    /// `aria-current="page"` when active.
    pub fn active(mut self, active: bool) -> Self {
        self.active = Some(active);
        self
    }

    /// Show a counter after the label.
    pub fn count(mut self, n: u64) -> Self {
        self.count = Some(n);
        self
    }

    /// Make the chip a client-routed link (`<a href data-route>`); when active it
    /// is the current page (`aria-current="page"`).
    pub fn href(mut self, path: impl Into<Cow<'static, str>>) -> Self {
        self.href = Some(path.into());
        self
    }

    /// Attach the click handler. Chain `.data(...)` on the built element for the
    /// handler's payload.
    pub fn on_click(mut self, handler: HandlerSpec) -> Self {
        self.on_click = Some(handler);
        self
    }

    /// Add extra CSS classes.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute the style tokens for the current state: 12 px text, fully rounded
    /// (radius 999); active chip gets the accent solid fill and its `on_solid` ink.
    pub fn compute_tokens(&self) -> Vec<St> {
        let mut tokens = vec![
            St::TextXs,
            St::PxChip,
            St::PyChip,
            St::RoundedFull,
            St::BorderDefault,
            St::CursorPointer,
        ];
        if self.active == Some(true) {
            tokens.extend([
                Tone::Accent.solid(),
                Tone::Accent.on_solid(),
                Tone::Accent.border(),
            ]);
        } else {
            // At rest: no ground, the hairline, the soft ink.
            tokens.extend([St::BgTransparent, St::TextMuted]);
        }
        tokens
    }

    /// Build the chip into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let active = self.active == Some(true);
        let mut builder = if let Some(ref href) = self.href {
            let link = rwire::router::Link::route(href);
            if active {
                link.attr("aria-current", "page")
            } else {
                link
            }
        } else {
            let button = el(El::Button).at(At::Type, Av::Button);
            match self.active {
                Some(on) => button.at(At::AriaPressed, if on { Av::True } else { Av::False }),
                None => button,
            }
        }
        .st(self.compute_tokens());
        if let Some(handler) = self.on_click {
            builder = builder.on(Ev::Click, handler);
        }
        if let Some(ref extra) = self.extra_class {
            builder = builder.class(extra.as_ref());
        }
        if !active {
            builder = builder.hover([St::TextHigh]);
        }
        if let Some(n) = self.count {
            // The accessible name separates the label from the count: "Failed, 3".
            builder = builder
                .attr("aria-label", &format!("{}, {n}", self.label))
                .st([St::DisplayInlineFlex, St::ItemsCenter, St::GapXs])
                .text(self.label.as_ref())
                .append([el(El::Span).text(&n.to_string())]);
        } else {
            builder = builder.text(self.label.as_ref());
        }
        builder
    }
}

/// A group of chips: a `role="group"` container with an accessible label.
pub struct ChipGroup {
    label: Cow<'static, str>,
    chips: Vec<ElementBuilder>,
}

impl ChipGroup {
    /// Create a group with its accessible label.
    pub fn new(label: impl Into<Cow<'static, str>>) -> Self {
        Self {
            label: label.into(),
            chips: Vec::new(),
        }
    }

    /// Set the chips of the group.
    pub fn chips(mut self, chips: impl IntoIterator<Item = ElementBuilder>) -> Self {
        self.chips.extend(chips);
        self
    }

    /// Build the group into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        el(El::Div)
            .st([St::DisplayFlex, St::ItemsCenter, St::GapSm, St::FlexWrap])
            .at_str(At::Role, "group")
            .at_str(At::AriaLabel, self.label.as_ref())
            .append(self.chips)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chip_count_renders_a_counter_after_the_label() {
        let html = Chip::new("Failed").count(3).build().to_static_html();
        assert!(html.contains("<button"), "{html}");
        assert!(html.contains("Failed"), "{html}");
        assert!(html.contains("<span>3</span>"), "{html}");
    }

    #[test]
    fn a_link_chip_is_routed_and_marks_the_current_page() {
        let html = Chip::new("Docs")
            .href("/docs")
            .active(true)
            .build()
            .to_static_html();
        assert!(html.starts_with("<a "), "{html}");
        assert!(html.contains("data-route"), "{html}");
        assert!(html.contains("aria-current=\"page\""), "{html}");
        assert!(
            !html.contains("aria-pressed"),
            "not allowed on a link: {html}"
        );
        let html = Chip::new("Docs").href("/docs").build().to_static_html();
        assert!(
            !html.contains("aria-current") && !html.contains("aria-pressed"),
            "{html}"
        );
    }

    #[test]
    fn a_button_chip_is_a_toggle_only_when_active_is_set() {
        let plain = Chip::new("All").build().to_static_html();
        assert!(!plain.contains("aria-pressed"), "{plain}");
        let off = Chip::new("All").active(false).build().to_static_html();
        assert!(off.contains("aria-pressed=\"false\""), "{off}");
    }

    #[test]
    fn a_counted_chip_names_itself_with_a_separator() {
        let html = Chip::new("Failed").count(3).build().to_static_html();
        assert!(html.contains("aria-label=\"Failed, 3\""), "{html}");
    }

    /// The default resting chip: no ground, the hairline, the soft ink.
    #[test]
    fn an_inactive_chip_rests_on_nothing() {
        let tokens = Chip::new("All").compute_tokens();
        for st in [St::BgTransparent, St::BorderDefault, St::TextMuted] {
            assert!(tokens.contains(&st), "{st:?}");
        }
        assert!(!tokens.contains(&St::BgApp));
        let css = rwire::theme::generate_theme_css(
            &rwire::theme::Theme::new().density(rwire::theme::Density::Compact),
        );
        assert!(css.contains("--Qzcx:9.6px;--Qzcy:2.4px;"), "{css}");
    }

    #[test]
    fn chip_group_has_role_group_and_aria_label() {
        let html = ChipGroup::new("Filters")
            .chips([
                Chip::new("All").build(),
                Chip::new("Failed").count(2).build(),
            ])
            .build()
            .to_static_html();
        assert!(html.contains("role=\"group\""), "{html}");
        assert!(html.contains("aria-label=\"Filters\""), "{html}");
        assert_eq!(html.matches("<button").count(), 2, "{html}");
    }

    #[test]
    fn active_chip_uses_accent_solid_with_on_solid_ink_and_radius_999() {
        let tokens = Chip::new("All").active(true).compute_tokens();
        assert!(tokens.contains(&Tone::Accent.solid()), "{tokens:?}");
        assert!(tokens.contains(&Tone::Accent.on_solid()), "{tokens:?}");
        assert!(tokens.contains(&St::RoundedFull), "radius 999");
        assert!(tokens.contains(&St::TextXs), "12 px font");
    }
}
