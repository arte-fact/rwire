//! Shared utilities for rwire components.
//!
//! This module provides common helpers used across component implementations:
//! - Z-index constants for layering
//! - Unique ID generation
//! - ARIA attribute helpers
//! - Overlay/backdrop utilities
//! - Animation/transition helpers

use crate::{el, El, ElementBuilder};
use std::sync::atomic::{AtomicU32, Ordering};

// ============================================================================
// CSS
// ============================================================================

/// CSS for utility classes (icons, screen reader only, etc.)
pub const UTILS_CSS: &str = "\
/* Icon styles */
.rw-icon {
  display: inline-block;
  vertical-align: middle;
  flex-shrink: 0;
}

.rw-icon-sm {
  width: 16px;
  height: 16px;
}

.rw-icon-lg {
  width: 32px;
  height: 32px;
}

/* Screen reader only */
.rw-sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border-width: 0;
}

/* Hidden utility */
.rw-hidden {
  display: none !important;
}

/* Portal container */
.rw-portal {
  position: fixed;
  top: 0;
  left: 0;
  pointer-events: none;
}

.rw-portal > * {
  pointer-events: auto;
}
";

// ============================================================================
// Z-Index Constants
// ============================================================================

/// Z-index for dropdown menus
pub const Z_DROPDOWN: &str = "1000";

/// Z-index for sticky elements
pub const Z_STICKY: &str = "1100";

/// Z-index for fixed position elements
pub const Z_FIXED: &str = "1200";

/// Z-index for modal backdrops
pub const Z_MODAL_BACKDROP: &str = "1300";

/// Z-index for modal content
pub const Z_MODAL: &str = "1400";

/// Z-index for popovers
pub const Z_POPOVER: &str = "1500";

/// Z-index for tooltips
pub const Z_TOOLTIP: &str = "1600";

/// Z-index for toast notifications
pub const Z_TOAST: &str = "1700";

// ============================================================================
// ID Generation
// ============================================================================

static ID_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Generates a unique ID for component instances.
///
/// This is useful for linking labels to inputs, ARIA relationships, etc.
///
/// # Example
///
/// ```rust
/// use rwire::components::utils::unique_id;
///
/// let input_id = unique_id("input");
/// // Returns something like "input-42"
/// ```
pub fn unique_id(prefix: &str) -> String {
    let id = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{}", prefix, id)
}

/// Resets the ID counter (useful for testing).
#[cfg(test)]
pub fn reset_id_counter() {
    ID_COUNTER.store(0, Ordering::Relaxed);
}

// ============================================================================
// ARIA Helpers
// ============================================================================

/// Adds ARIA live region attributes for dynamic content.
///
/// # Example
///
/// ```rust
/// use rwire::{el, El};
/// use rwire::components::utils::aria_live;
///
/// let alert = el(El::Div)
///     .attr("aria-live", "polite")
///     .attr("aria-relevant", "assertive");
/// ```
///
/// Note: This function is deprecated. Use `.attr()` directly instead.
#[deprecated(note = "Use .attr() directly instead")]
pub fn aria_live(politeness: &str, relevant: &str) -> String {
    format!("aria-live: {}; aria-relevant: {}", politeness, relevant)
}

/// Creates a visually hidden element for screen readers.
///
/// # Example
///
/// ```rust
/// use rwire::{el, El};
/// use rwire::components::utils::sr_only;
///
/// let label = sr_only("Loading...");
/// ```
pub fn sr_only(text: &str) -> ElementBuilder {
    el(El::Span)
        .class("rw-sr-only")
        .text(text)
}

/// Helper to apply common ARIA attributes to a builder.
pub trait AriaAttrs {
    /// Sets aria-label
    fn aria_label(self, label: &str) -> Self;

    /// Sets aria-labelledby
    fn aria_labelledby(self, id: &str) -> Self;

    /// Sets aria-describedby
    fn aria_describedby(self, id: &str) -> Self;

    /// Sets aria-hidden
    fn aria_hidden(self, hidden: bool) -> Self;

    /// Sets aria-expanded
    fn aria_expanded(self, expanded: bool) -> Self;

    /// Sets aria-controls
    fn aria_controls(self, id: &str) -> Self;

    /// Sets aria-pressed
    fn aria_pressed(self, pressed: bool) -> Self;

    /// Sets aria-disabled
    fn aria_disabled(self, disabled: bool) -> Self;

    /// Sets role
    fn role(self, role: &str) -> Self;
}

impl AriaAttrs for ElementBuilder {
    fn aria_label(self, label: &str) -> Self {
        self.attr("aria-label", label)
    }

    fn aria_labelledby(self, id: &str) -> Self {
        self.attr("aria-labelledby", id)
    }

    fn aria_describedby(self, id: &str) -> Self {
        self.attr("aria-describedby", id)
    }

    fn aria_hidden(self, hidden: bool) -> Self {
        self.attr("aria-hidden", if hidden { "true" } else { "false" })
    }

    fn aria_expanded(self, expanded: bool) -> Self {
        self.attr("aria-expanded", if expanded { "true" } else { "false" })
    }

    fn aria_controls(self, id: &str) -> Self {
        self.attr("aria-controls", id)
    }

    fn aria_pressed(self, pressed: bool) -> Self {
        self.attr("aria-pressed", if pressed { "true" } else { "false" })
    }

    fn aria_disabled(self, disabled: bool) -> Self {
        self.attr("aria-disabled", if disabled { "true" } else { "false" })
    }

    fn role(self, role: &str) -> Self {
        self.attr("role", role)
    }
}

// ============================================================================
// Overlay & Backdrop Helpers
// ============================================================================

/// Creates a backdrop element for modals, drawers, etc.
///
/// # Arguments
///
/// * `class` - CSS class name (e.g., "rw-modal-backdrop")
/// * `z_index` - Z-index constant (e.g., Z_MODAL_BACKDROP)
/// * `visible` - Whether the backdrop is visible
///
/// # Example
///
/// ```rust
/// use rwire::components::utils::{backdrop, Z_MODAL_BACKDROP};
///
/// let modal_backdrop = backdrop("rw-modal-backdrop", Z_MODAL_BACKDROP, true);
/// ```
pub fn backdrop(class: &str, z_index: &str, visible: bool) -> ElementBuilder {
    let mut classes = class.to_string();
    if !visible {
        classes.push_str(" rw-hidden");
    }

    el(El::Div)
        .class(&classes)
        .attr("style", &format!("z-index: {}", z_index))
        .aria_hidden(!visible)
}

/// Creates a focus trap container for accessible modals/dialogs.
///
/// # Example
///
/// ```rust
/// use rwire::{el, El};
/// use rwire::components::utils::focus_trap;
///
/// let modal_content = focus_trap("rw-modal-content", vec![
///     el(El::H2).text("Modal Title"),
///     el(El::P).text("Modal body..."),
/// ]);
/// ```
pub fn focus_trap(class: &str, children: Vec<ElementBuilder>) -> ElementBuilder {
    el(El::Div)
        .class(class)
        .attr("tabindex", "-1")
        .append(children)
}

// ============================================================================
// Animation Helpers
// ============================================================================

/// Transition state for animations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionState {
    /// Element is entering
    Enter,
    /// Element is leaving
    Leave,
    /// Element is idle (fully visible)
    Idle,
}

/// Applies transition classes based on state.
///
/// # Example
///
/// ```rust
/// use rwire::{el, El};
/// use rwire::components::utils::{transition_class, TransitionState};
///
/// let state = TransitionState::Enter;
/// let class = transition_class("rw-modal", state);
/// // Returns "rw-modal rw-modal-enter"
/// ```
pub fn transition_class(base: &str, state: TransitionState) -> String {
    match state {
        TransitionState::Enter => format!("{} {}-enter", base, base),
        TransitionState::Leave => format!("{} {}-leave", base, base),
        TransitionState::Idle => base.to_string(),
    }
}

// ============================================================================
// Keyboard Navigation Helpers
// ============================================================================

/// Common keyboard event keys
pub mod keys {
    pub const ENTER: &str = "Enter";
    pub const SPACE: &str = " ";
    pub const ESCAPE: &str = "Escape";
    pub const TAB: &str = "Tab";
    pub const ARROW_UP: &str = "ArrowUp";
    pub const ARROW_DOWN: &str = "ArrowDown";
    pub const ARROW_LEFT: &str = "ArrowLeft";
    pub const ARROW_RIGHT: &str = "ArrowRight";
    pub const HOME: &str = "Home";
    pub const END: &str = "End";
}

// ============================================================================
// Portal Helpers
// ============================================================================

/// Creates a portal target container for modals, tooltips, etc.
///
/// This should be rendered at the root level and used as a mount point
/// for absolutely positioned overlays.
///
/// # Example
///
/// ```rust
/// use rwire::components::utils::portal_container;
///
/// let portal = portal_container("rw-portal");
/// ```
pub fn portal_container(id: &str) -> ElementBuilder {
    el(El::Div)
        .attr("id", id)
        .class("rw-portal")
        .attr("style", "position: fixed; top: 0; left: 0; z-index: 9999;")
}

// ============================================================================
// CSS Class Combinators
// ============================================================================

/// Combines multiple CSS classes, filtering out empty strings.
///
/// # Example
///
/// ```rust
/// use rwire::components::utils::combine_classes;
///
/// let class = combine_classes(&[
///     "rw-button",
///     "rw-button-primary",
///     "", // ignored
/// ]);
/// // Returns "rw-button rw-button-primary"
/// ```
pub fn combine_classes(classes: &[&str]) -> String {
    classes
        .iter()
        .filter(|c| !c.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Conditionally includes a class name.
///
/// # Example
///
/// ```rust
/// use rwire::components::utils::class_if;
///
/// let is_active = true;
/// let class = class_if("active", is_active);
/// // Returns "active"
///
/// let is_disabled = false;
/// let class = class_if("disabled", is_disabled);
/// // Returns ""
/// ```
pub fn class_if(class: &str, condition: bool) -> &str {
    if condition { class } else { "" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unique_id_generation() {
        reset_id_counter();
        assert_eq!(unique_id("test"), "test-0");
        assert_eq!(unique_id("test"), "test-1");
        assert_eq!(unique_id("other"), "other-2");
    }

    #[test]
    fn test_combine_classes() {
        assert_eq!(
            combine_classes(&["a", "b", "c"]),
            "a b c"
        );
        assert_eq!(
            combine_classes(&["a", "", "c"]),
            "a c"
        );
        assert_eq!(
            combine_classes(&[]),
            ""
        );
    }

    #[test]
    fn test_class_if() {
        assert_eq!(class_if("active", true), "active");
        assert_eq!(class_if("active", false), "");
    }

    #[test]
    fn test_transition_class() {
        assert_eq!(
            transition_class("modal", TransitionState::Enter),
            "modal modal-enter"
        );
        assert_eq!(
            transition_class("modal", TransitionState::Leave),
            "modal modal-leave"
        );
        assert_eq!(
            transition_class("modal", TransitionState::Idle),
            "modal"
        );
    }

    #[test]
    fn test_aria_attrs() {
        let builder = el(El::Div)
            .aria_label("Test label")
            .aria_hidden(true)
            .aria_expanded(false);

        // Basic smoke test - just ensure it compiles
        drop(builder);
    }

    #[test]
    fn test_backdrop_visible() {
        let backdrop = backdrop("rw-backdrop", Z_MODAL_BACKDROP, true);
        drop(backdrop);
    }

    #[test]
    fn test_backdrop_hidden() {
        let backdrop = backdrop("rw-backdrop", Z_MODAL_BACKDROP, false);
        drop(backdrop);
    }
}
