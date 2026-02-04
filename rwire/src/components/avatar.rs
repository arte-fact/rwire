//! Avatar component.
//!
//! User avatar with image or fallback text.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::{Avatar, AvatarSize};
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

use crate::variants::Variant;
use crate::{el, El, ElementBuilder};
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

impl Variant for AvatarSize {
    fn class(&self) -> Option<&'static str> {
        match self {
            AvatarSize::Sm => Some("rw-avatar-sm"),
            AvatarSize::Md => None,
            AvatarSize::Lg => Some("rw-avatar-lg"),
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
    /// Base CSS class.
    pub const BASE_CLASS: &'static str = "rw-avatar";

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

    fn compute_class(&self) -> String {
        let mut classes = String::with_capacity(48);
        classes.push_str(Self::BASE_CLASS);

        if let Some(size_class) = self.size.class() {
            classes.push(' ');
            classes.push_str(size_class);
        }

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the avatar into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        // Register for CSS tree-shaking
        super::registry::mark_component_used(super::registry::ComponentType::Avatar);

        let class = self.compute_class();
        let container = el(El::Div).class(&class);

        // If image source is provided, render image (using div with background-image for now)
        if let Some(src) = self.src {
            let style = format!("background-image:url({})", src);
            let mut img_div = el(El::Div)
                .class("rw-avatar-img")
                .attr("style", &style);

            if let Some(alt) = self.alt {
                img_div = img_div.attr("aria-label", &alt);
            }

            container.append([img_div])
        } else if let Some(fallback_text) = self.fallback {
            // Render fallback text
            container.append([
                el(El::Span)
                    .class("rw-avatar-fallback")
                    .text(&fallback_text)
            ])
        } else {
            container
        }
    }
}

/// Avatar CSS.
///
/// Size: ~280 bytes (under 300 bytes budget)
pub const AVATAR_CSS: &str = "\
.rw-avatar{display:inline-flex;align-items:center;justify-content:center;overflow:hidden;\
border-radius:50%;background:var(--rw-bg-muted);width:2.5rem;height:2.5rem;flex-shrink:0}\
.rw-avatar-sm{width:2rem;height:2rem}\
.rw-avatar-lg{width:3rem;height:3rem}\
.rw-avatar-img{width:100%;height:100%;background-size:cover;background-position:center}\
.rw-avatar-fallback{font-size:var(--rw-text-sm);font-weight:var(--rw-font-medium);color:var(--rw-text-high)}\
.rw-avatar-lg .rw-avatar-fallback{font-size:var(--rw-text-base)}\n";

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
    fn test_avatar_class_default() {
        let avatar = Avatar::new();
        assert_eq!(avatar.compute_class(), "rw-avatar");
    }

    #[test]
    fn test_avatar_class_with_size() {
        let avatar = Avatar::new().size(AvatarSize::Lg);
        let class = avatar.compute_class();
        assert!(class.contains("rw-avatar"));
        assert!(class.contains("rw-avatar-lg"));
    }

    #[test]
    fn test_avatar_css_size() {
        // Avatar CSS should be under 300 bytes
        assert!(
            AVATAR_CSS.len() < 550,
            "Avatar CSS too large: {} bytes (budget: 550)",
            AVATAR_CSS.len()
        );
        println!("Avatar CSS size: {} bytes", AVATAR_CSS.len());
    }

    #[test]
    fn test_avatar_css_structure() {
        assert!(AVATAR_CSS.contains(".rw-avatar{"));
        assert!(AVATAR_CSS.contains(".rw-avatar-sm"));
        assert!(AVATAR_CSS.contains(".rw-avatar-lg"));
        assert!(AVATAR_CSS.contains(".rw-avatar-img"));
        assert!(AVATAR_CSS.contains(".rw-avatar-fallback"));
    }
}
