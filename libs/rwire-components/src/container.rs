//! Container component.
//!
//! Responsive width container with max-width constraints.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{Container, ContainerSize};
//!
//! Container::new()
//!     .size(ContainerSize::Md)
//!     .centered(true)
//!     .child(content)
//!     .build()
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
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

#[rwire::component]
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

    /// Compute style tokens for this container configuration.
    pub fn compute_tokens(&self) -> Vec<St> {
        let mut tokens = vec![St::WFull];

        if self.centered {
            tokens.push(St::MxAuto);
        }

        if self.padding {
            tokens.push(St::PxMd);
        }

        tokens
    }

    /// Get the max-width token for this size.
    fn max_width_token(&self) -> Option<St> {
        match self.size {
            ContainerSize::Sm => Some(St::MaxW40rem),
            ContainerSize::Md => Some(St::MaxW48rem),
            ContainerSize::Lg => Some(St::MaxW64rem),
            ContainerSize::Xl => Some(St::MaxW80rem),
            ContainerSize::Full => None,
        }
    }

    /// Build the container into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut tokens = self.compute_tokens();
        if let Some(mw) = self.max_width_token() {
            tokens.push(mw);
        }
        let mut builder = el(El::Div).st(tokens);

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
    fn test_container_defaults() {
        let container = Container::new();
        assert_eq!(container.size, ContainerSize::Md);
        assert!(container.centered);
        assert!(container.padding);
    }

    #[test]
    fn test_container_default_tokens() {
        let container = Container::new();
        let tokens = container.compute_tokens();
        assert!(tokens.contains(&St::WFull));
        assert!(tokens.contains(&St::MxAuto));
        assert!(tokens.contains(&St::PxMd));
    }

    #[test]
    fn test_container_no_center_no_padding() {
        let container = Container::new().centered(false).padding(false);

        let tokens = container.compute_tokens();
        assert!(tokens.contains(&St::WFull));
        assert!(!tokens.contains(&St::MxAuto));
        assert!(!tokens.contains(&St::PxMd));
    }

    #[test]
    fn test_container_max_width_token() {
        assert_eq!(
            Container::new().size(ContainerSize::Sm).max_width_token(),
            Some(St::MaxW40rem)
        );
        assert_eq!(
            Container::new().size(ContainerSize::Md).max_width_token(),
            Some(St::MaxW48rem)
        );
        assert_eq!(
            Container::new().size(ContainerSize::Lg).max_width_token(),
            Some(St::MaxW64rem)
        );
        assert_eq!(
            Container::new().size(ContainerSize::Xl).max_width_token(),
            Some(St::MaxW80rem)
        );
        assert_eq!(
            Container::new().size(ContainerSize::Full).max_width_token(),
            None
        );
    }
}
