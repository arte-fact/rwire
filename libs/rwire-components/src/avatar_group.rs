//! AvatarGroup component.
//!
//! Stacked display of multiple avatars with an overflow count.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{AvatarGroup, Avatar, AvatarSize};
//!
//! AvatarGroup::new()
//!     .avatar(Avatar::new().fallback("AB").size(AvatarSize::Sm))
//!     .avatar(Avatar::new().fallback("CD").size(AvatarSize::Sm))
//!     .avatar(Avatar::new().fallback("EF").size(AvatarSize::Sm))
//!     .max_visible(2) // Shows 2 avatars + "+1" overflow
//!     .build()
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};

use super::{Avatar, AvatarSize};

/// AvatarGroup builder.
#[derive(Clone, Default)]
pub struct AvatarGroup {
    avatars: Vec<Avatar>,
    max_visible: Option<usize>,
    size: AvatarSize,
}

impl AvatarGroup {
    /// Create a new avatar group.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an avatar to the group.
    pub fn avatar(mut self, avatar: Avatar) -> Self {
        self.avatars.push(avatar);
        self
    }

    /// Set the maximum number of visible avatars.
    /// Excess avatars are shown as a "+N" count badge.
    pub fn max_visible(mut self, max: usize) -> Self {
        self.max_visible = Some(max);
        self
    }

    /// Set the size for all avatars.
    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    /// Compute style tokens for the group container.
    pub fn compute_tokens() -> Vec<St> {
        vec![St::DisplayFlex, St::ItemsCenter]
    }

    /// Build the avatar group into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut container = el(El::Div).st(Self::compute_tokens());

        let max = self.max_visible.unwrap_or(self.avatars.len());
        let visible = self.avatars.len().min(max);
        let overflow = self.avatars.len().saturating_sub(max);

        for (i, avatar) in self.avatars.into_iter().take(visible).enumerate() {
            let built = avatar.size(self.size).build();
            let mut wrapper = el(El::Div).st([St::AvatarRing]);
            if i > 0 {
                wrapper = wrapper.st([St::NegMlOverlap]);
            }
            wrapper = wrapper.append([built]);
            container = container.append([wrapper]);
        }

        if overflow > 0 {
            let count_badge = el(El::Div)
                .st([
                    St::DisplayInlineFlex,
                    St::ItemsCenter,
                    St::JustifyCenter,
                    St::RoundedFull,
                    St::BgEmphasis,
                    St::TextXs,
                    St::FontMedium,
                    St::NegMlOverlap,
                    St::AvatarRing,
                ])
                .st(self.size.count_size_tokens())
                .text(&format!("+{overflow}"));

            container = container.append([count_badge]);
        }

        container
    }
}

impl AvatarSize {
    fn count_size_tokens(self) -> Vec<St> {
        match self {
            AvatarSize::Sm => vec![St::W2rem, St::H2rem],
            AvatarSize::Md => vec![St::W2_5rem, St::H2_5rem],
            AvatarSize::Lg => vec![St::W3rem, St::H3rem],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avatar_group_defaults() {
        let group = AvatarGroup::new();
        assert!(group.avatars.is_empty());
        assert!(group.max_visible.is_none());
    }

    #[test]
    fn test_avatar_group_tokens() {
        let tokens = AvatarGroup::compute_tokens();
        assert!(tokens.contains(&St::DisplayFlex));
        assert!(tokens.contains(&St::ItemsCenter));
    }

    #[test]
    fn test_avatar_group_with_avatars() {
        let group = AvatarGroup::new()
            .avatar(Avatar::new().fallback("A"))
            .avatar(Avatar::new().fallback("B"))
            .avatar(Avatar::new().fallback("C"))
            .max_visible(2);
        assert_eq!(group.avatars.len(), 3);
        assert_eq!(group.max_visible, Some(2));
    }

    #[test]
    fn test_avatar_group_size() {
        let group = AvatarGroup::new().size(AvatarSize::Lg);
        assert_eq!(group.size, AvatarSize::Lg);
    }
}
