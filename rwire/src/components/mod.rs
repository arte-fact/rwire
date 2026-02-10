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

mod accordion;
mod alert;
mod app_shell;
mod avatar;
mod avatar_group;
mod badge;
mod blockquote;
mod breadcrumb;
mod button;
mod card;
mod checkbox;
mod code;
mod container;
mod copy_button;
mod divider;
mod drawer;
mod dropdown;
mod empty_state;
mod footer;
mod form_field;
mod grid;
mod image;
mod input;
mod kbd;
mod label;
mod link;
mod list;
mod modal;
mod nav_menu;
mod pagination;
mod progress;
mod radio;
mod select;
mod skeleton;
mod slider;
mod spacer;
mod spinner;
mod stack;
mod stat;
mod stepper;
mod switch;
mod table;
mod tabs;
mod tag;
mod text;
mod textarea;
mod theme_toggle;
mod timeline;
mod toast;
mod tooltip;
pub mod utils;

pub use accordion::{Accordion, AccordionItem};
pub use alert::{Alert, AlertIntent};
pub use app_shell::AppShell;
pub use avatar::{Avatar, AvatarSize};
pub use avatar_group::AvatarGroup;
pub use badge::{Badge, BadgeIntent};
pub use blockquote::Blockquote;
pub use breadcrumb::{Breadcrumb, BreadcrumbItem};
pub use button::{Button, ButtonIntent, ButtonSize};
pub use card::{Card, CardPadding, CardShadow};
pub use checkbox::Checkbox;
pub use code::{Code, CodeMode};
pub use container::{Container, ContainerSize};
pub use copy_button::{CopyButton, COPY_BUTTON_CSS};
pub use divider::{Divider, SpacingSize};
pub use drawer::{Drawer, DrawerPosition};
pub use dropdown::{DropdownItem, DropdownMenu};
pub use empty_state::EmptyState;
pub use footer::{Footer, FooterColumn};
pub use form_field::FormField;
pub use grid::{Grid, GridColumns};
pub use image::{Image, ImageAspect, ImageFit};
pub use input::{Input, InputSize, InputType};
pub use kbd::Kbd;
pub use label::Label;
pub use link::Link;
pub use list::{List, ListItem};
pub use modal::{Modal, ModalSize};
pub use nav_menu::{NavItem, NavMenu};
pub use pagination::Pagination;
pub use progress::Progress;
pub use radio::Radio;
pub use select::{Select, SelectOption};
pub use skeleton::{Skeleton, SkeletonShape};
pub use slider::Slider;
pub use spacer::Spacer;
pub use spinner::{Spinner, SpinnerSize};
pub use stack::{Gap, Stack, StackAlign, StackDirection, StackJustify};
pub use stat::{Stat, StatTrend};
pub use stepper::Stepper;
pub use switch::Switch;
pub use table::{Table, TableRow};
pub use tabs::{Tab, Tabs};
pub use tag::{Tag, TagIntent};
pub use text::{Text, TextColor, TextVariant};
pub use textarea::Textarea;
pub use theme_toggle::{ThemeToggle, ThemeToggleMode, ToggleSize};
pub use timeline::{Timeline, TimelineItem};
pub use toast::{Toast, ToastContainer, ToastIntent};
pub use tooltip::{Tooltip, TooltipPosition};
pub use utils::{
    backdrop, class_if, combine_classes, focus_trap, portal_container, sr_only, transition_class,
    unique_id, AriaAttrs, TransitionState, Z_DROPDOWN, Z_FIXED, Z_MODAL,
    Z_MODAL_BACKDROP, Z_POPOVER, Z_STICKY, Z_TOAST, Z_TOOLTIP,
};
