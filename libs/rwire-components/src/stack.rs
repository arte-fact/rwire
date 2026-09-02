//! Stack layout component.
//!
//! Flexbox-based layout with configurable direction and spacing.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{Stack, Gap};
//!
//! Stack::column()
//!     .gap(Gap::Lg)
//!     .children([
//!         child1.build(),
//!         child2.build(),
//!     ])
//!     .build()
//!
//! Stack::row()
//!     .gap(Gap::Sm)
//!     .justify(StackJustify::Between)
//!     .build()
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Stack direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StackDirection {
    /// Vertical stack (flex-direction: column)
    #[default]
    Column,
    /// Horizontal stack (flex-direction: row)
    Row,
}

/// Stack alignment (cross-axis).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StackAlign {
    /// Items stretch to fill container (default)
    #[default]
    Stretch,
    /// Items align to start
    Start,
    /// Items align to center
    Center,
    /// Items align to end
    End,
}

/// Stack justify (main-axis).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StackJustify {
    /// Items pack at start (default)
    #[default]
    Start,
    /// Items pack at center
    Center,
    /// Items pack at end
    End,
    /// Items distributed with space between
    Between,
    /// Items distributed with space around
    Around,
}

/// Gap size using spacing tokens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Gap {
    /// No gap
    None,
    /// Extra small (space-1 = 4px)
    Xs,
    /// Small (space-2 = 8px)
    Sm,
    /// Medium (space-4 = 16px) - default
    #[default]
    Md,
    /// Large (space-6 = 24px)
    Lg,
    /// Extra large (space-8 = 32px)
    Xl,
}

/// Stack layout builder.
#[derive(Clone, Default)]
pub struct Stack {
    direction: StackDirection,
    gap: Gap,
    align: StackAlign,
    justify: StackJustify,
    wrap: bool,
    extra_class: Option<Cow<'static, str>>,
    children: Vec<ElementBuilder>,
}

#[rwire::component]
impl Stack {
    /// Create a new vertical stack.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a horizontal stack (row).
    pub fn row() -> Self {
        Self {
            direction: StackDirection::Row,
            ..Self::default()
        }
    }

    /// Create a vertical stack (column).
    pub fn column() -> Self {
        Self::default()
    }

    /// Create a centered stack (centers content both horizontally and vertically).
    pub fn centered() -> Self {
        Self {
            align: StackAlign::Center,
            justify: StackJustify::Center,
            ..Self::default()
        }
    }

    /// Set the stack direction.
    pub fn direction(mut self, direction: StackDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Set the gap between items.
    pub fn gap(mut self, gap: Gap) -> Self {
        self.gap = gap;
        self
    }

    /// Set cross-axis alignment.
    pub fn align(mut self, align: StackAlign) -> Self {
        self.align = align;
        self
    }

    /// Center items on cross-axis (shorthand for align(StackAlign::Center)).
    pub fn align_center(self) -> Self {
        self.align(StackAlign::Center)
    }

    /// Set main-axis justification.
    pub fn justify(mut self, justify: StackJustify) -> Self {
        self.justify = justify;
        self
    }

    /// Enable flex wrap.
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Add children to the stack.
    pub fn children(mut self, children: impl IntoIterator<Item = ElementBuilder>) -> Self {
        self.children.extend(children);
        self
    }

    /// Add a single child.
    pub fn child(mut self, child: ElementBuilder) -> Self {
        self.children.push(child);
        self
    }

    /// Compute style tokens for this stack configuration.
    pub fn compute_tokens(&self) -> Vec<St> {
        let mut tokens = vec![St::DisplayFlex];

        tokens.push(match self.direction {
            StackDirection::Column => St::FlexCol,
            StackDirection::Row => St::FlexRow,
        });

        match self.gap {
            Gap::None => tokens.push(St::Gap0),
            Gap::Xs => tokens.push(St::GapXs),
            Gap::Sm => tokens.push(St::GapSm),
            Gap::Md => tokens.push(St::GapMd),
            Gap::Lg => tokens.push(St::GapLg),
            Gap::Xl => tokens.push(St::GapXl),
        }

        match self.align {
            StackAlign::Stretch => {} // CSS default
            StackAlign::Start => tokens.push(St::ItemsStart),
            StackAlign::Center => tokens.push(St::ItemsCenter),
            StackAlign::End => tokens.push(St::ItemsEnd),
        }

        match self.justify {
            StackJustify::Start => {} // CSS default
            StackJustify::Center => tokens.push(St::JustifyCenter),
            StackJustify::End => tokens.push(St::JustifyEnd),
            StackJustify::Between => tokens.push(St::JustifyBetween),
            StackJustify::Around => tokens.push(St::JustifyAround),
        }

        if self.wrap {
            tokens.push(St::FlexWrap);
        }

        tokens
    }

    /// Build the stack into an ElementBuilder.
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
    fn test_stack_defaults() {
        let stack = Stack::new();
        assert_eq!(stack.direction, StackDirection::Column);
        assert_eq!(stack.gap, Gap::Md);
    }

    #[test]
    fn test_stack_default_tokens() {
        let stack = Stack::new();
        let tokens = stack.compute_tokens();
        assert!(tokens.contains(&St::DisplayFlex));
        assert!(tokens.contains(&St::FlexCol));
        assert!(tokens.contains(&St::GapMd));
    }

    #[test]
    fn test_stack_row_tokens() {
        let stack = Stack::row();
        let tokens = stack.compute_tokens();
        assert!(tokens.contains(&St::FlexRow));
        assert!(!tokens.contains(&St::FlexCol));
    }

    #[test]
    fn test_stack_full_tokens() {
        let stack = Stack::row()
            .gap(Gap::Lg)
            .align(StackAlign::Center)
            .justify(StackJustify::Between)
            .wrap(true);

        let tokens = stack.compute_tokens();
        assert!(tokens.contains(&St::DisplayFlex));
        assert!(tokens.contains(&St::FlexRow));
        assert!(tokens.contains(&St::GapLg));
        assert!(tokens.contains(&St::ItemsCenter));
        assert!(tokens.contains(&St::JustifyBetween));
        assert!(tokens.contains(&St::FlexWrap));
    }

    #[test]
    fn test_stack_centered() {
        let stack = Stack::centered();
        assert_eq!(stack.align, StackAlign::Center);
        assert_eq!(stack.justify, StackJustify::Center);
        let tokens = stack.compute_tokens();
        assert!(tokens.contains(&St::ItemsCenter));
        assert!(tokens.contains(&St::JustifyCenter));
    }

    #[test]
    fn test_stack_align_center_shorthand() {
        let stack = Stack::column().align_center();
        assert_eq!(stack.align, StackAlign::Center);
        assert!(stack.compute_tokens().contains(&St::ItemsCenter));
    }
}
