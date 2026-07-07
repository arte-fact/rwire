//! Tooltip component.
//!
//! CSS-only tooltip that appears on hover/focus. No JavaScript needed.
//! Uses CSS nesting (`&:hover>[data-tip]`) for child visibility toggle.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{Tooltip, TooltipPosition};
//!
//! Tooltip::new("Delete this item")
//!     .position(TooltipPosition::Top)
//!     .child(Button::primary("Delete").build())
//!     .build()
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Tooltip position relative to its trigger.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TooltipPosition {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

/// Tooltip builder.
#[derive(Clone, Default)]
pub struct Tooltip {
    text: Cow<'static, str>,
    position: TooltipPosition,
    child: Option<ElementBuilder>,
    extra_class: Option<Cow<'static, str>>,
}

#[rwire::component]
impl Tooltip {
    /// Create a new tooltip with text content.
    pub fn new(text: impl Into<Cow<'static, str>>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }

    /// Set the tooltip position.
    pub fn position(mut self, position: TooltipPosition) -> Self {
        self.position = position;
        self
    }

    /// Set the trigger element.
    pub fn child(mut self, child: ElementBuilder) -> Self {
        self.child = Some(child);
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for the tooltip popup.
    pub fn compute_popup_tokens() -> Vec<St> {
        vec![
            St::PositionAbsolute,
            St::TooltipBg,
            St::TextOnEmphasis,
            St::TextXs,
            St::RoundedSm,
            St::PxSm,
            St::PySm,
            St::WhitespaceNowrapInline,
            St::Z50,
            St::PointerEventsNone,
            St::Opacity0,
            St::TransitionOpacity,
        ]
    }

    /// Get position-specific inline style.
    fn position_style(&self) -> &'static str {
        match self.position {
            TooltipPosition::Top => {
                "bottom:100%;left:50%;transform:translateX(-50%);margin-bottom:4px"
            }
            TooltipPosition::Bottom => {
                "top:100%;left:50%;transform:translateX(-50%);margin-top:4px"
            }
            TooltipPosition::Left => {
                "right:100%;top:50%;transform:translateY(-50%);margin-right:4px"
            }
            TooltipPosition::Right => {
                "left:100%;top:50%;transform:translateY(-50%);margin-left:4px"
            }
        }
    }

    /// Build the tooltip into an ElementBuilder.
    ///
    /// The container uses `HoverShowChild` token which applies CSS nesting:
    /// `&:hover>[data-tip],&:focus-within>[data-tip]{opacity:1}`
    /// The popup has `data-tip` attribute and starts at `opacity:0`.
    pub fn build(self) -> ElementBuilder {
        let popup = el(El::Span)
            .st(Self::compute_popup_tokens())
            .attr("style", self.position_style())
            .attr("data-tip", "")
            .text(&self.text);

        let mut container = el(El::Div)
            .st([
                St::PositionRelative,
                St::DisplayInlineFlex,
                St::HoverShowChild,
            ])
            // Lift the anchor while hovered so the popup paints above later
            // siblings (rows, toolbars) that would otherwise stack over it.
            .hover([St::Z50]);

        if let Some(ref extra) = self.extra_class {
            container = container.class(extra.as_ref());
        }

        if let Some(child) = self.child {
            container = container.append([child]);
        }

        container.append([popup])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tooltip_new() {
        let tooltip = Tooltip::new("Help text");
        assert_eq!(tooltip.text.as_ref(), "Help text");
        assert_eq!(tooltip.position, TooltipPosition::Top);
    }

    #[test]
    fn test_tooltip_position() {
        let tooltip = Tooltip::new("Help").position(TooltipPosition::Bottom);
        assert_eq!(tooltip.position, TooltipPosition::Bottom);
    }

    #[test]
    fn test_tooltip_popup_tokens() {
        let tokens = Tooltip::compute_popup_tokens();
        assert!(tokens.contains(&St::PositionAbsolute));
        assert!(tokens.contains(&St::TooltipBg));
        assert!(tokens.contains(&St::TextOnEmphasis));
        assert!(tokens.contains(&St::TextXs));
        assert!(tokens.contains(&St::Opacity0));
    }

    #[test]
    fn test_tooltip_position_styles() {
        let top = Tooltip::new("t").position(TooltipPosition::Top);
        assert!(top.position_style().contains("bottom:100%"));

        let bottom = Tooltip::new("t").position(TooltipPosition::Bottom);
        assert!(bottom.position_style().contains("top:100%"));

        let left = Tooltip::new("t").position(TooltipPosition::Left);
        assert!(left.position_style().contains("right:100%"));

        let right = Tooltip::new("t").position(TooltipPosition::Right);
        assert!(right.position_style().contains("left:100%"));
    }
}
