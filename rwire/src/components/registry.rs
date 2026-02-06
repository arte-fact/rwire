//! Component registry for CSS tree-shaking.
//!
//! Tracks which components are used in the application to generate
//! minimal CSS containing only the styles actually needed.
//!
//! # How It Works
//!
//! 1. Before building the app tree, call `begin_tracking()`
//! 2. Components register themselves when `.build()` is called
//! 3. After building, call `end_tracking()` to get the registry
//! 4. Pass the registry to capsule generation for tree-shaken CSS
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::registry::{begin_tracking, end_tracking};
//!
//! begin_tracking();
//! let tree = app(); // Components register during build
//! let registry = end_tracking();
//!
//! // Generate CSS only for used components
//! let css = registry.generate_css();
//! ```

use std::cell::RefCell;
use std::collections::HashSet;

use super::{
    ALERT_CSS, AVATAR_CSS, BADGE_CSS, BREADCRUMB_CSS, BUTTON_CSS, CARD_CSS, CHECKBOX_CSS,
    CONTAINER_CSS, DIVIDER_CSS, FORM_FIELD_CSS, INPUT_CSS, LABEL_CSS, LINK_CSS, LIST_CSS,
    MODAL_CSS, PAGINATION_CSS, PROGRESS_CSS, RADIO_CSS, SELECT_CSS, SPACER_CSS, SPINNER_CSS,
    STACK_CSS, SWITCH_CSS, TABLE_CSS, TABS_CSS, TEXT_CSS, TEXTAREA_CSS,
};

/// Component types that have associated CSS.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ComponentType {
    Alert,
    Avatar,
    Badge,
    Breadcrumb,
    Button,
    Card,
    Checkbox,
    Container,
    Divider,
    FormField,
    Input,
    Label,
    Link,
    List,
    Modal,
    Pagination,
    Progress,
    Radio,
    Select,
    Spacer,
    Spinner,
    Stack,
    Switch,
    Table,
    Tabs,
    Text,
    Textarea,
    ThemeToggle,
}

/// Registry of used components.
#[derive(Clone, Debug, Default)]
pub struct ComponentRegistry {
    used: HashSet<ComponentType>,
}

impl ComponentRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a component type as used.
    pub fn mark_used(&mut self, component: ComponentType) {
        self.used.insert(component);
    }

    /// Check if a component is used.
    pub fn is_used(&self, component: ComponentType) -> bool {
        self.used.contains(&component)
    }

    /// Get all used component types.
    pub fn used_components(&self) -> impl Iterator<Item = ComponentType> + '_ {
        self.used.iter().copied()
    }

    /// Get the count of used components.
    pub fn len(&self) -> usize {
        self.used.len()
    }

    /// Check if no components are used.
    pub fn is_empty(&self) -> bool {
        self.used.is_empty()
    }

    /// Generate CSS for all used components.
    ///
    /// Returns a string containing only the CSS for components
    /// that were actually used in the application.
    pub fn generate_css(&self) -> String {
        if self.used.is_empty() {
            return String::new();
        }

        let mut css = String::with_capacity(8192);

        // Generate CSS in a consistent order (alphabetical)
        if self.used.contains(&ComponentType::Alert) {
            css.push_str(ALERT_CSS);
        }
        if self.used.contains(&ComponentType::Avatar) {
            css.push_str(AVATAR_CSS);
        }
        if self.used.contains(&ComponentType::Badge) {
            css.push_str(BADGE_CSS);
        }
        if self.used.contains(&ComponentType::Breadcrumb) {
            css.push_str(BREADCRUMB_CSS);
        }
        if self.used.contains(&ComponentType::Button) {
            css.push_str(BUTTON_CSS);
        }
        if self.used.contains(&ComponentType::Card) {
            css.push_str(CARD_CSS);
        }
        if self.used.contains(&ComponentType::Checkbox) {
            css.push_str(CHECKBOX_CSS);
        }
        if self.used.contains(&ComponentType::Container) {
            css.push_str(CONTAINER_CSS);
        }
        if self.used.contains(&ComponentType::Divider) {
            css.push_str(DIVIDER_CSS);
        }
        if self.used.contains(&ComponentType::FormField) {
            css.push_str(FORM_FIELD_CSS);
        }
        if self.used.contains(&ComponentType::Input) {
            css.push_str(INPUT_CSS);
        }
        if self.used.contains(&ComponentType::Label) {
            css.push_str(LABEL_CSS);
        }
        if self.used.contains(&ComponentType::Link) {
            css.push_str(LINK_CSS);
        }
        if self.used.contains(&ComponentType::List) {
            css.push_str(LIST_CSS);
        }
        if self.used.contains(&ComponentType::Modal) {
            css.push_str(MODAL_CSS);
        }
        if self.used.contains(&ComponentType::Pagination) {
            css.push_str(PAGINATION_CSS);
        }
        if self.used.contains(&ComponentType::Progress) {
            css.push_str(PROGRESS_CSS);
        }
        if self.used.contains(&ComponentType::Radio) {
            css.push_str(RADIO_CSS);
        }
        if self.used.contains(&ComponentType::Select) {
            css.push_str(SELECT_CSS);
        }
        if self.used.contains(&ComponentType::Spacer) {
            css.push_str(SPACER_CSS);
        }
        if self.used.contains(&ComponentType::Spinner) {
            css.push_str(SPINNER_CSS);
        }
        if self.used.contains(&ComponentType::Stack) {
            css.push_str(STACK_CSS);
        }
        if self.used.contains(&ComponentType::Switch) {
            css.push_str(SWITCH_CSS);
        }
        if self.used.contains(&ComponentType::Table) {
            css.push_str(TABLE_CSS);
        }
        if self.used.contains(&ComponentType::Tabs) {
            css.push_str(TABS_CSS);
        }
        if self.used.contains(&ComponentType::Text) {
            css.push_str(TEXT_CSS);
        }
        if self.used.contains(&ComponentType::Textarea) {
            css.push_str(TEXTAREA_CSS);
        }
        if self.used.contains(&ComponentType::ThemeToggle) {
            css.push_str(super::THEME_TOGGLE_CSS);
        }

        css
    }

    /// Get the total CSS size for used components.
    pub fn css_size(&self) -> usize {
        self.generate_css().len()
    }

    /// Print a budget report showing CSS size per component.
    pub fn print_budget_report(&self) {
        println!("Component CSS Budget Report");
        println!("===========================");

        let mut total = 0;

        if self.is_used(ComponentType::Alert) {
            let size = ALERT_CSS.len();
            println!("  Alert:      {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Avatar) {
            let size = AVATAR_CSS.len();
            println!("  Avatar:     {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Badge) {
            let size = BADGE_CSS.len();
            println!("  Badge:      {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Breadcrumb) {
            let size = BREADCRUMB_CSS.len();
            println!("  Breadcrumb: {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Button) {
            let size = BUTTON_CSS.len();
            println!("  Button:     {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Card) {
            let size = CARD_CSS.len();
            println!("  Card:       {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Checkbox) {
            let size = CHECKBOX_CSS.len();
            println!("  Checkbox:   {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Container) {
            let size = CONTAINER_CSS.len();
            println!("  Container:  {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Divider) {
            let size = DIVIDER_CSS.len();
            println!("  Divider:    {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::FormField) {
            let size = FORM_FIELD_CSS.len();
            println!("  FormField:  {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Input) {
            let size = INPUT_CSS.len();
            println!("  Input:      {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Label) {
            let size = LABEL_CSS.len();
            println!("  Label:      {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Link) {
            let size = LINK_CSS.len();
            println!("  Link:       {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::List) {
            let size = LIST_CSS.len();
            println!("  List:       {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Modal) {
            let size = MODAL_CSS.len();
            println!("  Modal:      {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Pagination) {
            let size = PAGINATION_CSS.len();
            println!("  Pagination: {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Progress) {
            let size = PROGRESS_CSS.len();
            println!("  Progress:   {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Radio) {
            let size = RADIO_CSS.len();
            println!("  Radio:      {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Select) {
            let size = SELECT_CSS.len();
            println!("  Select:     {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Spacer) {
            let size = SPACER_CSS.len();
            println!("  Spacer:     {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Spinner) {
            let size = SPINNER_CSS.len();
            println!("  Spinner:    {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Stack) {
            let size = STACK_CSS.len();
            println!("  Stack:      {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Switch) {
            let size = SWITCH_CSS.len();
            println!("  Switch:     {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Table) {
            let size = TABLE_CSS.len();
            println!("  Table:      {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Tabs) {
            let size = TABS_CSS.len();
            println!("  Tabs:       {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Text) {
            let size = TEXT_CSS.len();
            println!("  Text:       {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::Textarea) {
            let size = TEXTAREA_CSS.len();
            println!("  Textarea:   {:>5} bytes", size);
            total += size;
        }
        if self.is_used(ComponentType::ThemeToggle) {
            let size = super::THEME_TOGGLE_CSS.len();
            println!("  ThemeToggle:{:>5} bytes", size);
            total += size;
        }

        println!("===========================");
        println!("  Total:      {:>5} bytes", total);
    }
}

// Thread-local storage for component tracking during build
thread_local! {
    static REGISTRY: RefCell<Option<ComponentRegistry>> = const { RefCell::new(None) };
}

/// Begin tracking component usage.
///
/// Call this before building your application tree.
pub fn begin_tracking() {
    REGISTRY.with(|r| {
        *r.borrow_mut() = Some(ComponentRegistry::new());
    });
}

/// Stop tracking and return the registry.
///
/// Returns the registry with all components that were used since
/// `begin_tracking()` was called.
pub fn end_tracking() -> ComponentRegistry {
    REGISTRY.with(|r| r.borrow_mut().take().unwrap_or_default())
}

/// Mark a component as used (called by component builders).
///
/// This is called internally by component `.build()` methods.
pub fn mark_component_used(component: ComponentType) {
    REGISTRY.with(|r| {
        if let Some(ref mut registry) = *r.borrow_mut() {
            registry.mark_used(component);
        }
    });
}

/// Check if tracking is currently active.
pub fn is_tracking() -> bool {
    REGISTRY.with(|r| r.borrow().is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_empty() {
        let registry = ComponentRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.generate_css().is_empty());
    }

    #[test]
    fn test_registry_mark_used() {
        let mut registry = ComponentRegistry::new();
        registry.mark_used(ComponentType::Button);
        registry.mark_used(ComponentType::Input);

        assert!(registry.is_used(ComponentType::Button));
        assert!(registry.is_used(ComponentType::Input));
        assert!(!registry.is_used(ComponentType::Stack));
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_registry_generate_css() {
        let mut registry = ComponentRegistry::new();
        registry.mark_used(ComponentType::Button);

        let css = registry.generate_css();
        assert!(css.contains(".rw-btn"));
        assert!(!css.contains(".rw-input"));
    }

    #[test]
    fn test_tracking_lifecycle() {
        begin_tracking();

        mark_component_used(ComponentType::Button);
        mark_component_used(ComponentType::Stack);

        let registry = end_tracking();
        assert!(registry.is_used(ComponentType::Button));
        assert!(registry.is_used(ComponentType::Stack));
        assert!(!registry.is_used(ComponentType::Input));
    }

    #[test]
    fn test_tracking_inactive() {
        // When tracking is not active, mark_component_used is a no-op
        mark_component_used(ComponentType::Button);

        begin_tracking();
        let registry = end_tracking();

        // Should not contain Button since it was marked before tracking started
        assert!(!registry.is_used(ComponentType::Button));
    }
}
