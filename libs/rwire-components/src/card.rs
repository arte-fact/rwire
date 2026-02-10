//! Card component.
//!
//! Surface container with padding, border, and shadow.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{Card, CardPadding};
//!
//! Card::new()
//!     .padding(CardPadding::Lg)
//!     .child(content)
//!     .build()
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Card padding.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CardPadding {
    /// No padding
    None,
    /// Small padding (space-2)
    Sm,
    /// Medium padding (space-4) - default
    #[default]
    Md,
    /// Large padding (space-6)
    Lg,
}

/// Card shadow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CardShadow {
    /// No shadow
    None,
    /// Small shadow (default)
    #[default]
    Sm,
    /// Medium shadow
    Md,
    /// Large shadow
    Lg,
}

/// Card builder.
#[derive(Clone, Default)]
pub struct Card {
    padding: CardPadding,
    shadow: CardShadow,
    bordered: bool,
    extra_class: Option<Cow<'static, str>>,
    children: Vec<ElementBuilder>,
}

impl Card {
    /// Create a new card with default settings.
    pub fn new() -> Self {
        Self {
            bordered: true,
            ..Self::default()
        }
    }

    /// Set the padding.
    pub fn padding(mut self, padding: CardPadding) -> Self {
        self.padding = padding;
        self
    }

    /// Set the shadow.
    pub fn shadow(mut self, shadow: CardShadow) -> Self {
        self.shadow = shadow;
        self
    }

    /// Set whether the card has a border.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Add children to the card.
    pub fn children(mut self, children: impl IntoIterator<Item = ElementBuilder>) -> Self {
        self.children.extend(children);
        self
    }

    /// Add a single child.
    pub fn child(mut self, child: ElementBuilder) -> Self {
        self.children.push(child);
        self
    }

    /// Compute style tokens for this card configuration.
    pub fn compute_tokens(&self) -> Vec<St> {
        let mut tokens = vec![St::BgSurfaceRaised, St::RoundedLg];

        if self.bordered {
            tokens.push(St::BorderSubtle);
        }

        match self.padding {
            CardPadding::None => tokens.push(St::P0),
            CardPadding::Sm => tokens.push(St::PSm),
            CardPadding::Md => tokens.push(St::PMd),
            CardPadding::Lg => tokens.push(St::PLg),
        }

        match self.shadow {
            CardShadow::None => tokens.push(St::ShadowNone),
            CardShadow::Sm => tokens.push(St::ShadowTheme),
            CardShadow::Md => tokens.push(St::ShadowMd),
            CardShadow::Lg => tokens.push(St::ShadowLg),
        }

        // Theme-aware hooks for Glass/Neon support
        tokens.extend([
            St::BackdropTheme, St::OpacityTheme, St::BorderCTheme,
            St::GlowTheme, St::TextShadowTheme, St::TransTheme,
        ]);

        tokens
    }

    /// Build the card into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut builder = el(El::Div).st(self.compute_tokens());

        if let Some(ref extra) = self.extra_class {
            builder = builder.class(extra.as_ref());
        }

        for child in self.children {
            builder = builder.append([child]);
        }

        builder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_defaults() {
        let card = Card::new();
        assert_eq!(card.padding, CardPadding::Md);
        assert_eq!(card.shadow, CardShadow::Sm);
        assert!(card.bordered);
    }

    #[test]
    fn test_card_default_tokens() {
        let card = Card::new();
        let tokens = card.compute_tokens();
        assert!(tokens.contains(&St::BgSurfaceRaised));
        assert!(tokens.contains(&St::RoundedLg));
        assert!(tokens.contains(&St::BorderSubtle));
        assert!(tokens.contains(&St::PMd));
        assert!(tokens.contains(&St::ShadowTheme));
    }

    #[test]
    fn test_card_full_tokens() {
        let card = Card::new()
            .padding(CardPadding::Lg)
            .shadow(CardShadow::Lg)
            .bordered(false);

        let tokens = card.compute_tokens();
        assert!(tokens.contains(&St::PLg));
        assert!(tokens.contains(&St::ShadowLg));
        assert!(!tokens.contains(&St::BorderSubtle));
    }

}
