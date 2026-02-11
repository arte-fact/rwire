//! Skeleton component.
//!
//! Loading placeholder that shows content shape before data arrives.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{Skeleton, SkeletonShape};
//!
//! // Single text line
//! Skeleton::text().build()
//!
//! // Multi-line text placeholder
//! Skeleton::text().lines(3).build()
//!
//! // Circle (avatar placeholder)
//! Skeleton::circle().build()
//!
//! // Rectangle (card/image placeholder)
//! Skeleton::rect().build()
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Skeleton shape variant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SkeletonShape {
    /// Single-line text placeholder
    #[default]
    Text,
    /// Circle (avatar placeholder)
    Circle,
    /// Rectangle (card/image placeholder)
    Rect,
}

/// Skeleton builder.
#[derive(Clone, Debug)]
pub struct Skeleton {
    shape: SkeletonShape,
    lines: u8,
    extra_class: Option<Cow<'static, str>>,
}

impl Default for Skeleton {
    fn default() -> Self {
        Self {
            shape: SkeletonShape::Text,
            lines: 1,
            extra_class: None,
        }
    }
}

#[rwire::component]
impl Skeleton {
    /// Create a text line skeleton.
    pub fn text() -> Self {
        Self::default()
    }

    /// Create a circle skeleton (avatar placeholder).
    pub fn circle() -> Self {
        Self {
            shape: SkeletonShape::Circle,
            ..Self::default()
        }
    }

    /// Create a rectangle skeleton (card/image placeholder).
    pub fn rect() -> Self {
        Self {
            shape: SkeletonShape::Rect,
            ..Self::default()
        }
    }

    /// Set number of lines (for text shape).
    pub fn lines(mut self, lines: u8) -> Self {
        self.lines = lines.max(1);
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute base style tokens for the shimmer effect.
    pub fn compute_base_tokens() -> Vec<St> {
        vec![St::BgShimmer, St::RoundedMd]
    }

    /// Compute shape-specific style tokens.
    pub fn compute_shape_tokens(&self) -> Vec<St> {
        match self.shape {
            SkeletonShape::Text => vec![St::WFull, St::H1rem],
            SkeletonShape::Circle => vec![St::W3rem, St::H3rem, St::RoundedFull],
            SkeletonShape::Rect => vec![St::WFull, St::MinH6rem],
        }
    }

    /// Build the skeleton into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        if self.shape == SkeletonShape::Text && self.lines > 1 {
            // Multi-line: wrap in a flex column
            let mut container = el(El::Div).st([St::DisplayFlex, St::FlexCol, St::GapSm]);

            if let Some(ref extra) = self.extra_class {
                container = container.class(extra.as_ref());
            }

            let children: Vec<ElementBuilder> = (0..self.lines)
                .map(|i| {
                    let mut tokens = Self::compute_base_tokens();
                    tokens.extend(vec![St::H1rem]);
                    // Last line is shorter for a natural look
                    if i == self.lines - 1 {
                        tokens.push(St::MaxWLg);
                    } else {
                        tokens.push(St::WFull);
                    }
                    el(El::Div).st(tokens)
                })
                .collect();

            container.append(children)
        } else {
            let mut tokens = Self::compute_base_tokens();
            tokens.extend(self.compute_shape_tokens());

            let mut builder = el(El::Div).st(tokens);

            if let Some(ref extra) = self.extra_class {
                builder = builder.class(extra.as_ref());
            }

            builder
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skeleton_text() {
        let sk = Skeleton::text();
        assert_eq!(sk.shape, SkeletonShape::Text);
        assert_eq!(sk.lines, 1);
    }

    #[test]
    fn test_skeleton_circle() {
        let sk = Skeleton::circle();
        assert_eq!(sk.shape, SkeletonShape::Circle);
    }

    #[test]
    fn test_skeleton_rect() {
        let sk = Skeleton::rect();
        assert_eq!(sk.shape, SkeletonShape::Rect);
    }

    #[test]
    fn test_skeleton_lines() {
        let sk = Skeleton::text().lines(3);
        assert_eq!(sk.lines, 3);
    }

    #[test]
    fn test_skeleton_base_tokens() {
        let tokens = Skeleton::compute_base_tokens();
        assert!(tokens.contains(&St::BgShimmer));
        assert!(tokens.contains(&St::RoundedMd));
    }

    #[test]
    fn test_skeleton_text_shape_tokens() {
        let sk = Skeleton::text();
        let tokens = sk.compute_shape_tokens();
        assert!(tokens.contains(&St::WFull));
        assert!(tokens.contains(&St::H1rem));
    }

    #[test]
    fn test_skeleton_circle_shape_tokens() {
        let sk = Skeleton::circle();
        let tokens = sk.compute_shape_tokens();
        assert!(tokens.contains(&St::W3rem));
        assert!(tokens.contains(&St::H3rem));
        assert!(tokens.contains(&St::RoundedFull));
    }

    #[test]
    fn test_skeleton_rect_shape_tokens() {
        let sk = Skeleton::rect();
        let tokens = sk.compute_shape_tokens();
        assert!(tokens.contains(&St::WFull));
        assert!(tokens.contains(&St::MinH6rem));
    }
}
