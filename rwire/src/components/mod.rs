//! rwire component library.
//!
//! Pre-built components with design token integration and type-safe variants.
//!
//! # Philosophy
//!
//! Components are builder structs that produce `ElementBuilder` instances.
//! They provide:
//! - Sensible defaults (primary button, medium size)
//! - Type-safe variants (no typos in class strings)
//! - Escape hatches (`.class()` for custom overrides)
//!
//! # Available Components
//!
//! - **Button**: Primary, Secondary, Ghost, Destructive buttons
//! - **Input**: Text, password, email, number inputs
//! - **Textarea**: Multi-line text input
//! - **Label**: Form labels
//! - **Checkbox**: Boolean checkbox input
//! - **Radio**: Radio button input
//! - **Switch**: Toggle switch input
//! - **Select**: Dropdown select with options
//! - **FormField**: Form field wrapper with label/help/error
//! - **Avatar**: User avatar with image or fallback
//! - **Progress**: Progress bar
//! - **Spinner**: Loading spinner
//! - **Table**: Div-based table
//! - **Alert**: Alert messages with intent
//! - **Breadcrumb**: Navigation breadcrumb trail
//! - **Tabs**: Tab navigation
//! - **Pagination**: Page navigation
//! - **Stack**: Flexbox layout (row/column)
//! - **Card**: Surface container with padding/shadow
//! - **Badge**: Status indicators
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::*;
//!
//! Stack::column()
//!     .gap(Gap::Lg)
//!     .children([
//!         Card::new()
//!             .child(
//!                 Stack::row()
//!                     .gap(Gap::Sm)
//!                     .children([
//!                         Input::text().placeholder("Search...").build(),
//!                         Button::primary("Search").build(),
//!                     ])
//!             )
//!             .build(),
//!         Stack::row()
//!             .gap(Gap::Sm)
//!             .children([
//!                 Badge::success("Active").build(),
//!                 Badge::warning("Pending").build(),
//!             ])
//!             .build(),
//!     ])
//!     .build()
//! ```

mod alert;
mod avatar;
mod badge;
mod breadcrumb;
mod button;
mod card;
mod checkbox;
mod container;
mod divider;
mod form_field;
mod input;
mod label;
mod link;
mod list;
mod modal;
mod pagination;
mod progress;
mod radio;
pub mod registry;
mod select;
mod spacer;
mod spinner;
mod stack;
mod switch;
mod table;
mod tabs;
mod text;
mod textarea;
mod theme_toggle;
pub mod utils;

pub use alert::{Alert, AlertIntent, ALERT_CSS};
pub use avatar::{Avatar, AvatarSize, AVATAR_CSS};
pub use badge::{Badge, BadgeIntent, BADGE_CSS};
pub use breadcrumb::{Breadcrumb, BreadcrumbItem, BREADCRUMB_CSS};
pub use button::{Button, ButtonIntent, ButtonSize, BUTTON_CSS};
pub use card::{Card, CardPadding, CardShadow, CARD_CSS};
pub use checkbox::{Checkbox, CHECKBOX_CSS};
pub use container::{Container, ContainerSize, CONTAINER_CSS};
pub use divider::{Divider, SpacingSize, DIVIDER_CSS};
pub use form_field::{FormField, FORM_FIELD_CSS};
pub use input::{Input, InputSize, InputType, INPUT_CSS};
pub use label::{Label, LABEL_CSS};
pub use link::{Link, LINK_CSS};
pub use list::{List, ListItem, LIST_CSS};
pub use modal::{Modal, ModalSize, MODAL_CSS};
pub use pagination::{Pagination, PAGINATION_CSS};
pub use progress::{Progress, PROGRESS_CSS};
pub use radio::{Radio, RADIO_CSS};
pub use registry::{begin_tracking, end_tracking, ComponentRegistry, ComponentType};
pub use select::{Select, SelectOption, SELECT_CSS};
pub use spacer::{Spacer, SPACER_CSS};
pub use spinner::{Spinner, SpinnerSize, SPINNER_CSS};
pub use stack::{Gap, Stack, StackAlign, StackDirection, StackJustify, STACK_CSS};
pub use switch::{Switch, SWITCH_CSS};
pub use table::{Table, TableRow, TABLE_CSS};
pub use tabs::{Tab, Tabs, TABS_CSS};
pub use text::{Text, TextColor, TextVariant, TEXT_CSS};
pub use textarea::{Textarea, TEXTAREA_CSS};
pub use theme_toggle::{ThemeToggle, ThemeToggleMode, ToggleSize, THEME_TOGGLE_CSS};
pub use utils::{
    backdrop, class_if, combine_classes, focus_trap, portal_container, sr_only, transition_class,
    unique_id, AriaAttrs, TransitionState, UTILS_CSS, Z_DROPDOWN, Z_FIXED, Z_MODAL,
    Z_MODAL_BACKDROP, Z_POPOVER, Z_STICKY, Z_TOAST, Z_TOOLTIP,
};

/// Generate CSS for all components.
///
/// In the future, this will be tree-shaken to only include
/// CSS for components actually used in the application.
pub fn generate_components_css() -> String {
    let mut css = String::with_capacity(12288);
    css.push_str(UTILS_CSS);
    css.push_str(ALERT_CSS);
    css.push_str(AVATAR_CSS);
    css.push_str(BADGE_CSS);
    css.push_str(BREADCRUMB_CSS);
    css.push_str(BUTTON_CSS);
    css.push_str(CARD_CSS);
    css.push_str(CHECKBOX_CSS);
    css.push_str(CONTAINER_CSS);
    css.push_str(DIVIDER_CSS);
    css.push_str(FORM_FIELD_CSS);
    css.push_str(INPUT_CSS);
    css.push_str(LABEL_CSS);
    css.push_str(LINK_CSS);
    css.push_str(LIST_CSS);
    css.push_str(MODAL_CSS);
    css.push_str(PAGINATION_CSS);
    css.push_str(PROGRESS_CSS);
    css.push_str(RADIO_CSS);
    css.push_str(SELECT_CSS);
    css.push_str(SPACER_CSS);
    css.push_str(SPINNER_CSS);
    css.push_str(STACK_CSS);
    css.push_str(SWITCH_CSS);
    css.push_str(TABLE_CSS);
    css.push_str(TABS_CSS);
    css.push_str(TEXT_CSS);
    css.push_str(TEXTAREA_CSS);
    css.push_str(THEME_TOGGLE_CSS);
    css
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_components_css_not_empty() {
        let css = generate_components_css();
        assert!(!css.is_empty());
        assert!(css.contains(".rw-btn"));
        assert!(css.contains(".rw-input"));
        assert!(css.contains(".rw-stack"));
        assert!(css.contains(".rw-card"));
        assert!(css.contains(".rw-badge"));
        assert!(css.contains(".rw-alert"));
        assert!(css.contains(".rw-avatar"));
        assert!(css.contains(".rw-checkbox"));
        assert!(css.contains(".rw-select"));
        assert!(css.contains(".rw-theme-toggle"));
        assert!(css.contains(".rw-modal"));
    }

    #[test]
    fn test_total_components_css_size() {
        let css = generate_components_css();
        // Total component CSS should be under 20KB (realistic for full component library)
        assert!(
            css.len() < 20480,
            "Total component CSS too large: {} bytes",
            css.len()
        );
        println!("Total component CSS size: {} bytes", css.len());
    }
}
