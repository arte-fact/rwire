//! Avatar component.
//!
//! User avatar with image or fallback text.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{Avatar, AvatarSize};
//!
//! // With image
//! Avatar::new()
//!     .src("/users/avatar.jpg")
//!     .alt("John Doe")
//!     .build()
//!
//! // With fallback text
//! Avatar::new()
//!     .fallback("JD")
//!     .size(AvatarSize::Lg)
//!     .build()
//! ```

use rwire::attr_tokens::At;
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Avatar size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AvatarSize {
    /// Small: 32px
    Sm,
    /// Medium: 40px (default)
    #[default]
    Md,
    /// Large: 48px
    Lg,
}

impl AvatarSize {
    /// Get size tokens for width/height.
    fn size_tokens(self) -> Vec<St> {
        match self {
            AvatarSize::Sm => vec![St::W2rem, St::H2rem],
            AvatarSize::Md => vec![St::W2_5rem, St::H2_5rem],
            AvatarSize::Lg => vec![St::W3rem, St::H3rem],
        }
    }
}

/// Avatar builder.
#[derive(Clone, Debug, Default)]
pub struct Avatar {
    src: Option<Cow<'static, str>>,
    alt: Option<Cow<'static, str>>,
    fallback: Option<Cow<'static, str>>,
    size: AvatarSize,
    extra_class: Option<Cow<'static, str>>,
}

impl Avatar {
    /// Create a new avatar.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the image source.
    pub fn src(mut self, src: impl Into<Cow<'static, str>>) -> Self {
        self.src = Some(src.into());
        self
    }

    /// Set the image alt text.
    pub fn alt(mut self, alt: impl Into<Cow<'static, str>>) -> Self {
        self.alt = Some(alt.into());
        self
    }

    /// Set fallback text (shown if no image).
    pub fn fallback(mut self, fallback: impl Into<Cow<'static, str>>) -> Self {
        self.fallback = Some(fallback.into());
        self
    }

    /// Set the avatar size.
    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for the avatar container.
    pub fn compute_tokens(&self) -> Vec<St> {
        vec![
            St::DisplayInlineFlex,
            St::ItemsCenter,
            St::JustifyCenter,
            St::OverflowHidden,
            St::RoundedFull,
            St::BgMuted,
            St::FlexShrink,
        ]
    }

    /// Build the avatar into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut tokens = self.compute_tokens();
        tokens.extend(self.size.size_tokens());
        let mut container = el(El::Div)
            .st(tokens);

        if let Some(ref extra) = self.extra_class {
            container = container.class(extra.as_ref());
        }

        if let Some(src) = self.src {
            let mut img_div = el(El::Div)
                .st([St::WFull, St::HFull, St::BgSizeCover, St::BgPosCenter])
                .attr("style", &format!("background-image:url({})", src));

            if let Some(alt) = self.alt {
                img_div = img_div.at_str(At::AriaLabel, &alt);
            }

            container.append([img_div])
        } else if let Some(fallback_text) = self.fallback {
            container.append([
                el(El::Span)
                    .st([St::TextSm, St::FontMedium, St::TextHigh])
                    .text(&fallback_text)
            ])
        } else {
            container
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avatar_defaults() {
        let avatar = Avatar::new();
        assert!(avatar.src.is_none());
        assert!(avatar.fallback.is_none());
        assert_eq!(avatar.size, AvatarSize::Md);
    }

    #[test]
    fn test_avatar_default_tokens() {
        let avatar = Avatar::new();
        let tokens = avatar.compute_tokens();
        assert!(tokens.contains(&St::DisplayInlineFlex));
        assert!(tokens.contains(&St::ItemsCenter));
        assert!(tokens.contains(&St::RoundedFull));
        assert!(tokens.contains(&St::BgMuted));
    }

    #[test]
    fn test_avatar_size_tokens() {
        assert_eq!(AvatarSize::Sm.size_tokens(), vec![St::W2rem, St::H2rem]);
        assert_eq!(AvatarSize::Md.size_tokens(), vec![St::W2_5rem, St::H2_5rem]);
        assert_eq!(AvatarSize::Lg.size_tokens(), vec![St::W3rem, St::H3rem]);
    }

}
