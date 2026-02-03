//! Card component.
//!
//! Surface container with padding, border, and shadow.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::{Card, CardPadding};
//!
//! Card::new()
//!     .padding(CardPadding::Lg)
//!     .child(content)
//!     .build()
//! ```

use crate::{el, El, ElementBuilder};
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

    fn compute_class(&self) -> String {
        let mut classes = String::with_capacity(48);
        classes.push_str("rw-card");

        match self.padding {
            CardPadding::None => classes.push_str(" rw-p-0"),
            CardPadding::Sm => classes.push_str(" rw-p-sm"),
            CardPadding::Md => {}
            CardPadding::Lg => classes.push_str(" rw-p-lg"),
        }

        match self.shadow {
            CardShadow::None => classes.push_str(" rw-shadow-none"),
            CardShadow::Sm => {}
            CardShadow::Md => classes.push_str(" rw-shadow-md"),
            CardShadow::Lg => classes.push_str(" rw-shadow-lg"),
        }

        if !self.bordered {
            classes.push_str(" rw-border-none");
        }

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the card into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let class = self.compute_class();
        let mut builder = el(El::Div).class(&class);

        for child in self.children {
            builder = builder.append([child]);
        }

        builder
    }
}

/// Card CSS.
pub const CARD_CSS: &str = "\
.rw-card{background:var(--rw-bg-app);border:1px solid var(--rw-border-subtle);\
border-radius:var(--rw-radius-lg);padding:var(--rw-space-4);box-shadow:var(--rw-shadow-sm)}\
.rw-p-0{padding:0}.rw-p-sm{padding:var(--rw-space-2)}.rw-p-lg{padding:var(--rw-space-6)}\
.rw-shadow-none{box-shadow:none}.rw-shadow-md{box-shadow:var(--rw-shadow-md)}.rw-shadow-lg{box-shadow:var(--rw-shadow-lg)}\
.rw-border-none{border:none}\n";

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
    fn test_card_class_default() {
        let card = Card::new();
        assert_eq!(card.compute_class(), "rw-card");
    }

    #[test]
    fn test_card_class_full() {
        let card = Card::new()
            .padding(CardPadding::Lg)
            .shadow(CardShadow::Lg)
            .bordered(false);

        let class = card.compute_class();
        assert!(class.contains("rw-card"));
        assert!(class.contains("rw-p-lg"));
        assert!(class.contains("rw-shadow-lg"));
        assert!(class.contains("rw-border-none"));
    }

    #[test]
    fn test_card_css_size() {
        assert!(CARD_CSS.len() < 450, "Card CSS too large: {} bytes", CARD_CSS.len());
        println!("Card CSS size: {} bytes", CARD_CSS.len());
    }
}
