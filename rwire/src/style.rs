//! CSS styling utilities for rwire.
//!
//! Provides inline style building and CSS generation.
//!
//! # Example
//!
//! ```ignore
//! use rwire::style::Style;
//!
//! fn styled_button() -> ElementBuilder {
//!     el(El::Button)
//!         .style(Style::new()
//!             .background("#007bff")
//!             .color("white")
//!             .padding("0.5rem 1rem")
//!             .border_radius("4px"))
//!         .text("Click Me")
//! }
//! ```

use std::collections::HashMap;

/// Builder for inline CSS styles.
#[derive(Clone, Debug, Default)]
pub struct Style {
    properties: HashMap<String, String>,
}

impl Style {
    /// Create a new empty style builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a CSS property.
    pub fn set(mut self, property: &str, value: &str) -> Self {
        self.properties.insert(property.to_string(), value.to_string());
        self
    }

    // ========================================================================
    // Layout
    // ========================================================================

    /// Set display property.
    pub fn display(self, value: &str) -> Self {
        self.set("display", value)
    }

    /// Set position property.
    pub fn position(self, value: &str) -> Self {
        self.set("position", value)
    }

    /// Set width property.
    pub fn width(self, value: &str) -> Self {
        self.set("width", value)
    }

    /// Set height property.
    pub fn height(self, value: &str) -> Self {
        self.set("height", value)
    }

    /// Set min-width property.
    pub fn min_width(self, value: &str) -> Self {
        self.set("min-width", value)
    }

    /// Set max-width property.
    pub fn max_width(self, value: &str) -> Self {
        self.set("max-width", value)
    }

    /// Set min-height property.
    pub fn min_height(self, value: &str) -> Self {
        self.set("min-height", value)
    }

    /// Set max-height property.
    pub fn max_height(self, value: &str) -> Self {
        self.set("max-height", value)
    }

    // ========================================================================
    // Flexbox
    // ========================================================================

    /// Set flex property.
    pub fn flex(self, value: &str) -> Self {
        self.set("flex", value)
    }

    /// Set flex-direction property.
    pub fn flex_direction(self, value: &str) -> Self {
        self.set("flex-direction", value)
    }

    /// Set flex-wrap property.
    pub fn flex_wrap(self, value: &str) -> Self {
        self.set("flex-wrap", value)
    }

    /// Set justify-content property.
    pub fn justify_content(self, value: &str) -> Self {
        self.set("justify-content", value)
    }

    /// Set align-items property.
    pub fn align_items(self, value: &str) -> Self {
        self.set("align-items", value)
    }

    /// Set align-self property.
    pub fn align_self(self, value: &str) -> Self {
        self.set("align-self", value)
    }

    /// Set gap property.
    pub fn gap(self, value: &str) -> Self {
        self.set("gap", value)
    }

    // ========================================================================
    // Grid
    // ========================================================================

    /// Set grid-template-columns property.
    pub fn grid_template_columns(self, value: &str) -> Self {
        self.set("grid-template-columns", value)
    }

    /// Set grid-template-rows property.
    pub fn grid_template_rows(self, value: &str) -> Self {
        self.set("grid-template-rows", value)
    }

    /// Set grid-column property.
    pub fn grid_column(self, value: &str) -> Self {
        self.set("grid-column", value)
    }

    /// Set grid-row property.
    pub fn grid_row(self, value: &str) -> Self {
        self.set("grid-row", value)
    }

    // ========================================================================
    // Spacing
    // ========================================================================

    /// Set margin property.
    pub fn margin(self, value: &str) -> Self {
        self.set("margin", value)
    }

    /// Set margin-top property.
    pub fn margin_top(self, value: &str) -> Self {
        self.set("margin-top", value)
    }

    /// Set margin-right property.
    pub fn margin_right(self, value: &str) -> Self {
        self.set("margin-right", value)
    }

    /// Set margin-bottom property.
    pub fn margin_bottom(self, value: &str) -> Self {
        self.set("margin-bottom", value)
    }

    /// Set margin-left property.
    pub fn margin_left(self, value: &str) -> Self {
        self.set("margin-left", value)
    }

    /// Set padding property.
    pub fn padding(self, value: &str) -> Self {
        self.set("padding", value)
    }

    /// Set padding-top property.
    pub fn padding_top(self, value: &str) -> Self {
        self.set("padding-top", value)
    }

    /// Set padding-right property.
    pub fn padding_right(self, value: &str) -> Self {
        self.set("padding-right", value)
    }

    /// Set padding-bottom property.
    pub fn padding_bottom(self, value: &str) -> Self {
        self.set("padding-bottom", value)
    }

    /// Set padding-left property.
    pub fn padding_left(self, value: &str) -> Self {
        self.set("padding-left", value)
    }

    // ========================================================================
    // Colors
    // ========================================================================

    /// Set color property.
    pub fn color(self, value: &str) -> Self {
        self.set("color", value)
    }

    /// Set background property.
    pub fn background(self, value: &str) -> Self {
        self.set("background", value)
    }

    /// Set background-color property.
    pub fn background_color(self, value: &str) -> Self {
        self.set("background-color", value)
    }

    /// Set opacity property.
    pub fn opacity(self, value: &str) -> Self {
        self.set("opacity", value)
    }

    // ========================================================================
    // Typography
    // ========================================================================

    /// Set font-size property.
    pub fn font_size(self, value: &str) -> Self {
        self.set("font-size", value)
    }

    /// Set font-weight property.
    pub fn font_weight(self, value: &str) -> Self {
        self.set("font-weight", value)
    }

    /// Set font-family property.
    pub fn font_family(self, value: &str) -> Self {
        self.set("font-family", value)
    }

    /// Set line-height property.
    pub fn line_height(self, value: &str) -> Self {
        self.set("line-height", value)
    }

    /// Set text-align property.
    pub fn text_align(self, value: &str) -> Self {
        self.set("text-align", value)
    }

    /// Set text-decoration property.
    pub fn text_decoration(self, value: &str) -> Self {
        self.set("text-decoration", value)
    }

    /// Set text-transform property.
    pub fn text_transform(self, value: &str) -> Self {
        self.set("text-transform", value)
    }

    /// Set letter-spacing property.
    pub fn letter_spacing(self, value: &str) -> Self {
        self.set("letter-spacing", value)
    }

    // ========================================================================
    // Borders
    // ========================================================================

    /// Set border property.
    pub fn border(self, value: &str) -> Self {
        self.set("border", value)
    }

    /// Set border-width property.
    pub fn border_width(self, value: &str) -> Self {
        self.set("border-width", value)
    }

    /// Set border-style property.
    pub fn border_style(self, value: &str) -> Self {
        self.set("border-style", value)
    }

    /// Set border-color property.
    pub fn border_color(self, value: &str) -> Self {
        self.set("border-color", value)
    }

    /// Set border-radius property.
    pub fn border_radius(self, value: &str) -> Self {
        self.set("border-radius", value)
    }

    // ========================================================================
    // Effects
    // ========================================================================

    /// Set box-shadow property.
    pub fn box_shadow(self, value: &str) -> Self {
        self.set("box-shadow", value)
    }

    /// Set transform property.
    pub fn transform(self, value: &str) -> Self {
        self.set("transform", value)
    }

    /// Set transition property.
    pub fn transition(self, value: &str) -> Self {
        self.set("transition", value)
    }

    /// Set cursor property.
    pub fn cursor(self, value: &str) -> Self {
        self.set("cursor", value)
    }

    /// Set overflow property.
    pub fn overflow(self, value: &str) -> Self {
        self.set("overflow", value)
    }

    /// Set z-index property.
    pub fn z_index(self, value: &str) -> Self {
        self.set("z-index", value)
    }

    // ========================================================================
    // Conversion
    // ========================================================================

    /// Convert to CSS string.
    pub fn to_css(&self) -> String {
        self.properties
            .iter()
            .map(|(k, v)| format!("{}:{}", k, v))
            .collect::<Vec<_>>()
            .join(";")
    }

    /// Check if style is empty.
    pub fn is_empty(&self) -> bool {
        self.properties.is_empty()
    }
}

impl std::fmt::Display for Style {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_css())
    }
}

/// A scoped CSS class with generated styles.
#[derive(Clone, Debug)]
pub struct ScopedClass {
    /// The generated class name.
    pub class_name: String,
    /// The CSS rules for this class.
    pub css: String,
}

impl ScopedClass {
    /// Create a new scoped class with the given CSS.
    pub fn new(css: &str) -> Self {
        // Generate a unique class name based on the CSS content hash
        let hash = css
            .bytes()
            .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
        let class_name = format!("rw-{:x}", hash);

        Self {
            class_name,
            css: css.to_string(),
        }
    }

    /// Get the full CSS rule with selector.
    pub fn to_css_rule(&self) -> String {
        format!(".{}{{{}}}", self.class_name, self.css)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_style_basic() {
        let style = Style::new()
            .color("red")
            .background("blue");

        let css = style.to_css();
        assert!(css.contains("color:red"));
        assert!(css.contains("background:blue"));
    }

    #[test]
    fn test_style_empty() {
        let style = Style::new();
        assert!(style.is_empty());
        assert_eq!(style.to_css(), "");
    }

    #[test]
    fn test_style_complex() {
        let style = Style::new()
            .display("flex")
            .flex_direction("column")
            .padding("1rem")
            .margin("0 auto")
            .border_radius("4px")
            .box_shadow("0 2px 4px rgba(0,0,0,0.1)");

        let css = style.to_css();
        assert!(css.contains("display:flex"));
        assert!(css.contains("flex-direction:column"));
        assert!(css.contains("padding:1rem"));
    }

    #[test]
    fn test_scoped_class() {
        let class = ScopedClass::new("color:red;font-size:16px");

        assert!(class.class_name.starts_with("rw-"));
        assert!(class.to_css_rule().contains(&class.class_name));
        assert!(class.to_css_rule().contains("color:red"));
    }

    #[test]
    fn test_scoped_class_unique() {
        let class1 = ScopedClass::new("color:red");
        let class2 = ScopedClass::new("color:blue");
        let class3 = ScopedClass::new("color:red");

        assert_ne!(class1.class_name, class2.class_name);
        assert_eq!(class1.class_name, class3.class_name); // Same CSS = same name
    }
}
