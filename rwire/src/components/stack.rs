//! Stack layout component.
//!
//! Flexbox-based layout with configurable direction and spacing.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::{Stack, Gap};
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

use crate::{el, El, ElementBuilder};
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

    fn compute_class(&self) -> String {
        let mut classes = String::with_capacity(64);
        classes.push_str("rw-stack");

        if self.direction == StackDirection::Row {
            classes.push_str(" rw-stack-row");
        }

        match self.gap {
            Gap::None => classes.push_str(" rw-gap-0"),
            Gap::Xs => classes.push_str(" rw-gap-xs"),
            Gap::Sm => classes.push_str(" rw-gap-sm"),
            Gap::Md => {} // Default
            Gap::Lg => classes.push_str(" rw-gap-lg"),
            Gap::Xl => classes.push_str(" rw-gap-xl"),
        }

        match self.align {
            StackAlign::Stretch => {}
            StackAlign::Start => classes.push_str(" rw-items-start"),
            StackAlign::Center => classes.push_str(" rw-items-center"),
            StackAlign::End => classes.push_str(" rw-items-end"),
        }

        match self.justify {
            StackJustify::Start => {}
            StackJustify::Center => classes.push_str(" rw-justify-center"),
            StackJustify::End => classes.push_str(" rw-justify-end"),
            StackJustify::Between => classes.push_str(" rw-justify-between"),
            StackJustify::Around => classes.push_str(" rw-justify-around"),
        }

        if self.wrap {
            classes.push_str(" rw-flex-wrap");
        }

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the stack into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        // Register for CSS tree-shaking
        super::registry::mark_component_used(super::registry::ComponentType::Stack);

        let class = self.compute_class();
        let mut builder = el(El::Div).class(&class);

        for child in self.children {
            builder = builder.append([child]);
        }

        builder
    }
}

/// Stack CSS.
pub const STACK_CSS: &str = "\
.rw-stack{display:flex;flex-direction:column;gap:var(--rw-space-4)}\
.rw-stack-row{flex-direction:row}\
.rw-gap-0{gap:0}.rw-gap-xs{gap:var(--rw-space-1)}.rw-gap-sm{gap:var(--rw-space-2)}\
.rw-gap-lg{gap:var(--rw-space-6)}.rw-gap-xl{gap:var(--rw-space-8)}\
.rw-items-start{align-items:flex-start}.rw-items-center{align-items:center}.rw-items-end{align-items:flex-end}\
.rw-justify-center{justify-content:center}.rw-justify-end{justify-content:flex-end}\
.rw-justify-between{justify-content:space-between}.rw-justify-around{justify-content:space-around}\
.rw-flex-wrap{flex-wrap:wrap}\n";

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
    fn test_stack_class_default() {
        let stack = Stack::new();
        assert_eq!(stack.compute_class(), "rw-stack");
    }

    #[test]
    fn test_stack_class_row() {
        let stack = Stack::row();
        assert_eq!(stack.compute_class(), "rw-stack rw-stack-row");
    }

    #[test]
    fn test_stack_class_full() {
        let stack = Stack::row()
            .gap(Gap::Lg)
            .align(StackAlign::Center)
            .justify(StackJustify::Between)
            .wrap(true);

        let class = stack.compute_class();
        assert!(class.contains("rw-stack"));
        assert!(class.contains("rw-stack-row"));
        assert!(class.contains("rw-gap-lg"));
        assert!(class.contains("rw-items-center"));
        assert!(class.contains("rw-justify-between"));
        assert!(class.contains("rw-flex-wrap"));
    }

    #[test]
    fn test_stack_centered() {
        let stack = Stack::centered();
        assert_eq!(stack.align, StackAlign::Center);
        assert_eq!(stack.justify, StackJustify::Center);
        let class = stack.compute_class();
        assert!(class.contains("rw-items-center"));
        assert!(class.contains("rw-justify-center"));
    }

    #[test]
    fn test_stack_align_center_shorthand() {
        let stack = Stack::column().align_center();
        assert_eq!(stack.align, StackAlign::Center);
        assert!(stack.compute_class().contains("rw-items-center"));
    }

    #[test]
    fn test_stack_css_size() {
        assert!(STACK_CSS.len() < 600, "Stack CSS too large: {} bytes", STACK_CSS.len());
        println!("Stack CSS size: {} bytes", STACK_CSS.len());
    }
}
