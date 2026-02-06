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

pub use alert::{Alert, AlertIntent};
pub use avatar::{Avatar, AvatarSize};
pub use badge::{Badge, BadgeIntent};
pub use breadcrumb::{Breadcrumb, BreadcrumbItem};
pub use button::{Button, ButtonIntent, ButtonSize};
pub use card::{Card, CardPadding, CardShadow};
pub use checkbox::Checkbox;
pub use container::{Container, ContainerSize};
pub use divider::{Divider, SpacingSize};
pub use form_field::FormField;
pub use input::{Input, InputSize, InputType};
pub use label::Label;
pub use link::Link;
pub use list::{List, ListItem};
pub use modal::{Modal, ModalSize};
pub use pagination::Pagination;
pub use progress::Progress;
pub use radio::Radio;
pub use select::{Select, SelectOption};
pub use spacer::Spacer;
pub use spinner::{Spinner, SpinnerSize};
pub use stack::{Gap, Stack, StackAlign, StackDirection, StackJustify};
pub use switch::Switch;
pub use table::{Table, TableRow};
pub use tabs::{Tab, Tabs};
pub use text::{Text, TextColor, TextVariant};
pub use textarea::Textarea;
pub use theme_toggle::{ThemeToggle, ThemeToggleMode, ToggleSize};
pub use utils::{
    backdrop, class_if, combine_classes, focus_trap, portal_container, sr_only, transition_class,
    unique_id, AriaAttrs, TransitionState, Z_DROPDOWN, Z_FIXED, Z_MODAL,
    Z_MODAL_BACKDROP, Z_POPOVER, Z_STICKY, Z_TOAST, Z_TOOLTIP,
};
