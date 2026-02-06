//! Container component.
//!
//! Responsive width container with max-width constraints.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::{Container, ContainerSize};
//!
//! Container::new()
//!     .size(ContainerSize::Md)
//!     .centered(true)
//!     .child(content)
//!     .build()
//! ```

use crate::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Container max-width size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ContainerSize {
    /// Small: max-width 640px
    Sm,
    /// Medium: max-width 768px (default)
    #[default]
    Md,
    /// Large: max-width 1024px
    Lg,
    /// Extra large: max-width 1280px
    Xl,
    /// Full width (no max-width)
    Full,
}

/// Container component builder.
#[derive(Clone, Default)]
pub struct Container {
    size: ContainerSize,
    centered: bool,
    padding: bool,
    extra_class: Option<Cow<'static, str>>,
    children: Vec<ElementBuilder>,
}

impl Container {
    /// Create a new container with default settings.
    pub fn new() -> Self {
        Self {
            padding: true,
            centered: true,
            ..Self::default()
        }
    }

    /// Set the max-width size.
    pub fn size(mut self, size: ContainerSize) -> Self {
        self.size = size;
        self
    }

    /// Set whether the container is centered.
    pub fn centered(mut self, centered: bool) -> Self {
        self.centered = centered;
        self
    }

    /// Set whether to add horizontal padding.
    pub fn padding(mut self, padding: bool) -> Self {
        self.padding = padding;
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Add children to the container.
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
        classes.push_str("rw-container");

        match self.size {
            ContainerSize::Sm => classes.push_str(" rw-container-sm"),
            ContainerSize::Md => {}
            ContainerSize::Lg => classes.push_str(" rw-container-lg"),
            ContainerSize::Xl => classes.push_str(" rw-container-xl"),
            ContainerSize::Full => classes.push_str(" rw-container-full"),
        }

        if !self.centered {
            classes.push_str(" rw-container-left");
        }

        if !self.padding {
            classes.push_str(" rw-container-flush");
        }

        if let Some(ref extra) = self.extra_class {
            classes.push(' ');
            classes.push_str(extra);
        }

        classes
    }

    /// Build the container into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        super::registry::mark_component_used(super::registry::ComponentType::Container);

        let class = self.compute_class();
        let mut builder = el(El::Div).class(&class);

        for child in self.children {
            builder = builder.append([child]);
        }

        builder
    }
}

/// Container CSS.
pub const CONTAINER_CSS: &str = "\
.rw-container{width:100%;max-width:48rem;margin-inline:auto;padding-inline:var(--rw-space-4)}\
.rw-container-sm{max-width:40rem}\
.rw-container-lg{max-width:64rem}\
.rw-container-xl{max-width:80rem}\
.rw-container-full{max-width:none}\
.rw-container-left{margin-inline:0}\
.rw-container-flush{padding-inline:0}\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_defaults() {
        let container = Container::new();
        assert_eq!(container.size, ContainerSize::Md);
        assert!(container.centered);
        assert!(container.padding);
    }

    #[test]
    fn test_container_class_default() {
        let container = Container::new();
        assert_eq!(container.compute_class(), "rw-container");
    }

    #[test]
    fn test_container_class_full() {
        let container = Container::new()
            .size(ContainerSize::Lg)
            .centered(false)
            .padding(false);

        let class = container.compute_class();
        assert!(class.contains("rw-container"));
        assert!(class.contains("rw-container-lg"));
        assert!(class.contains("rw-container-left"));
        assert!(class.contains("rw-container-flush"));
    }

    #[test]
    fn test_container_css_size() {
        assert!(CONTAINER_CSS.len() < 350, "Container CSS too large: {} bytes", CONTAINER_CSS.len());
        println!("Container CSS size: {} bytes", CONTAINER_CSS.len());
    }
}
