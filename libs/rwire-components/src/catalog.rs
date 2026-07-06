//! Component catalog with metadata for auto-generating documentation.
//!
//! Each component self-describes its variants, boolean props, and provides
//! a `build_demo` function that renders a live preview from selections.

use rwire::{el, El, ElementBuilder, Icon};

// ============================================================================
// Types
// ============================================================================

/// Component category for sidebar grouping.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    Layout,
    Forms,
    DataDisplay,
    Feedback,
    Navigation,
    Overlay,
}

impl Category {
    /// All categories in display order.
    pub const ALL: &[Self] = &[
        Self::Layout,
        Self::Forms,
        Self::DataDisplay,
        Self::Feedback,
        Self::Navigation,
        Self::Overlay,
    ];

    /// Human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Layout => "Layout",
            Self::Forms => "Forms",
            Self::DataDisplay => "Data Display",
            Self::Feedback => "Feedback",
            Self::Navigation => "Navigation",
            Self::Overlay => "Overlay",
        }
    }

    /// URL-safe slug.
    pub fn slug(self) -> &'static str {
        match self {
            Self::Layout => "layout",
            Self::Forms => "forms",
            Self::DataDisplay => "data-display",
            Self::Feedback => "feedback",
            Self::Navigation => "navigation",
            Self::Overlay => "overlay",
        }
    }
}

/// One possible value of a variant axis.
pub struct VariantValue {
    pub label: &'static str,
    pub rust_expr: &'static str,
}

/// A variant axis (e.g., "Intent" with values [Primary, Secondary, ...]).
pub struct VariantAxis {
    pub name: &'static str,
    pub display_name: &'static str,
    pub rust_type: &'static str,
    pub values: &'static [VariantValue],
    pub default_index: usize,
}

/// A boolean prop (e.g., "disabled").
pub struct BoolProp {
    pub name: &'static str,
    pub description: &'static str,
    pub default: bool,
}

/// A numeric prop in the playground. `slider: true` renders a range slider (best for visual
/// magnitudes like a progress value); `false` renders a number input (best for discrete counts).
pub struct NumProp {
    pub name: &'static str,
    pub default: i32,
    pub min: i32,
    pub max: i32,
    pub step: i32,
    pub slider: bool,
}

/// A string prop, surfaced as a text input in the playground (e.g., a "label").
pub struct TextProp {
    pub name: &'static str,
    pub default: &'static str,
}

/// All adjustable values for a live demo. Richer than the legacy `(variants, bools)`
/// signature so components can expose numeric (slider) and text params.
pub struct DemoParams<'a> {
    pub variants: &'a [usize],
    pub bools: &'a [bool],
    pub nums: &'a [i32],
    pub texts: &'a [&'a str],
}

/// Numeric (slider) props for a component, by slug. Empty unless the component opts in.
pub fn num_props(slug: &str) -> &'static [NumProp] {
    match slug {
        "progress" => PROGRESS_NUM,
        "slider" => SLIDER_NUM,
        "pagination" => PAGINATION_NUM,
        "textarea" => TEXTAREA_NUM,
        "tabs" => TABS_NUM,
        "stepper" => STEPPER_NUM,
        "app-shell" => APP_SHELL_NUM,
        "avatar-group" => AVATAR_GROUP_NUM,
        "skeleton" => SKELETON_NUM,
        _ => &[],
    }
}

/// Text props for a component, by slug. Empty unless the component opts in.
pub fn text_props(slug: &str) -> &'static [TextProp] {
    match slug {
        "progress" => PROGRESS_TEXT,
        "slider" => SLIDER_TEXT,
        "textarea" => TEXTAREA_TEXT,
        "badge" => BADGE_TEXT,
        "checkbox" => CHECKBOX_TEXT,
        "switch" => SWITCH_TEXT,
        "button" => BUTTON_TEXT,
        "alert" => ALERT_TEXT,
        "modal" => MODAL_TEXT,
        "drawer" => DRAWER_TEXT,
        "empty-state" => EMPTY_STATE_TEXT,
        "input" => INPUT_TEXT,
        "text" => TEXT_TEXT,
        "spinner" => SPINNER_TEXT,
        "avatar" => AVATAR_TEXT,
        "footer" => FOOTER_TEXT,
        "nav-menu" => NAV_MENU_TEXT,
        "select" => SELECT_TEXT,
        "toast" => TOAST_TEXT,
        "tag" => TAG_TEXT,
        "tooltip" => TOOLTIP_TEXT,
        "label" => LABEL_TEXT,
        "kbd" => KBD_TEXT,
        "copy-button" => COPY_BUTTON_TEXT,
        "blockquote" => BLOCKQUOTE_TEXT,
        "link" => LINK_TEXT,
        "stat" => STAT_TEXT,
        "image" => IMAGE_TEXT,
        _ => &[],
    }
}

/// A richer demo builder for components that expose numeric/text params. When present, the
/// playground calls this (with all params) instead of the legacy `build_demo`.
pub fn rich_demo(slug: &str) -> Option<fn(&DemoParams) -> ElementBuilder> {
    match slug {
        "progress" => Some(progress_rich),
        "slider" => Some(slider_rich),
        "pagination" => Some(pagination_rich),
        "textarea" => Some(textarea_rich),
        "badge" => Some(badge_rich),
        "checkbox" => Some(checkbox_rich),
        "switch" => Some(switch_rich),
        "button" => Some(button_rich),
        "alert" => Some(alert_rich),
        "modal" => Some(modal_rich),
        "drawer" => Some(drawer_rich),
        "empty-state" => Some(empty_state_rich),
        "input" => Some(input_rich),
        "tabs" => Some(tabs_rich),
        "stepper" => Some(stepper_rich),
        "app-shell" => Some(app_shell_rich),
        "avatar-group" => Some(avatar_group_rich),
        "skeleton" => Some(skeleton_rich),
        "text" => Some(text_rich),
        "spinner" => Some(spinner_rich),
        "avatar" => Some(avatar_rich),
        "footer" => Some(footer_rich),
        "nav-menu" => Some(nav_menu_rich),
        "select" => Some(select_rich),
        "toast" => Some(toast_rich),
        "tag" => Some(tag_rich),
        "tooltip" => Some(tooltip_rich),
        "label" => Some(label_rich),
        "kbd" => Some(kbd_rich),
        "copy-button" => Some(copy_button_rich),
        "blockquote" => Some(blockquote_rich),
        "link" => Some(link_rich),
        "stat" => Some(stat_rich),
        "image" => Some(image_rich),
        _ => None,
    }
}

const PROGRESS_NUM: &[NumProp] = &[NumProp {
    name: "value",
    default: 65,
    min: 0,
    max: 100,
    step: 1,
    slider: true,
}];
const PROGRESS_TEXT: &[TextProp] = &[TextProp {
    name: "label",
    default: "Upload progress",
}];

fn progress_rich(p: &DemoParams) -> ElementBuilder {
    use crate::Progress;
    let value = p.nums.first().copied().unwrap_or(65).clamp(0, 100) as u32;
    let label = p
        .texts
        .first()
        .copied()
        .unwrap_or("Upload progress")
        .to_string();
    Progress::new().value(value).max(100).label(label).build()
}

const SLIDER_NUM: &[NumProp] = &[
    NumProp {
        name: "value",
        default: 50,
        min: 0,
        max: 100,
        step: 1,
        slider: true,
    },
    NumProp {
        name: "step",
        default: 1,
        min: 1,
        max: 25,
        step: 1,
        slider: false,
    },
];
const SLIDER_TEXT: &[TextProp] = &[TextProp {
    name: "label",
    default: "Volume",
}];
fn slider_rich(p: &DemoParams) -> ElementBuilder {
    use crate::Slider;
    Slider::new()
        .min(0)
        .max(100)
        .value(p.nums.first().copied().unwrap_or(50))
        .step(p.nums.get(1).copied().unwrap_or(1).max(1))
        .label(p.texts.first().copied().unwrap_or("Volume").to_string())
        .disabled(b(p.bools, 0))
        .build()
}

const PAGINATION_NUM: &[NumProp] = &[
    NumProp {
        name: "current_page",
        default: 3,
        min: 1,
        max: 20,
        step: 1,
        slider: true,
    },
    NumProp {
        name: "total_pages",
        default: 10,
        min: 1,
        max: 20,
        step: 1,
        slider: true,
    },
    NumProp {
        name: "max_visible",
        default: 5,
        min: 3,
        max: 9,
        step: 1,
        slider: false,
    },
];
fn pagination_rich(p: &DemoParams) -> ElementBuilder {
    use crate::Pagination;
    Pagination::new()
        .current_page(p.nums.first().copied().unwrap_or(3).max(1) as usize)
        .total_pages(p.nums.get(1).copied().unwrap_or(10).max(1) as usize)
        .max_visible(p.nums.get(2).copied().unwrap_or(5).max(1) as usize)
        .build()
}

const TEXTAREA_NUM: &[NumProp] = &[NumProp {
    name: "rows",
    default: 4,
    min: 1,
    max: 12,
    step: 1,
    slider: false,
}];
const TEXTAREA_TEXT: &[TextProp] = &[TextProp {
    name: "placeholder",
    default: "Enter a description...",
}];
fn textarea_rich(p: &DemoParams) -> ElementBuilder {
    use crate::textarea::TextareaSize;
    use crate::Textarea;
    let size = match v(p.variants, 0) {
        0 => TextareaSize::Sm,
        2 => TextareaSize::Lg,
        _ => TextareaSize::Md,
    };
    Textarea::new()
        .size(size)
        .placeholder(
            p.texts
                .first()
                .copied()
                .unwrap_or("Enter a description...")
                .to_string(),
        )
        .rows(p.nums.first().copied().unwrap_or(4).max(1) as u32)
        .disabled(b(p.bools, 0))
        .readonly(b(p.bools, 1))
        .required(b(p.bools, 2))
        .invalid(b(p.bools, 3))
        .build()
}

const BADGE_TEXT: &[TextProp] = &[TextProp {
    name: "text",
    default: "Badge",
}];
fn badge_rich(p: &DemoParams) -> ElementBuilder {
    use crate::{Badge, BadgeFill, BadgeIntent, BadgeShape};
    let intent = match v(p.variants, 0) {
        1 => BadgeIntent::Primary,
        2 => BadgeIntent::Success,
        3 => BadgeIntent::Warning,
        4 => BadgeIntent::Error,
        _ => BadgeIntent::Default,
    };
    let shape = match v(p.variants, 1) {
        1 => BadgeShape::Square,
        _ => BadgeShape::Pill,
    };
    let fill = match v(p.variants, 2) {
        1 => BadgeFill::Outline,
        _ => BadgeFill::Solid,
    };
    Badge::new()
        .intent(intent)
        .shape(shape)
        .fill(fill)
        .text(p.texts.first().copied().unwrap_or("Badge").to_string())
        .build()
}

const CHECKBOX_TEXT: &[TextProp] = &[TextProp {
    name: "label",
    default: "Accept terms and conditions",
}];
fn checkbox_rich(p: &DemoParams) -> ElementBuilder {
    use crate::Checkbox;
    Checkbox::new()
        .label(
            p.texts
                .first()
                .copied()
                .unwrap_or("Accept terms and conditions")
                .to_string(),
        )
        .checked(b(p.bools, 0))
        .disabled(b(p.bools, 1))
        .required(b(p.bools, 2))
        .invalid(b(p.bools, 3))
        .build()
}

const SWITCH_TEXT: &[TextProp] = &[TextProp {
    name: "label",
    default: "Enable notifications",
}];
fn switch_rich(p: &DemoParams) -> ElementBuilder {
    use crate::Switch;
    Switch::new()
        .label(
            p.texts
                .first()
                .copied()
                .unwrap_or("Enable notifications")
                .to_string(),
        )
        .checked(b(p.bools, 0))
        .disabled(b(p.bools, 1))
        .required(b(p.bools, 2))
        .invalid(b(p.bools, 3))
        .build()
}

const BUTTON_TEXT: &[TextProp] = &[TextProp {
    name: "text",
    default: "Button",
}];
fn button_rich(p: &DemoParams) -> ElementBuilder {
    use crate::{Button, ButtonIntent, ButtonSize};
    let intent = match v(p.variants, 0) {
        1 => ButtonIntent::Secondary,
        2 => ButtonIntent::Ghost,
        3 => ButtonIntent::Destructive,
        _ => ButtonIntent::Primary,
    };
    let size = match v(p.variants, 1) {
        0 => ButtonSize::Sm,
        2 => ButtonSize::Lg,
        _ => ButtonSize::Md,
    };
    let mut btn = Button::new()
        .intent(intent)
        .size(size)
        .text(p.texts.first().copied().unwrap_or("Button").to_string())
        .disabled(b(p.bools, 0))
        .loading(b(p.bools, 1))
        .full_width(b(p.bools, 2));
    if b(p.bools, 3) {
        btn = btn.icon(Icon::Check);
    }
    btn.build()
}

const ALERT_TEXT: &[TextProp] = &[
    TextProp {
        name: "title",
        default: "Alert Title",
    },
    TextProp {
        name: "message",
        default: "This is an alert message with important information.",
    },
];
fn alert_rich(p: &DemoParams) -> ElementBuilder {
    use crate::{Alert, AlertIntent};
    let intent = match v(p.variants, 0) {
        1 => AlertIntent::Success,
        2 => AlertIntent::Warning,
        3 => AlertIntent::Error,
        _ => AlertIntent::Info,
    };
    Alert::new()
        .intent(intent)
        .title(
            p.texts
                .first()
                .copied()
                .unwrap_or("Alert Title")
                .to_string(),
        )
        .message(
            p.texts
                .get(1)
                .copied()
                .unwrap_or("This is an alert message with important information.")
                .to_string(),
        )
        .build()
}

const MODAL_TEXT: &[TextProp] = &[TextProp {
    name: "title",
    default: "Confirm Action",
}];
fn modal_rich(p: &DemoParams) -> ElementBuilder {
    use crate::{Modal, ModalSize};
    let size = match v(p.variants, 0) {
        0 => ModalSize::Sm,
        2 => ModalSize::Lg,
        3 => ModalSize::Xl,
        4 => ModalSize::Full,
        _ => ModalSize::Md,
    };
    Modal::new()
        .title(p.texts.first().copied().unwrap_or("Confirm Action"))
        .size(size)
        .open(b(p.bools, 0))
        .close_on_backdrop_click(b(p.bools, 1))
        .content(el(El::P).text("Are you sure you want to proceed?"))
        .build()
}

const DRAWER_TEXT: &[TextProp] = &[TextProp {
    name: "title",
    default: "Navigation",
}];
fn drawer_rich(p: &DemoParams) -> ElementBuilder {
    use crate::{Drawer, DrawerPosition};
    let position = match v(p.variants, 0) {
        1 => DrawerPosition::Right,
        _ => DrawerPosition::Left,
    };
    Drawer::new()
        .title(p.texts.first().copied().unwrap_or("Navigation").to_string())
        .position(position)
        .open(b(p.bools, 0))
        .content(el(El::P).text("Drawer content goes here."))
        .build()
}

const EMPTY_STATE_TEXT: &[TextProp] = &[
    TextProp {
        name: "title",
        default: "No results found",
    },
    TextProp {
        name: "description",
        default: "Try adjusting your search or filter criteria.",
    },
];
fn empty_state_rich(p: &DemoParams) -> ElementBuilder {
    use crate::EmptyState;
    EmptyState::new()
        .title(
            p.texts
                .first()
                .copied()
                .unwrap_or("No results found")
                .to_string(),
        )
        .description(
            p.texts
                .get(1)
                .copied()
                .unwrap_or("Try adjusting your search or filter criteria.")
                .to_string(),
        )
        .build()
}

const INPUT_TEXT: &[TextProp] = &[
    TextProp {
        name: "placeholder",
        default: "Enter text...",
    },
    TextProp {
        name: "value",
        default: "",
    },
    TextProp {
        name: "autocomplete",
        default: "",
    },
    TextProp {
        name: "pattern",
        default: "",
    },
    // min/max/step apply to number/range inputs (ignored by the browser otherwise).
    TextProp {
        name: "min",
        default: "",
    },
    TextProp {
        name: "max",
        default: "",
    },
    TextProp {
        name: "step",
        default: "",
    },
];
fn input_rich(p: &DemoParams) -> ElementBuilder {
    use crate::{Input, InputSize, InputType};
    let input_type = match v(p.variants, 0) {
        1 => InputType::Password,
        2 => InputType::Email,
        3 => InputType::Number,
        4 => InputType::Search,
        5 => InputType::Tel,
        6 => InputType::Url,
        _ => InputType::Text,
    };
    let size = match v(p.variants, 1) {
        0 => InputSize::Sm,
        2 => InputSize::Lg,
        _ => InputSize::Md,
    };
    let mut input = Input::new()
        .input_type(input_type)
        .size(size)
        .placeholder(
            p.texts
                .first()
                .copied()
                .unwrap_or("Enter text...")
                .to_string(),
        )
        .disabled(b(p.bools, 0))
        .readonly(b(p.bools, 1))
        .required(b(p.bools, 2))
        .invalid(b(p.bools, 3))
        .autofocus(b(p.bools, 4));
    if b(p.bools, 5) {
        input = input.spellcheck(true);
    }
    // Optional string-typed attributes — applied only when provided.
    let txt = |i: usize| p.texts.get(i).copied().unwrap_or("");
    if !txt(1).is_empty() {
        input = input.value(txt(1).to_string());
    }
    if !txt(2).is_empty() {
        input = input.autocomplete(txt(2).to_string());
    }
    if !txt(3).is_empty() {
        input = input.pattern(txt(3).to_string());
    }
    if !txt(4).is_empty() {
        input = input.min(txt(4).to_string());
    }
    if !txt(5).is_empty() {
        input = input.max(txt(5).to_string());
    }
    if !txt(6).is_empty() {
        input = input.step(txt(6).to_string());
    }
    input.build()
}

const TABS_NUM: &[NumProp] = &[NumProp {
    name: "active",
    default: 0,
    min: 0,
    max: 2,
    step: 1,
    slider: false,
}];
fn tabs_rich(p: &DemoParams) -> ElementBuilder {
    use crate::{Tab, Tabs};
    Tabs::new()
        .tab(Tab::new("Overview", el(El::P).text("Overview content")))
        .tab(Tab::new("Settings", el(El::P).text("Settings content")))
        .tab(Tab::new("Activity", el(El::P).text("Activity content")))
        .active(p.nums.first().copied().unwrap_or(0).clamp(0, 2) as usize)
        .build()
}

const STEPPER_NUM: &[NumProp] = &[NumProp {
    name: "current",
    default: 1,
    min: 0,
    max: 2,
    step: 1,
    slider: false,
}];
fn stepper_rich(p: &DemoParams) -> ElementBuilder {
    use crate::Stepper;
    Stepper::new()
        .step("Account")
        .step("Profile")
        .step("Review")
        .current(p.nums.first().copied().unwrap_or(1).clamp(0, 2) as usize)
        .build()
}

const APP_SHELL_NUM: &[NumProp] = &[
    NumProp {
        name: "sidebar_width",
        default: 240,
        min: 120,
        max: 400,
        step: 10,
        slider: true,
    },
    NumProp {
        name: "header_height",
        default: 56,
        min: 40,
        max: 120,
        step: 4,
        slider: true,
    },
];
fn app_shell_rich(p: &DemoParams) -> ElementBuilder {
    use crate::AppShell;
    AppShell::new()
        .header(el(El::Div).st([rwire::St::PMd]).text("Header"))
        .sidebar(el(El::Div).st([rwire::St::PMd]).text("Sidebar"))
        .main(el(El::Div).st([rwire::St::PMd]).text("Main content"))
        .sidebar_width(p.nums.first().copied().unwrap_or(240).max(0) as u16)
        .header_height(p.nums.get(1).copied().unwrap_or(56).max(0) as u16)
        .build()
}

const AVATAR_GROUP_NUM: &[NumProp] = &[NumProp {
    name: "max_visible",
    default: 3,
    min: 1,
    max: 4,
    step: 1,
    slider: false,
}];
fn avatar_group_rich(p: &DemoParams) -> ElementBuilder {
    use crate::{Avatar, AvatarGroup, AvatarSize};
    AvatarGroup::new()
        .avatar(Avatar::new().fallback("AB").size(AvatarSize::Md))
        .avatar(Avatar::new().fallback("CD").size(AvatarSize::Md))
        .avatar(Avatar::new().fallback("EF").size(AvatarSize::Md))
        .avatar(Avatar::new().fallback("GH").size(AvatarSize::Md))
        .max_visible(p.nums.first().copied().unwrap_or(3).clamp(1, 4) as usize)
        .build()
}

const SKELETON_NUM: &[NumProp] = &[NumProp {
    name: "lines",
    default: 3,
    min: 1,
    max: 6,
    step: 1,
    slider: false,
}];
fn skeleton_rich(p: &DemoParams) -> ElementBuilder {
    use crate::Skeleton;
    let lines = p.nums.first().copied().unwrap_or(3).clamp(1, 6) as u8;
    match v(p.variants, 0) {
        1 => Skeleton::circle().build(),
        2 => Skeleton::rect().build(),
        _ => Skeleton::text().lines(lines).build(),
    }
}

const TEXT_TEXT: &[TextProp] = &[TextProp {
    name: "content",
    default: "The quick brown fox jumps over the lazy dog.",
}];
fn text_rich(p: &DemoParams) -> ElementBuilder {
    use crate::{Text, TextColor, TextVariant};
    let variant = match v(p.variants, 0) {
        1 => TextVariant::BodySmall,
        2 => TextVariant::Heading1,
        3 => TextVariant::Heading2,
        4 => TextVariant::Heading3,
        5 => TextVariant::Label,
        6 => TextVariant::Caption,
        _ => TextVariant::Body,
    };
    let color = match v(p.variants, 1) {
        1 => TextColor::High,
        2 => TextColor::Muted,
        3 => TextColor::Accent,
        4 => TextColor::Success,
        5 => TextColor::Warning,
        6 => TextColor::Error,
        _ => TextColor::Default,
    };
    Text::new()
        .variant(variant)
        .color(color)
        .content(
            p.texts
                .first()
                .copied()
                .unwrap_or("The quick brown fox jumps over the lazy dog.")
                .to_string(),
        )
        .build()
}

const SPINNER_TEXT: &[TextProp] = &[TextProp {
    name: "label",
    default: "Loading...",
}];
fn spinner_rich(p: &DemoParams) -> ElementBuilder {
    use crate::{Spinner, SpinnerSize};
    let size = match v(p.variants, 0) {
        0 => SpinnerSize::Sm,
        2 => SpinnerSize::Lg,
        _ => SpinnerSize::Md,
    };
    Spinner::new()
        .size(size)
        .label(p.texts.first().copied().unwrap_or("Loading...").to_string())
        .build()
}

const AVATAR_TEXT: &[TextProp] = &[TextProp {
    name: "fallback",
    default: "JD",
}];
fn avatar_rich(p: &DemoParams) -> ElementBuilder {
    use crate::{Avatar, AvatarSize};
    let size = match v(p.variants, 0) {
        0 => AvatarSize::Sm,
        2 => AvatarSize::Lg,
        _ => AvatarSize::Md,
    };
    Avatar::new()
        .fallback(p.texts.first().copied().unwrap_or("JD").to_string())
        .size(size)
        .build()
}

const FOOTER_TEXT: &[TextProp] = &[
    TextProp {
        name: "tagline",
        default: "Building the future, one component at a time.",
    },
    TextProp {
        name: "copyright",
        default: "2026 Brand Inc.",
    },
];
fn footer_rich(p: &DemoParams) -> ElementBuilder {
    use crate::{Footer, FooterColumn};
    Footer::new()
        .logo(
            el(El::Span)
                .st([rwire::St::TextLg, rwire::St::FontBold])
                .text("Brand"),
        )
        .tagline(
            p.texts
                .first()
                .copied()
                .unwrap_or("Building the future, one component at a time."),
        )
        .column(
            FooterColumn::new("Product")
                .link("Features", "/features")
                .link("Pricing", "/pricing"),
        )
        .column(
            FooterColumn::new("Company")
                .link("About", "/about")
                .link("Blog", "/blog"),
        )
        .copyright(p.texts.get(1).copied().unwrap_or("2026 Brand Inc."))
        .build()
}

const NAV_MENU_TEXT: &[TextProp] = &[TextProp {
    name: "active_path",
    default: "/",
}];
fn nav_menu_rich(p: &DemoParams) -> ElementBuilder {
    use crate::{NavItem, NavMenu};
    NavMenu::new()
        .item(NavItem::new("Home", "/"))
        .item(NavItem::new("About", "/about"))
        .item(NavItem::new("Contact", "/contact"))
        .active_path(p.texts.first().copied().unwrap_or("/").to_string())
        .build()
}

const SELECT_TEXT: &[TextProp] = &[TextProp {
    name: "aria_label",
    default: "Country",
}];
fn select_rich(p: &DemoParams) -> ElementBuilder {
    use crate::Select;
    Select::new()
        .aria_label(p.texts.first().copied().unwrap_or("Country").to_string())
        .option("us", "United States")
        .option("uk", "United Kingdom")
        .option("ca", "Canada")
        .option("au", "Australia")
        .disabled(b(p.bools, 0))
        .required(b(p.bools, 1))
        .invalid(b(p.bools, 2))
        .build()
}

const TOAST_TEXT: &[TextProp] = &[TextProp {
    name: "message",
    default: "Toast notification message",
}];
fn toast_rich(p: &DemoParams) -> ElementBuilder {
    use crate::{Toast, ToastIntent};
    let intent = match v(p.variants, 0) {
        1 => ToastIntent::Success,
        2 => ToastIntent::Warning,
        3 => ToastIntent::Error,
        _ => ToastIntent::Info,
    };
    Toast::new(
        p.texts
            .first()
            .copied()
            .unwrap_or("Toast notification message")
            .to_string(),
    )
    .intent(intent)
    .build()
}

const TAG_TEXT: &[TextProp] = &[TextProp {
    name: "text",
    default: "Tag",
}];
fn tag_rich(p: &DemoParams) -> ElementBuilder {
    use crate::{Tag, TagIntent};
    let intent = match v(p.variants, 0) {
        1 => TagIntent::Primary,
        2 => TagIntent::Success,
        3 => TagIntent::Warning,
        4 => TagIntent::Error,
        _ => TagIntent::Default,
    };
    Tag::new(p.texts.first().copied().unwrap_or("Tag").to_string())
        .intent(intent)
        .removable(b(p.bools, 0))
        .build()
}

const TOOLTIP_TEXT: &[TextProp] = &[TextProp {
    name: "text",
    default: "Tooltip text",
}];
fn tooltip_rich(p: &DemoParams) -> ElementBuilder {
    use crate::{Tooltip, TooltipPosition};
    let position = match v(p.variants, 0) {
        1 => TooltipPosition::Bottom,
        2 => TooltipPosition::Left,
        3 => TooltipPosition::Right,
        _ => TooltipPosition::Top,
    };
    Tooltip::new(
        p.texts
            .first()
            .copied()
            .unwrap_or("Tooltip text")
            .to_string(),
    )
    .position(position)
    .child(
        el(El::Button)
            .st([
                rwire::St::BgPrimary,
                rwire::St::TextOnPrimary,
                rwire::St::PxMd,
                rwire::St::PySm,
                rwire::St::RoundedMd,
                rwire::St::BorderNone,
                rwire::St::CursorPointer,
            ])
            .text("Hover me"),
    )
    .build()
}

const LABEL_TEXT: &[TextProp] = &[TextProp {
    name: "text",
    default: "Email address",
}];
fn label_rich(p: &DemoParams) -> ElementBuilder {
    use crate::Label;
    Label::new(
        p.texts
            .first()
            .copied()
            .unwrap_or("Email address")
            .to_string(),
    )
    .required(b(p.bools, 0))
    .build()
}

const KBD_TEXT: &[TextProp] = &[TextProp {
    name: "key",
    default: "Ctrl",
}];
fn kbd_rich(p: &DemoParams) -> ElementBuilder {
    use crate::Kbd;
    Kbd::new(p.texts.first().copied().unwrap_or("Ctrl").to_string()).build()
}

const COPY_BUTTON_TEXT: &[TextProp] = &[TextProp {
    name: "text",
    default: "cargo add rwire",
}];
fn copy_button_rich(p: &DemoParams) -> ElementBuilder {
    use crate::CopyButton;
    CopyButton::new(p.texts.first().copied().unwrap_or("cargo add rwire")).build()
}

const BLOCKQUOTE_TEXT: &[TextProp] = &[
    TextProp {
        name: "content",
        default: "The best way to predict the future is to invent it.",
    },
    TextProp {
        name: "cite",
        default: "Alan Kay",
    },
];
fn blockquote_rich(p: &DemoParams) -> ElementBuilder {
    use crate::Blockquote;
    Blockquote::new(
        p.texts
            .first()
            .copied()
            .unwrap_or("The best way to predict the future is to invent it.")
            .to_string(),
    )
    .cite(p.texts.get(1).copied().unwrap_or("Alan Kay").to_string())
    .build()
}

const LINK_TEXT: &[TextProp] = &[
    TextProp {
        name: "href",
        default: "/docs",
    },
    TextProp {
        name: "text",
        default: "Internal Link",
    },
];
fn link_rich(p: &DemoParams) -> ElementBuilder {
    use crate::Link;
    let href = p.texts.first().copied().unwrap_or("/docs").to_string();
    let text = p
        .texts
        .get(1)
        .copied()
        .unwrap_or("Internal Link")
        .to_string();
    if b(p.bools, 0) {
        Link::external(href).text(text).build()
    } else {
        Link::new(href).text(text).build()
    }
}

const STAT_TEXT: &[TextProp] = &[
    TextProp {
        name: "value",
        default: "$12,345",
    },
    TextProp {
        name: "label",
        default: "Revenue",
    },
    TextProp {
        name: "description",
        default: "+12.5% from last month",
    },
];
fn stat_rich(p: &DemoParams) -> ElementBuilder {
    use crate::{Stat, StatTrend};
    let trend = match v(p.variants, 0) {
        1 => StatTrend::Down,
        2 => StatTrend::Neutral,
        _ => StatTrend::Up,
    };
    Stat::new(p.texts.first().copied().unwrap_or("$12,345").to_string())
        .label(p.texts.get(1).copied().unwrap_or("Revenue").to_string())
        .description(
            p.texts
                .get(2)
                .copied()
                .unwrap_or("+12.5% from last month")
                .to_string(),
        )
        .trend(trend, "+12.5%")
        .build()
}

const IMAGE_TEXT: &[TextProp] = &[
    TextProp {
        name: "src",
        default: "https://picsum.photos/400/300",
    },
    TextProp {
        name: "alt",
        default: "Sample image",
    },
];
fn image_rich(p: &DemoParams) -> ElementBuilder {
    use crate::{Image, ImageFit};
    let fit = match v(p.variants, 0) {
        1 => ImageFit::Contain,
        2 => ImageFit::Fill,
        3 => ImageFit::None,
        _ => ImageFit::Cover,
    };
    let src = p
        .texts
        .first()
        .copied()
        .unwrap_or("https://picsum.photos/400/300")
        .to_string();
    let alt = p
        .texts
        .get(1)
        .copied()
        .unwrap_or("Sample image")
        .to_string();
    let mut img = Image::new(src).alt(alt).fit(fit);
    match v(p.variants, 1) {
        1 => img = img.aspect_square(),
        2 => img = img.aspect_video(),
        _ => {}
    }
    if b(p.bools, 0) {
        img = img.rounded();
    }
    if b(p.bools, 1) {
        img = img.full_width();
    }
    img.build()
}

/// Full catalog entry for one component.
pub struct ComponentEntry {
    pub name: &'static str,
    pub slug: &'static str,
    pub description: &'static str,
    pub category: Category,
    pub order: u32,
    pub variants: &'static [VariantAxis],
    pub bool_props: &'static [BoolProp],
    pub build_demo: fn(variants: &[usize], bools: &[bool]) -> ElementBuilder,
}

// ============================================================================
// Public API
// ============================================================================

/// Returns the full catalog of all components.
pub fn catalog() -> &'static [ComponentEntry] {
    ENTRIES
}

/// Lookup a single entry by slug.
pub fn find(slug: &str) -> Option<&'static ComponentEntry> {
    ENTRIES.iter().find(|e| e.slug == slug)
}

// ============================================================================
// Helpers
// ============================================================================

fn v(variants: &[usize], axis: usize) -> usize {
    variants.get(axis).copied().unwrap_or(0)
}

fn b(bools: &[bool], idx: usize) -> bool {
    bools.get(idx).copied().unwrap_or(false)
}

// ============================================================================
// Code-snippet generation
// ============================================================================

/// Resolved parameter values for generating the playground's code snippet.
pub struct CodeCtx<'a> {
    pub variants: &'a [usize],
    pub nums: &'a [i32],
    pub texts: &'a [&'a str],
    pub bools: &'a [bool],
}

fn cb(c: &CodeCtx, i: usize) -> bool {
    c.bools.get(i).copied().unwrap_or(false)
}

/// Text param value at index `i`, falling back to `default`.
fn ctext<'a>(c: &'a CodeCtx, i: usize, default: &'a str) -> &'a str {
    c.texts.get(i).copied().unwrap_or(default)
}

/// `rust_expr` for a variant axis when the selection differs from its default (else `None`).
fn vexpr<'a>(entry: &'a ComponentEntry, c: &CodeCtx, axis: usize) -> Option<&'a str> {
    let a = entry.variants.get(axis)?;
    let sel = c.variants.get(axis).copied().unwrap_or(a.default_index);
    if sel != a.default_index {
        a.values.get(sel).map(|v| v.rust_expr)
    } else {
        None
    }
}

/// `rust_expr` for the current selection of a variant axis (always emitted, even at default).
fn vexpr_always<'a>(entry: &'a ComponentEntry, c: &CodeCtx, axis: usize) -> &'a str {
    let Some(a) = entry.variants.get(axis) else {
        return "";
    };
    let sel = c.variants.get(axis).copied().unwrap_or(a.default_index);
    a.values.get(sel).map(|v| v.rust_expr).unwrap_or("")
}

/// Generic chain lines for components with a plain `Name::new()` constructor.
fn generic_chain(entry: &ComponentEntry, c: &CodeCtx) -> Vec<String> {
    let mut lines = Vec::new();
    for (i, axis) in entry.variants.iter().enumerate() {
        let sel = c.variants.get(i).copied().unwrap_or(axis.default_index);
        if sel != axis.default_index {
            if let Some(val) = axis.values.get(sel) {
                lines.push(format!("    .{}({})", axis.name, val.rust_expr));
            }
        }
    }
    for (i, prop) in entry.bool_props.iter().enumerate() {
        let checked = c.bools.get(i).copied().unwrap_or(prop.default);
        if checked != prop.default {
            lines.push(format!("    .{}({})", prop.name, checked));
        }
    }
    // Numeric + text params are the meaningful content of these demos, so always show them.
    for (i, np) in num_props(entry.slug).iter().enumerate() {
        let cur = c.nums.get(i).copied().unwrap_or(np.default);
        lines.push(format!("    .{}({})", np.name, cur));
    }
    for (i, tp) in text_props(entry.slug).iter().enumerate() {
        let cur = c.texts.get(i).copied().unwrap_or(tp.default);
        if !cur.is_empty() {
            lines.push(format!("    .{}({:?})", tp.name, cur));
        }
    }
    lines
}

/// Representative child lines for composite components, so the snippet mirrors the preview
/// (which is built from children the variant/bool/text/number controls can't express). The
/// children are illustrative — they assume `use rwire_components::*; use rwire::{el, El};`.
fn composite_children(slug: &str) -> &'static [&'static str] {
    match slug {
        "tabs" => &[
            "    .tab(Tab::new(\"Overview\", el(El::P).text(\"Overview content\")))",
            "    .tab(Tab::new(\"Settings\", el(El::P).text(\"Settings content\")))",
            "    .tab(Tab::new(\"Activity\", el(El::P).text(\"Activity content\")))",
        ],
        "stepper" => &[
            "    .step(\"Account\")",
            "    .step(\"Profile\")",
            "    .step(\"Review\")",
        ],
        "select" => &[
            "    .option(\"us\", \"United States\")",
            "    .option(\"uk\", \"United Kingdom\")",
            "    .option(\"ca\", \"Canada\")",
            "    .option(\"au\", \"Australia\")",
        ],
        "nav-menu" => &[
            "    .item(NavItem::new(\"Home\", \"/\"))",
            "    .item(NavItem::new(\"About\", \"/about\"))",
            "    .item(NavItem::new(\"Contact\", \"/contact\"))",
        ],
        "footer" => &[
            "    .logo(el(El::Span).text(\"Brand\"))",
            "    .column(FooterColumn::new(\"Product\").link(\"Features\", \"/features\").link(\"Pricing\", \"/pricing\"))",
            "    .column(FooterColumn::new(\"Company\").link(\"About\", \"/about\").link(\"Blog\", \"/blog\"))",
        ],
        "table" => &[
            "    .headers([\"Name\", \"Role\", \"Status\"])",
            "    .row(TableRow::new().cell(\"Alice\").cell(\"Engineer\").cell(\"Active\"))",
            "    .row(TableRow::new().cell(\"Bob\").cell(\"Designer\").cell(\"Active\"))",
            "    .row(TableRow::new().cell(\"Carol\").cell(\"Manager\").cell(\"Away\"))",
        ],
        "timeline" => &[
            "    .item(TimelineItem::new(\"Order placed\").description(\"Jan 1, 2026\").active(true))",
            "    .item(TimelineItem::new(\"Shipped\").description(\"Jan 3, 2026\").active(true))",
            "    .item(TimelineItem::new(\"Delivered\").description(\"Pending\"))",
        ],
        "breadcrumb" => &[
            "    .item(\"Home\", Some(\"/\"))",
            "    .item(\"Components\", Some(\"/components\"))",
            "    .item(\"Button\", None::<&str>)",
        ],
        "accordion" => &[
            "    .item(AccordionItem::new(\"What is rwire?\").content(el(El::P).text(\"A WebSocket-based UI framework.\")).open(true))",
            "    .item(AccordionItem::new(\"How does it work?\").content(el(El::P).text(\"Server renders DOM via binary protocol.\")))",
            "    .item(AccordionItem::new(\"Is it production-ready?\").content(el(El::P).text(\"It is currently in experimental phase.\")))",
        ],
        "dropdown" => &[
            "    .trigger(Button::secondary(\"Options\").build())",
            "    .item(DropdownItem::new(\"Edit\"))",
            "    .item(DropdownItem::new(\"Duplicate\"))",
            "    .item(DropdownItem::new(\"Delete\").destructive())",
        ],
        "grid" => &[
            "    .children([el(El::Div).text(\"1\"), el(El::Div).text(\"2\"), el(El::Div).text(\"3\")])",
        ],
        "list" => &[
            "    .children([ListItem::new(\"First item\").build(), ListItem::new(\"Second item\").build(), ListItem::new(\"Third item\").build()])",
        ],
        "modal" => &["    .content(el(El::P).text(\"Are you sure you want to proceed?\"))"],
        "drawer" => &["    .content(el(El::P).text(\"Drawer content goes here.\"))"],
        "app-shell" => &[
            "    .header(el(El::Div).text(\"Header\"))",
            "    .sidebar(el(El::Div).text(\"Sidebar\"))",
            "    .main(el(El::Div).text(\"Main content\"))",
        ],
        "avatar-group" => &[
            "    .avatar(Avatar::new().fallback(\"AB\").size(AvatarSize::Md))",
            "    .avatar(Avatar::new().fallback(\"CD\").size(AvatarSize::Md))",
            "    .avatar(Avatar::new().fallback(\"EF\").size(AvatarSize::Md))",
            "    .avatar(Avatar::new().fallback(\"GH\").size(AvatarSize::Md))",
        ],
        "card" => &["    .child(el(El::P).text(\"Card content goes here\"))"],
        "container" => &["    .child(el(El::Div).text(\"Container content\"))"],
        _ => &[],
    }
}

/// Generate a copy-pasteable, compiling Rust snippet for the current playground selections.
///
/// Components with required-arg constructors (`Toast::new("…")`), named constructors
/// (`Skeleton::text()`), multi-arg setters (`Stat::trend(dir, "…")`), or no-arg toggles
/// (`Spacer::horizontal()`) are special-cased so the snippet compiles and mirrors the demo.
/// Composite components additionally emit representative children via [`composite_children`].
pub fn generate_code(entry: &ComponentEntry, c: &CodeCtx) -> String {
    let lines: Vec<String> = match entry.slug {
        "toast" => {
            let mut l = vec![format!(
                "Toast::new({:?})",
                ctext(c, 0, "Toast notification message")
            )];
            if let Some(e) = vexpr(entry, c, 0) {
                l.push(format!("    .intent({e})"));
            }
            l
        }
        "tag" => {
            let mut l = vec![format!("Tag::new({:?})", ctext(c, 0, "Tag"))];
            if let Some(e) = vexpr(entry, c, 0) {
                l.push(format!("    .intent({e})"));
            }
            if cb(c, 0) {
                l.push("    .removable(true)".to_string());
            }
            l
        }
        "tooltip" => {
            let mut l = vec![format!("Tooltip::new({:?})", ctext(c, 0, "Tooltip text"))];
            if let Some(e) = vexpr(entry, c, 0) {
                l.push(format!("    .position({e})"));
            }
            l.push("    .child(el(El::Span).text(\"Hover me\"))".to_string());
            l
        }
        "label" => {
            let mut l = vec![format!("Label::new({:?})", ctext(c, 0, "Email address"))];
            if cb(c, 0) {
                l.push("    .required(true)".to_string());
            }
            l
        }
        "kbd" => vec![format!("Kbd::new({:?})", ctext(c, 0, "Ctrl"))],
        "stat" => vec![
            format!("Stat::new({:?})", ctext(c, 0, "$12,345")),
            format!("    .label({:?})", ctext(c, 1, "Revenue")),
            format!(
                "    .description({:?})",
                ctext(c, 2, "+12.5% from last month")
            ),
            format!("    .trend({}, {:?})", vexpr_always(entry, c, 0), "+12.5%"),
        ],
        "copy-button" => vec![format!(
            "CopyButton::new({:?})",
            ctext(c, 0, "cargo add rwire")
        )],
        "blockquote" => vec![
            format!(
                "Blockquote::new({:?})",
                ctext(c, 0, "The best way to predict the future is to invent it.")
            ),
            format!("    .cite({:?})", ctext(c, 1, "Alan Kay")),
        ],
        "image" => {
            let mut l = vec![
                format!(
                    "Image::new({:?})",
                    ctext(c, 0, "https://picsum.photos/400/300")
                ),
                format!("    .alt({:?})", ctext(c, 1, "Sample image")),
            ];
            if let Some(e) = vexpr(entry, c, 0) {
                l.push(format!("    .fit({e})"));
            }
            match c.variants.get(1).copied().unwrap_or(0) {
                1 => l.push("    .aspect_square()".to_string()),
                2 => l.push("    .aspect_video()".to_string()),
                _ => {}
            }
            if cb(c, 0) {
                l.push("    .rounded()".to_string());
            }
            if cb(c, 1) {
                l.push("    .full_width()".to_string());
            }
            l
        }
        "spacer" => {
            let mut l = vec![format!("Spacer::new({})", vexpr_always(entry, c, 0))];
            if cb(c, 0) {
                l.push("    .horizontal()".to_string());
            }
            l
        }
        "skeleton" => match c.variants.first().copied().unwrap_or(0) {
            1 => vec!["Skeleton::circle()".to_string()],
            2 => vec!["Skeleton::rect()".to_string()],
            _ => vec![
                "Skeleton::text()".to_string(),
                format!("    .lines({})", c.nums.first().copied().unwrap_or(3)),
            ],
        },
        "code" => match c.variants.first().copied().unwrap_or(0) {
            1 => vec![
                format!(
                    "Code::block({:?})",
                    "fn main() {\n    println!(\"Hello, world!\");\n}"
                ),
                format!("    .language({:?})", "rust"),
            ],
            _ => vec![format!("Code::inline({:?})", "cargo run")],
        },
        "divider" => {
            let ctor = if cb(c, 0) {
                "Divider::vertical()"
            } else {
                "Divider::horizontal()"
            };
            vec![
                ctor.to_string(),
                format!("    .margin({})", vexpr_always(entry, c, 0)),
            ]
        }
        "form-field" => {
            let mut l = vec![
                "FormField::new()".to_string(),
                format!("    .label({:?})", "Email"),
                "    .input(Input::email().placeholder(\"user@example.com\").build())".to_string(),
            ];
            if cb(c, 0) {
                l.push("    .required(true)".to_string());
            }
            l.push(format!("    .help({:?})", "We'll never share your email"));
            if cb(c, 1) {
                l.push(format!("    .error({:?})", "Invalid email address"));
            }
            l
        }
        "link" => {
            let href = ctext(c, 0, "/docs");
            let text = ctext(c, 1, "Internal Link");
            if cb(c, 0) {
                vec![
                    format!("Link::external({:?})", href),
                    format!("    .text({:?})", text),
                ]
            } else {
                vec![
                    format!("Link::new({:?})", href),
                    format!("    .text({:?})", text),
                ]
            }
        }
        // Custom arm: the `icon` bool maps to an `Icon` value, not `.icon(true)`,
        // so the generic chain can't express it.
        "button" => {
            let mut l = vec!["Button::new()".to_string()];
            if let Some(e) = vexpr(entry, c, 0) {
                l.push(format!("    .intent({e})"));
            }
            if let Some(e) = vexpr(entry, c, 1) {
                l.push(format!("    .size({e})"));
            }
            l.push(format!("    .text({:?})", ctext(c, 0, "Button")));
            if cb(c, 0) {
                l.push("    .disabled(true)".to_string());
            }
            if cb(c, 1) {
                l.push("    .loading(true)".to_string());
            }
            if cb(c, 2) {
                l.push("    .full_width(true)".to_string());
            }
            if cb(c, 3) {
                l.push("    .icon(Icon::Check)".to_string());
            }
            l
        }
        _ => {
            let mut l = vec![format!("{}::new()", entry.name)];
            l.extend(composite_children(entry.slug).iter().map(|s| s.to_string()));
            l.extend(generic_chain(entry, c));
            l
        }
    };
    format!("{}\n    .build()", lines.join("\n"))
}

// ============================================================================
// Layout: Stack, Grid, Container, Card, AppShell, Spacer, Divider
// ============================================================================

fn stack_demo(variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{Gap, Stack, StackAlign, StackDirection, StackJustify};
    let dir = match v(variants, 0) {
        1 => StackDirection::Row,
        _ => StackDirection::Column,
    };
    let gap = match v(variants, 1) {
        0 => Gap::None,
        1 => Gap::Xs,
        2 => Gap::Sm,
        4 => Gap::Lg,
        5 => Gap::Xl,
        _ => Gap::Md,
    };
    let align = match v(variants, 2) {
        1 => StackAlign::Start,
        2 => StackAlign::Center,
        3 => StackAlign::End,
        _ => StackAlign::Stretch,
    };
    let justify = match v(variants, 3) {
        1 => StackJustify::Center,
        2 => StackJustify::End,
        3 => StackJustify::Between,
        4 => StackJustify::Around,
        _ => StackJustify::Start,
    };
    let wrap = b(bools, 0);
    let item = || {
        el(El::Div)
            .st([
                rwire::St::BgMuted,
                rwire::St::RoundedMd,
                rwire::St::PMd,
                rwire::St::TextSm,
            ])
            .text("Item")
    };
    Stack::new()
        .direction(dir)
        .gap(gap)
        .align(align)
        .justify(justify)
        .wrap(wrap)
        .children([item(), item(), item()])
        .build()
}

fn grid_demo(variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{Gap, Grid, GridColumns};
    let cols = match v(variants, 0) {
        0 => GridColumns::Auto,
        1 => GridColumns::Fixed1,
        2 => GridColumns::Fixed2,
        4 => GridColumns::Fixed4,
        _ => GridColumns::Fixed3,
    };
    let gap = match v(variants, 1) {
        0 => Gap::None,
        1 => Gap::Xs,
        2 => Gap::Sm,
        4 => Gap::Lg,
        5 => Gap::Xl,
        _ => Gap::Md,
    };
    let item = |n: &str| {
        el(El::Div)
            .st([
                rwire::St::BgMuted,
                rwire::St::RoundedMd,
                rwire::St::PMd,
                rwire::St::TextSm,
            ])
            .text(n)
    };
    Grid::new()
        .columns(cols)
        .gap(gap)
        .children([
            item("1"),
            item("2"),
            item("3"),
            item("4"),
            item("5"),
            item("6"),
        ])
        .build()
}

fn container_demo(variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{Container, ContainerSize};
    let size = match v(variants, 0) {
        0 => ContainerSize::Sm,
        1 => ContainerSize::Md,
        2 => ContainerSize::Lg,
        3 => ContainerSize::Xl,
        4 => ContainerSize::Full,
        _ => ContainerSize::Md,
    };
    Container::new()
        .size(size)
        .centered(b(bools, 0))
        .padding(b(bools, 1))
        .child(
            el(El::Div)
                .st([
                    rwire::St::BgMuted,
                    rwire::St::RoundedMd,
                    rwire::St::PMd,
                    rwire::St::TextSm,
                ])
                .text("Container content"),
        )
        .build()
}

fn card_demo(variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{Card, CardPadding, CardShadow};
    let padding = match v(variants, 0) {
        0 => CardPadding::None,
        1 => CardPadding::Sm,
        3 => CardPadding::Lg,
        _ => CardPadding::Md,
    };
    let shadow = match v(variants, 1) {
        0 => CardShadow::None,
        2 => CardShadow::Md,
        3 => CardShadow::Lg,
        _ => CardShadow::Sm,
    };
    Card::new()
        .padding(padding)
        .shadow(shadow)
        .bordered(b(bools, 0))
        .child(el(El::P).text("Card content goes here"))
        .build()
}

fn app_shell_demo(_variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::AppShell;
    AppShell::new()
        .header(el(El::Div).st([rwire::St::PMd]).text("Header"))
        .sidebar(el(El::Div).st([rwire::St::PMd]).text("Sidebar"))
        .main(el(El::Div).st([rwire::St::PMd]).text("Main content"))
        .build()
}

fn spacer_demo(variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{Spacer, SpacingSize};
    let size = match v(variants, 0) {
        0 => SpacingSize::None,
        1 => SpacingSize::Xs,
        2 => SpacingSize::Sm,
        4 => SpacingSize::Lg,
        5 => SpacingSize::Xl,
        _ => SpacingSize::Md,
    };
    let horizontal = b(bools, 0);
    let mut spacer = Spacer::new(size);
    if horizontal {
        spacer = spacer.horizontal();
    }
    let line = || {
        el(El::Div)
            .st([
                rwire::St::BgMuted,
                rwire::St::RoundedMd,
                rwire::St::PMd,
                rwire::St::TextSm,
            ])
            .text("Content")
    };
    if horizontal {
        el(El::Div)
            .st([
                rwire::St::DisplayFlex,
                rwire::St::FlexRow,
                rwire::St::ItemsCenter,
            ])
            .append([line(), spacer.build(), line()])
    } else {
        el(El::Div).append([line(), spacer.build(), line()])
    }
}

fn divider_demo(variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{Divider, SpacingSize};
    let margin = match v(variants, 0) {
        0 => SpacingSize::None,
        1 => SpacingSize::Xs,
        2 => SpacingSize::Sm,
        4 => SpacingSize::Lg,
        5 => SpacingSize::Xl,
        _ => SpacingSize::Md,
    };
    let vertical = b(bools, 0);
    let divider = if vertical {
        Divider::vertical().margin(margin)
    } else {
        Divider::horizontal().margin(margin)
    };
    let line = |t: &str| el(El::P).st([rwire::St::TextSm]).text(t);
    if vertical {
        el(El::Div)
            .st([
                rwire::St::DisplayFlex,
                rwire::St::FlexRow,
                rwire::St::ItemsCenter,
                rwire::St::HFull,
            ])
            .append([line("Left"), divider.build(), line("Right")])
    } else {
        el(El::Div).append([line("Above"), divider.build(), line("Below")])
    }
}

// ============================================================================
// Forms: Button, Input, Textarea, Select, Checkbox, Radio, Switch, Slider,
//        Label, FormField
// ============================================================================

fn button_demo(variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{Button, ButtonIntent, ButtonSize};
    let intent = match v(variants, 0) {
        1 => ButtonIntent::Secondary,
        2 => ButtonIntent::Ghost,
        3 => ButtonIntent::Destructive,
        _ => ButtonIntent::Primary,
    };
    let size = match v(variants, 1) {
        0 => ButtonSize::Sm,
        2 => ButtonSize::Lg,
        _ => ButtonSize::Md,
    };
    let mut btn = Button::new()
        .intent(intent)
        .size(size)
        .text("Button")
        .disabled(b(bools, 0))
        .loading(b(bools, 1))
        .full_width(b(bools, 2));
    if b(bools, 3) {
        btn = btn.icon(Icon::Check);
    }
    btn.build()
}

fn input_demo(variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{Input, InputSize, InputType};
    let input_type = match v(variants, 0) {
        1 => InputType::Password,
        2 => InputType::Email,
        3 => InputType::Number,
        4 => InputType::Search,
        5 => InputType::Tel,
        6 => InputType::Url,
        _ => InputType::Text,
    };
    let size = match v(variants, 1) {
        0 => InputSize::Sm,
        2 => InputSize::Lg,
        _ => InputSize::Md,
    };
    Input::new()
        .input_type(input_type)
        .size(size)
        .placeholder("Enter text...")
        .disabled(b(bools, 0))
        .readonly(b(bools, 1))
        .required(b(bools, 2))
        .invalid(b(bools, 3))
        .build()
}

fn textarea_demo(variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::textarea::TextareaSize;
    use crate::Textarea;
    let size = match v(variants, 0) {
        0 => TextareaSize::Sm,
        2 => TextareaSize::Lg,
        _ => TextareaSize::Md,
    };
    Textarea::new()
        .size(size)
        .placeholder("Enter a description...")
        .rows(4)
        .disabled(b(bools, 0))
        .readonly(b(bools, 1))
        .required(b(bools, 2))
        .invalid(b(bools, 3))
        .build()
}

fn select_demo(_variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::Select;
    Select::new()
        .aria_label("Country")
        .option("us", "United States")
        .option("uk", "United Kingdom")
        .option("ca", "Canada")
        .option("au", "Australia")
        .disabled(b(bools, 0))
        .required(b(bools, 1))
        .invalid(b(bools, 2))
        .build()
}

fn checkbox_demo(_variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::Checkbox;
    Checkbox::new()
        .label("Accept terms and conditions")
        .checked(b(bools, 0))
        .disabled(b(bools, 1))
        .required(b(bools, 2))
        .invalid(b(bools, 3))
        .build()
}

fn radio_demo(_variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{Gap, Radio, Stack};
    Stack::column()
        .gap(Gap::Sm)
        .children([
            Radio::new()
                .name("plan")
                .value("free")
                .label("Free Plan")
                .checked(!b(bools, 0))
                .disabled(b(bools, 1))
                .required(b(bools, 2))
                .invalid(b(bools, 3))
                .build(),
            Radio::new()
                .name("plan")
                .value("pro")
                .label("Pro Plan")
                .checked(b(bools, 0))
                .disabled(b(bools, 1))
                .required(b(bools, 2))
                .invalid(b(bools, 3))
                .build(),
        ])
        .build()
}

fn switch_demo(_variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::Switch;
    Switch::new()
        .label("Enable notifications")
        .checked(b(bools, 0))
        .disabled(b(bools, 1))
        .build()
}

fn slider_demo(_variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::Slider;
    Slider::new()
        .min(0)
        .max(100)
        .value(50)
        .step(1)
        .label("Volume")
        .disabled(b(bools, 0))
        .build()
}

fn label_demo(_variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::Label;
    Label::new("Email address").required(b(bools, 0)).build()
}

fn form_field_demo(_variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{FormField, Input};
    let mut ff = FormField::new()
        .label("Email")
        .input(Input::email().placeholder("user@example.com").build())
        .required(b(bools, 0))
        .help("We'll never share your email");
    if b(bools, 1) {
        ff = ff.error("Invalid email address");
    }
    ff.build()
}

// ============================================================================
// Data Display: Text, Badge, Tag, Code, Kbd, Table, Stat, Avatar,
//               AvatarGroup, Image, List, Blockquote, Timeline
// ============================================================================

fn text_demo(variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{Text, TextColor, TextVariant};
    let variant = match v(variants, 0) {
        1 => TextVariant::BodySmall,
        2 => TextVariant::Heading1,
        3 => TextVariant::Heading2,
        4 => TextVariant::Heading3,
        5 => TextVariant::Label,
        6 => TextVariant::Caption,
        _ => TextVariant::Body,
    };
    let color = match v(variants, 1) {
        1 => TextColor::High,
        2 => TextColor::Muted,
        3 => TextColor::Accent,
        4 => TextColor::Success,
        5 => TextColor::Warning,
        6 => TextColor::Error,
        _ => TextColor::Default,
    };
    Text::new()
        .variant(variant)
        .color(color)
        .content("The quick brown fox jumps over the lazy dog.")
        .build()
}

fn badge_demo(variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{Badge, BadgeFill, BadgeIntent, BadgeShape};
    let intent = match v(variants, 0) {
        1 => BadgeIntent::Primary,
        2 => BadgeIntent::Success,
        3 => BadgeIntent::Warning,
        4 => BadgeIntent::Error,
        _ => BadgeIntent::Default,
    };
    let shape = match v(variants, 1) {
        1 => BadgeShape::Square,
        _ => BadgeShape::Pill,
    };
    let fill = match v(variants, 2) {
        1 => BadgeFill::Outline,
        _ => BadgeFill::Solid,
    };
    Badge::new()
        .intent(intent)
        .shape(shape)
        .fill(fill)
        .text("Badge")
        .build()
}

const STATUS_DOT_VARIANTS: &[VariantAxis] = &[VariantAxis {
    name: "intent",
    display_name: "Intent",
    rust_type: "StatusDotIntent",
    default_index: 1,
    values: &[
        VariantValue {
            label: "Muted",
            rust_expr: "StatusDotIntent::Muted",
        },
        VariantValue {
            label: "Primary",
            rust_expr: "StatusDotIntent::Primary",
        },
        VariantValue {
            label: "Success",
            rust_expr: "StatusDotIntent::Success",
        },
        VariantValue {
            label: "Warning",
            rust_expr: "StatusDotIntent::Warning",
        },
        VariantValue {
            label: "Error",
            rust_expr: "StatusDotIntent::Error",
        },
    ],
}];

fn status_dot_demo(variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{StatusDot, StatusDotIntent};
    let intent = match v(variants, 0) {
        1 => StatusDotIntent::Primary,
        2 => StatusDotIntent::Success,
        3 => StatusDotIntent::Warning,
        4 => StatusDotIntent::Error,
        _ => StatusDotIntent::Muted,
    };
    let mut dot = StatusDot::new().intent(intent).pulse(b(bools, 0));
    if b(bools, 1) {
        dot = dot.label("running");
    }
    dot.build()
}

fn composer_demo(_variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::Composer;
    let mut composer = Composer::new()
        .placeholder("Message the team…")
        .compact(b(bools, 0));
    if !b(bools, 0) {
        composer = composer.hint("⏎ send · ⇧⏎ newline");
    }
    composer.build()
}

fn streamed_content_demo(_variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{Spinner, SpinnerSize};
    // Static rendition of the streamed region: delivered chunks, plus the
    // sentinel spinner row while more content remains. The live component arms
    // a one-shot visibility sentinel (BIND_SENTINEL) on that row instead.
    use rwire::St;
    let loading = b(bools, 0);
    let mut root = el(El::Div).st([St::DisplayFlex, St::FlexCol, St::GapMd, St::MinW0]);
    for i in 0..3 {
        root = root.append([el(El::Div)
            .st([St::BgSurface, St::PMd, St::RoundedMd])
            .append([el(El::P).st([St::TextSm, St::TextMuted]).text(
                format!(
                    "Chunk {} — delivered when the sentinel neared the viewport.",
                    i + 1
                )
                .as_str(),
            )])]);
    }
    if loading {
        root = root.append([el(El::Div)
            .st([St::DisplayFlex, St::JustifyCenter, St::PMd])
            .append([Spinner::new().size(SpinnerSize::Sm).build()])]);
    }
    root
}

fn chip_demo(_variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::Chip;
    el(El::Div)
        .st([
            rwire::St::DisplayFlex,
            rwire::St::GapSm,
            rwire::St::ItemsCenter,
        ])
        .append([
            Chip::new("All").active(b(bools, 0)).build(),
            Chip::new("Running").build(),
            Chip::new("Failed").build(),
        ])
}

fn chat_scroll_demo(_variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::ChatScroll;
    let entries = (1..=12).map(|n| {
        el(El::Div)
            .st([rwire::St::TextSm, rwire::St::PySm])
            .text(&format!(
                "message {n} — the newest stays pinned at the bottom"
            ))
    });
    el(El::Div)
        .style(rwire::style::Style::new().set("height", "10rem"))
        .st([rwire::St::DisplayFlex, rwire::St::FlexCol])
        .append([ChatScroll::new(
            el(El::Div)
                .st([rwire::St::DisplayFlex, rwire::St::FlexCol])
                .append(entries),
        )
        .build()])
}

fn tag_demo(variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{Tag, TagIntent};
    let intent = match v(variants, 0) {
        1 => TagIntent::Primary,
        2 => TagIntent::Success,
        3 => TagIntent::Warning,
        4 => TagIntent::Error,
        _ => TagIntent::Default,
    };
    Tag::new("Tag")
        .intent(intent)
        .removable(b(bools, 0))
        .build()
}

fn code_demo(variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::Code;
    match v(variants, 0) {
        1 => Code::block("fn main() {\n    println!(\"Hello, world!\");\n}")
            .language("rust")
            .build(),
        _ => Code::inline("cargo run").build(),
    }
}

fn kbd_demo(_variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{Gap, Kbd, Stack};
    Stack::row()
        .gap(Gap::Sm)
        .children([
            Kbd::new("Ctrl").build(),
            Kbd::new("Shift").build(),
            Kbd::new("P").build(),
        ])
        .build()
}

fn table_demo(_variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{Table, TableRow};
    Table::new()
        .headers(["Name", "Role", "Status"])
        .row(
            TableRow::new()
                .cell("Alice")
                .cell("Engineer")
                .cell("Active"),
        )
        .row(TableRow::new().cell("Bob").cell("Designer").cell("Active"))
        .row(TableRow::new().cell("Carol").cell("Manager").cell("Away"))
        .striped(b(bools, 0))
        .build()
}

fn stat_demo(variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{Stat, StatTrend};
    let trend = match v(variants, 0) {
        1 => StatTrend::Down,
        2 => StatTrend::Neutral,
        _ => StatTrend::Up,
    };
    Stat::new("$12,345")
        .label("Revenue")
        .description("+12.5% from last month")
        .trend(trend, "+12.5%")
        .build()
}

fn avatar_demo(variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{Avatar, AvatarSize};
    let size = match v(variants, 0) {
        0 => AvatarSize::Sm,
        2 => AvatarSize::Lg,
        _ => AvatarSize::Md,
    };
    Avatar::new().fallback("JD").size(size).build()
}

fn avatar_group_demo(_variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{Avatar, AvatarGroup, AvatarSize};
    AvatarGroup::new()
        .avatar(Avatar::new().fallback("AB").size(AvatarSize::Md))
        .avatar(Avatar::new().fallback("CD").size(AvatarSize::Md))
        .avatar(Avatar::new().fallback("EF").size(AvatarSize::Md))
        .avatar(Avatar::new().fallback("GH").size(AvatarSize::Md))
        .max_visible(3)
        .build()
}

fn image_demo(variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{Image, ImageFit};
    let fit = match v(variants, 0) {
        1 => ImageFit::Contain,
        2 => ImageFit::Fill,
        3 => ImageFit::None,
        _ => ImageFit::Cover,
    };
    let mut img = Image::new("https://picsum.photos/400/300")
        .alt("Sample image")
        .fit(fit);
    match v(variants, 1) {
        1 => img = img.aspect_square(),
        2 => img = img.aspect_video(),
        _ => {}
    }
    if b(bools, 0) {
        img = img.rounded();
    }
    if b(bools, 1) {
        img = img.full_width();
    }
    img.build()
}

fn list_demo(_variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{List, ListItem};
    let mut list = if b(bools, 0) {
        List::ordered()
    } else {
        List::unordered()
    };
    list = list.children([
        ListItem::new("First item").build(),
        ListItem::new("Second item").build(),
        ListItem::new("Third item").build(),
    ]);
    list.build()
}

fn blockquote_demo(_variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::Blockquote;
    Blockquote::new("The best way to predict the future is to invent it.")
        .cite("Alan Kay")
        .build()
}

fn timeline_demo(_variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{Timeline, TimelineItem};
    Timeline::new()
        .item(
            TimelineItem::new("Order placed")
                .description("Jan 1, 2026")
                .active(true),
        )
        .item(
            TimelineItem::new("Shipped")
                .description("Jan 3, 2026")
                .active(true),
        )
        .item(TimelineItem::new("Delivered").description("Pending"))
        .build()
}

// ============================================================================
// Feedback: Alert, Toast, Spinner, Progress, Skeleton, EmptyState, Tooltip
// ============================================================================

fn alert_demo(variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{Alert, AlertIntent};
    let intent = match v(variants, 0) {
        1 => AlertIntent::Success,
        2 => AlertIntent::Warning,
        3 => AlertIntent::Error,
        _ => AlertIntent::Info,
    };
    Alert::new()
        .intent(intent)
        .title("Alert Title")
        .message("This is an alert message with important information.")
        .build()
}

fn toast_demo(variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{Toast, ToastIntent};
    let intent = match v(variants, 0) {
        1 => ToastIntent::Success,
        2 => ToastIntent::Warning,
        3 => ToastIntent::Error,
        _ => ToastIntent::Info,
    };
    Toast::new("Toast notification message")
        .intent(intent)
        .build()
}

fn spinner_demo(variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{Spinner, SpinnerSize};
    let size = match v(variants, 0) {
        0 => SpinnerSize::Sm,
        2 => SpinnerSize::Lg,
        _ => SpinnerSize::Md,
    };
    Spinner::new().size(size).label("Loading...").build()
}

fn progress_demo(variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{Progress, ProgressSize};
    match v(variants, 0) {
        1 => Progress::new()
            .value(3)
            .max(5)
            .size(ProgressSize::Sm)
            .label("3 of 5 tasks done")
            .build(),
        _ => Progress::new()
            .value(65)
            .max(100)
            .label("Upload progress")
            .build(),
    }
}

fn skeleton_demo(variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::Skeleton;
    match v(variants, 0) {
        1 => Skeleton::circle().build(),
        2 => Skeleton::rect().build(),
        _ => Skeleton::text().lines(3).build(),
    }
}

fn empty_state_demo(_variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::EmptyState;
    EmptyState::new()
        .title("No results found")
        .description("Try adjusting your search or filter criteria.")
        .build()
}

fn tooltip_demo(variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{Tooltip, TooltipPosition};
    let position = match v(variants, 0) {
        1 => TooltipPosition::Bottom,
        2 => TooltipPosition::Left,
        3 => TooltipPosition::Right,
        _ => TooltipPosition::Top,
    };
    Tooltip::new("Tooltip text")
        .position(position)
        .child(
            el(El::Button)
                .st([
                    rwire::St::BgPrimary,
                    rwire::St::TextOnPrimary,
                    rwire::St::PxMd,
                    rwire::St::PySm,
                    rwire::St::RoundedMd,
                    rwire::St::BorderNone,
                    rwire::St::CursorPointer,
                ])
                .text("Hover me"),
        )
        .build()
}

// ============================================================================
// Navigation: Link, NavMenu, Breadcrumb, Tabs, Pagination, Footer
// ============================================================================

fn link_demo(_variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::Link;
    if b(bools, 0) {
        Link::external("https://example.com")
            .text("External Link")
            .build()
    } else {
        Link::new("/docs").text("Internal Link").build()
    }
}

fn nav_menu_demo(_variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{NavItem, NavMenu};
    NavMenu::new()
        .item(NavItem::new("Home", "/"))
        .item(NavItem::new("About", "/about"))
        .item(NavItem::new("Contact", "/contact"))
        .active_path("/")
        .build()
}

fn breadcrumb_demo(_variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::Breadcrumb;
    Breadcrumb::new()
        .item("Home", Some("/"))
        .item("Components", Some("/components"))
        .item("Button", None::<&str>)
        .build()
}

fn tabs_demo(_variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{Tab, Tabs};
    Tabs::new()
        .tab(Tab::new("Overview", el(El::P).text("Overview content")))
        .tab(Tab::new("Settings", el(El::P).text("Settings content")))
        .tab(Tab::new("Activity", el(El::P).text("Activity content")))
        .active(0)
        .build()
}

fn pagination_demo(_variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::Pagination;
    Pagination::new().current_page(3).total_pages(10).build()
}

fn footer_demo(_variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{Footer, FooterColumn};
    Footer::new()
        .logo(
            el(El::Span)
                .st([rwire::St::TextLg, rwire::St::FontBold])
                .text("Brand"),
        )
        .tagline("Building the future, one component at a time.")
        .column(
            FooterColumn::new("Product")
                .link("Features", "/features")
                .link("Pricing", "/pricing"),
        )
        .column(
            FooterColumn::new("Company")
                .link("About", "/about")
                .link("Blog", "/blog"),
        )
        .copyright("2026 Brand Inc.")
        .build()
}

// ============================================================================
// Overlay: Modal, Drawer, Dropdown, Accordion, CopyButton, ThemeToggle,
//          Stepper
// ============================================================================

fn modal_demo(variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{Modal, ModalSize};
    let size = match v(variants, 0) {
        0 => ModalSize::Sm,
        2 => ModalSize::Lg,
        3 => ModalSize::Xl,
        4 => ModalSize::Full,
        _ => ModalSize::Md,
    };
    Modal::new()
        .title("Confirm Action")
        .size(size)
        .open(b(bools, 0))
        .content(el(El::P).text("Are you sure you want to proceed?"))
        .build()
}

fn drawer_demo(variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{Drawer, DrawerPosition};
    let position = match v(variants, 0) {
        1 => DrawerPosition::Right,
        _ => DrawerPosition::Left,
    };
    Drawer::new()
        .title("Navigation")
        .position(position)
        .open(b(bools, 0))
        .content(el(El::P).text("Drawer content goes here."))
        .build()
}

fn dropdown_demo(_variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{Button, DropdownItem, DropdownMenu};
    DropdownMenu::new()
        .open(b(bools, 0))
        .trigger(Button::secondary("Options").build())
        .item(DropdownItem::new("Edit"))
        .item(DropdownItem::new("Duplicate"))
        .item(DropdownItem::new("Delete").destructive())
        .build()
}

fn accordion_demo(_variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{Accordion, AccordionItem};
    Accordion::new()
        .item(
            AccordionItem::new("What is rwire?")
                .content(el(El::P).text("A WebSocket-based UI framework."))
                .open(true),
        )
        .item(
            AccordionItem::new("How does it work?")
                .content(el(El::P).text("Server renders DOM via binary protocol.")),
        )
        .item(
            AccordionItem::new("Is it production-ready?")
                .content(el(El::P).text("It is currently in experimental phase.")),
        )
        .build()
}

fn copy_button_demo(_variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::CopyButton;
    CopyButton::new("cargo add rwire").build()
}

fn theme_toggle_demo(variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{ThemeToggle, ThemeToggleMode, ToggleSize};
    let size = match v(variants, 0) {
        0 => ToggleSize::Sm,
        2 => ToggleSize::Lg,
        _ => ToggleSize::Md,
    };
    let mode = match v(variants, 1) {
        1 => ThemeToggleMode::Dark,
        _ => ThemeToggleMode::Light,
    };
    ThemeToggle::new()
        .size(size)
        .mode(mode)
        .show_label(b(bools, 0))
        .build()
}

fn stepper_demo(_variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::Stepper;
    Stepper::new()
        .step("Account")
        .step("Profile")
        .step("Review")
        .current(1)
        .build()
}

// ============================================================================
// Variant/Bool Prop Const Data
// ============================================================================

// --- Layout ---

const STACK_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "direction",
        display_name: "Direction",
        rust_type: "StackDirection",
        values: &[
            VariantValue {
                label: "Column",
                rust_expr: "StackDirection::Column",
            },
            VariantValue {
                label: "Row",
                rust_expr: "StackDirection::Row",
            },
        ],
        default_index: 0,
    },
    VariantAxis {
        name: "gap",
        display_name: "Gap",
        rust_type: "Gap",
        values: &[
            VariantValue {
                label: "None",
                rust_expr: "Gap::None",
            },
            VariantValue {
                label: "Xs",
                rust_expr: "Gap::Xs",
            },
            VariantValue {
                label: "Sm",
                rust_expr: "Gap::Sm",
            },
            VariantValue {
                label: "Md",
                rust_expr: "Gap::Md",
            },
            VariantValue {
                label: "Lg",
                rust_expr: "Gap::Lg",
            },
            VariantValue {
                label: "Xl",
                rust_expr: "Gap::Xl",
            },
        ],
        default_index: 3,
    },
    VariantAxis {
        name: "align",
        display_name: "Align",
        rust_type: "StackAlign",
        values: &[
            VariantValue {
                label: "Stretch",
                rust_expr: "StackAlign::Stretch",
            },
            VariantValue {
                label: "Start",
                rust_expr: "StackAlign::Start",
            },
            VariantValue {
                label: "Center",
                rust_expr: "StackAlign::Center",
            },
            VariantValue {
                label: "End",
                rust_expr: "StackAlign::End",
            },
        ],
        default_index: 0,
    },
    VariantAxis {
        name: "justify",
        display_name: "Justify",
        rust_type: "StackJustify",
        values: &[
            VariantValue {
                label: "Start",
                rust_expr: "StackJustify::Start",
            },
            VariantValue {
                label: "Center",
                rust_expr: "StackJustify::Center",
            },
            VariantValue {
                label: "End",
                rust_expr: "StackJustify::End",
            },
            VariantValue {
                label: "Between",
                rust_expr: "StackJustify::Between",
            },
            VariantValue {
                label: "Around",
                rust_expr: "StackJustify::Around",
            },
        ],
        default_index: 0,
    },
];

const GRID_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "columns",
        display_name: "Columns",
        rust_type: "GridColumns",
        values: &[
            VariantValue {
                label: "Auto",
                rust_expr: "GridColumns::Auto",
            },
            VariantValue {
                label: "1",
                rust_expr: "GridColumns::Fixed1",
            },
            VariantValue {
                label: "2",
                rust_expr: "GridColumns::Fixed2",
            },
            VariantValue {
                label: "3",
                rust_expr: "GridColumns::Fixed3",
            },
            VariantValue {
                label: "4",
                rust_expr: "GridColumns::Fixed4",
            },
        ],
        default_index: 3,
    },
    VariantAxis {
        name: "gap",
        display_name: "Gap",
        rust_type: "Gap",
        values: &[
            VariantValue {
                label: "None",
                rust_expr: "Gap::None",
            },
            VariantValue {
                label: "Xs",
                rust_expr: "Gap::Xs",
            },
            VariantValue {
                label: "Sm",
                rust_expr: "Gap::Sm",
            },
            VariantValue {
                label: "Md",
                rust_expr: "Gap::Md",
            },
            VariantValue {
                label: "Lg",
                rust_expr: "Gap::Lg",
            },
            VariantValue {
                label: "Xl",
                rust_expr: "Gap::Xl",
            },
        ],
        default_index: 3,
    },
];

const CONTAINER_VARIANTS: &[VariantAxis] = &[VariantAxis {
    name: "size",
    display_name: "Size",
    rust_type: "ContainerSize",
    values: &[
        VariantValue {
            label: "Sm",
            rust_expr: "ContainerSize::Sm",
        },
        VariantValue {
            label: "Md",
            rust_expr: "ContainerSize::Md",
        },
        VariantValue {
            label: "Lg",
            rust_expr: "ContainerSize::Lg",
        },
        VariantValue {
            label: "Xl",
            rust_expr: "ContainerSize::Xl",
        },
        VariantValue {
            label: "Full",
            rust_expr: "ContainerSize::Full",
        },
    ],
    default_index: 1,
}];

const CARD_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "padding",
        display_name: "Padding",
        rust_type: "CardPadding",
        values: &[
            VariantValue {
                label: "None",
                rust_expr: "CardPadding::None",
            },
            VariantValue {
                label: "Sm",
                rust_expr: "CardPadding::Sm",
            },
            VariantValue {
                label: "Md",
                rust_expr: "CardPadding::Md",
            },
            VariantValue {
                label: "Lg",
                rust_expr: "CardPadding::Lg",
            },
        ],
        default_index: 2,
    },
    VariantAxis {
        name: "shadow",
        display_name: "Shadow",
        rust_type: "CardShadow",
        values: &[
            VariantValue {
                label: "None",
                rust_expr: "CardShadow::None",
            },
            VariantValue {
                label: "Sm",
                rust_expr: "CardShadow::Sm",
            },
            VariantValue {
                label: "Md",
                rust_expr: "CardShadow::Md",
            },
            VariantValue {
                label: "Lg",
                rust_expr: "CardShadow::Lg",
            },
        ],
        default_index: 1,
    },
];

const SPACING_SIZE_VARIANTS: &[VariantAxis] = &[VariantAxis {
    name: "size",
    display_name: "Size",
    rust_type: "SpacingSize",
    values: &[
        VariantValue {
            label: "None",
            rust_expr: "SpacingSize::None",
        },
        VariantValue {
            label: "Xs",
            rust_expr: "SpacingSize::Xs",
        },
        VariantValue {
            label: "Sm",
            rust_expr: "SpacingSize::Sm",
        },
        VariantValue {
            label: "Md",
            rust_expr: "SpacingSize::Md",
        },
        VariantValue {
            label: "Lg",
            rust_expr: "SpacingSize::Lg",
        },
        VariantValue {
            label: "Xl",
            rust_expr: "SpacingSize::Xl",
        },
    ],
    default_index: 3,
}];

// --- Forms ---

const BUTTON_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "intent",
        display_name: "Intent",
        rust_type: "ButtonIntent",
        values: &[
            VariantValue {
                label: "Primary",
                rust_expr: "ButtonIntent::Primary",
            },
            VariantValue {
                label: "Secondary",
                rust_expr: "ButtonIntent::Secondary",
            },
            VariantValue {
                label: "Ghost",
                rust_expr: "ButtonIntent::Ghost",
            },
            VariantValue {
                label: "Destructive",
                rust_expr: "ButtonIntent::Destructive",
            },
        ],
        default_index: 0,
    },
    VariantAxis {
        name: "size",
        display_name: "Size",
        rust_type: "ButtonSize",
        values: &[
            VariantValue {
                label: "Sm",
                rust_expr: "ButtonSize::Sm",
            },
            VariantValue {
                label: "Md",
                rust_expr: "ButtonSize::Md",
            },
            VariantValue {
                label: "Lg",
                rust_expr: "ButtonSize::Lg",
            },
        ],
        default_index: 1,
    },
];

const INPUT_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "type",
        display_name: "Type",
        rust_type: "InputType",
        values: &[
            VariantValue {
                label: "Text",
                rust_expr: "InputType::Text",
            },
            VariantValue {
                label: "Password",
                rust_expr: "InputType::Password",
            },
            VariantValue {
                label: "Email",
                rust_expr: "InputType::Email",
            },
            VariantValue {
                label: "Number",
                rust_expr: "InputType::Number",
            },
            VariantValue {
                label: "Search",
                rust_expr: "InputType::Search",
            },
            VariantValue {
                label: "Tel",
                rust_expr: "InputType::Tel",
            },
            VariantValue {
                label: "Url",
                rust_expr: "InputType::Url",
            },
        ],
        default_index: 0,
    },
    VariantAxis {
        name: "size",
        display_name: "Size",
        rust_type: "InputSize",
        values: &[
            VariantValue {
                label: "Sm",
                rust_expr: "InputSize::Sm",
            },
            VariantValue {
                label: "Md",
                rust_expr: "InputSize::Md",
            },
            VariantValue {
                label: "Lg",
                rust_expr: "InputSize::Lg",
            },
        ],
        default_index: 1,
    },
];

const TEXTAREA_VARIANTS: &[VariantAxis] = &[VariantAxis {
    name: "size",
    display_name: "Size",
    rust_type: "TextareaSize",
    values: &[
        VariantValue {
            label: "Sm",
            rust_expr: "TextareaSize::Sm",
        },
        VariantValue {
            label: "Md",
            rust_expr: "TextareaSize::Md",
        },
        VariantValue {
            label: "Lg",
            rust_expr: "TextareaSize::Lg",
        },
    ],
    default_index: 1,
}];

// --- Data Display ---

const TEXT_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "variant",
        display_name: "Variant",
        rust_type: "TextVariant",
        values: &[
            VariantValue {
                label: "Body",
                rust_expr: "TextVariant::Body",
            },
            VariantValue {
                label: "BodySmall",
                rust_expr: "TextVariant::BodySmall",
            },
            VariantValue {
                label: "Heading1",
                rust_expr: "TextVariant::Heading1",
            },
            VariantValue {
                label: "Heading2",
                rust_expr: "TextVariant::Heading2",
            },
            VariantValue {
                label: "Heading3",
                rust_expr: "TextVariant::Heading3",
            },
            VariantValue {
                label: "Label",
                rust_expr: "TextVariant::Label",
            },
            VariantValue {
                label: "Caption",
                rust_expr: "TextVariant::Caption",
            },
        ],
        default_index: 0,
    },
    VariantAxis {
        name: "color",
        display_name: "Color",
        rust_type: "TextColor",
        values: &[
            VariantValue {
                label: "Default",
                rust_expr: "TextColor::Default",
            },
            VariantValue {
                label: "High",
                rust_expr: "TextColor::High",
            },
            VariantValue {
                label: "Muted",
                rust_expr: "TextColor::Muted",
            },
            VariantValue {
                label: "Accent",
                rust_expr: "TextColor::Accent",
            },
            VariantValue {
                label: "Success",
                rust_expr: "TextColor::Success",
            },
            VariantValue {
                label: "Warning",
                rust_expr: "TextColor::Warning",
            },
            VariantValue {
                label: "Error",
                rust_expr: "TextColor::Error",
            },
        ],
        default_index: 0,
    },
];

const BADGE_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "intent",
        display_name: "Intent",
        rust_type: "BadgeIntent",
        values: &[
            VariantValue {
                label: "Default",
                rust_expr: "BadgeIntent::Default",
            },
            VariantValue {
                label: "Primary",
                rust_expr: "BadgeIntent::Primary",
            },
            VariantValue {
                label: "Success",
                rust_expr: "BadgeIntent::Success",
            },
            VariantValue {
                label: "Warning",
                rust_expr: "BadgeIntent::Warning",
            },
            VariantValue {
                label: "Error",
                rust_expr: "BadgeIntent::Error",
            },
        ],
        default_index: 0,
    },
    VariantAxis {
        name: "shape",
        display_name: "Shape",
        rust_type: "BadgeShape",
        default_index: 0,
        values: &[
            VariantValue {
                label: "Pill",
                rust_expr: "BadgeShape::Pill",
            },
            VariantValue {
                label: "Square",
                rust_expr: "BadgeShape::Square",
            },
        ],
    },
    VariantAxis {
        name: "fill",
        display_name: "Fill",
        rust_type: "BadgeFill",
        default_index: 0,
        values: &[
            VariantValue {
                label: "Solid",
                rust_expr: "BadgeFill::Solid",
            },
            VariantValue {
                label: "Outline",
                rust_expr: "BadgeFill::Outline",
            },
        ],
    },
];

const TAG_VARIANTS: &[VariantAxis] = &[VariantAxis {
    name: "intent",
    display_name: "Intent",
    rust_type: "TagIntent",
    values: &[
        VariantValue {
            label: "Default",
            rust_expr: "TagIntent::Default",
        },
        VariantValue {
            label: "Primary",
            rust_expr: "TagIntent::Primary",
        },
        VariantValue {
            label: "Success",
            rust_expr: "TagIntent::Success",
        },
        VariantValue {
            label: "Warning",
            rust_expr: "TagIntent::Warning",
        },
        VariantValue {
            label: "Error",
            rust_expr: "TagIntent::Error",
        },
    ],
    default_index: 0,
}];

const CODE_VARIANTS: &[VariantAxis] = &[VariantAxis {
    name: "mode",
    display_name: "Mode",
    rust_type: "CodeMode",
    values: &[
        VariantValue {
            label: "Inline",
            rust_expr: "CodeMode::Inline",
        },
        VariantValue {
            label: "Block",
            rust_expr: "CodeMode::Block",
        },
    ],
    default_index: 0,
}];

const STAT_VARIANTS: &[VariantAxis] = &[VariantAxis {
    name: "trend",
    display_name: "Trend",
    rust_type: "StatTrend",
    values: &[
        VariantValue {
            label: "Up",
            rust_expr: "StatTrend::Up",
        },
        VariantValue {
            label: "Down",
            rust_expr: "StatTrend::Down",
        },
        VariantValue {
            label: "Neutral",
            rust_expr: "StatTrend::Neutral",
        },
    ],
    default_index: 0,
}];

const AVATAR_SIZE: &[VariantAxis] = &[VariantAxis {
    name: "size",
    display_name: "Size",
    rust_type: "AvatarSize",
    values: &[
        VariantValue {
            label: "Sm",
            rust_expr: "AvatarSize::Sm",
        },
        VariantValue {
            label: "Md",
            rust_expr: "AvatarSize::Md",
        },
        VariantValue {
            label: "Lg",
            rust_expr: "AvatarSize::Lg",
        },
    ],
    default_index: 1,
}];

const PROGRESS_SIZE: &[VariantAxis] = &[VariantAxis {
    name: "size",
    display_name: "Size",
    rust_type: "ProgressSize",
    values: &[
        VariantValue {
            label: "Md",
            rust_expr: "ProgressSize::Md",
        },
        VariantValue {
            label: "Sm (hairline)",
            rust_expr: "ProgressSize::Sm",
        },
    ],
    default_index: 0,
}];

const SPINNER_SIZE: &[VariantAxis] = &[VariantAxis {
    name: "size",
    display_name: "Size",
    rust_type: "SpinnerSize",
    values: &[
        VariantValue {
            label: "Sm",
            rust_expr: "SpinnerSize::Sm",
        },
        VariantValue {
            label: "Md",
            rust_expr: "SpinnerSize::Md",
        },
        VariantValue {
            label: "Lg",
            rust_expr: "SpinnerSize::Lg",
        },
    ],
    default_index: 1,
}];

const IMAGE_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "fit",
        display_name: "Fit",
        rust_type: "ImageFit",
        values: &[
            VariantValue {
                label: "Cover",
                rust_expr: "ImageFit::Cover",
            },
            VariantValue {
                label: "Contain",
                rust_expr: "ImageFit::Contain",
            },
            VariantValue {
                label: "Fill",
                rust_expr: "ImageFit::Fill",
            },
            VariantValue {
                label: "None",
                rust_expr: "ImageFit::None",
            },
        ],
        default_index: 0,
    },
    VariantAxis {
        name: "aspect",
        display_name: "Aspect",
        rust_type: "ImageAspect",
        values: &[
            VariantValue {
                label: "Auto",
                rust_expr: "ImageAspect::Auto",
            },
            VariantValue {
                label: "Square",
                rust_expr: "ImageAspect::Square",
            },
            VariantValue {
                label: "Video",
                rust_expr: "ImageAspect::Video",
            },
        ],
        default_index: 0,
    },
];

// --- Feedback ---

const ALERT_VARIANTS: &[VariantAxis] = &[VariantAxis {
    name: "intent",
    display_name: "Intent",
    rust_type: "AlertIntent",
    values: &[
        VariantValue {
            label: "Info",
            rust_expr: "AlertIntent::Info",
        },
        VariantValue {
            label: "Success",
            rust_expr: "AlertIntent::Success",
        },
        VariantValue {
            label: "Warning",
            rust_expr: "AlertIntent::Warning",
        },
        VariantValue {
            label: "Error",
            rust_expr: "AlertIntent::Error",
        },
    ],
    default_index: 0,
}];

const TOAST_VARIANTS: &[VariantAxis] = &[VariantAxis {
    name: "intent",
    display_name: "Intent",
    rust_type: "ToastIntent",
    values: &[
        VariantValue {
            label: "Info",
            rust_expr: "ToastIntent::Info",
        },
        VariantValue {
            label: "Success",
            rust_expr: "ToastIntent::Success",
        },
        VariantValue {
            label: "Warning",
            rust_expr: "ToastIntent::Warning",
        },
        VariantValue {
            label: "Error",
            rust_expr: "ToastIntent::Error",
        },
    ],
    default_index: 0,
}];

const SKELETON_VARIANTS: &[VariantAxis] = &[VariantAxis {
    name: "shape",
    display_name: "Shape",
    rust_type: "SkeletonShape",
    values: &[
        VariantValue {
            label: "Text",
            rust_expr: "SkeletonShape::Text",
        },
        VariantValue {
            label: "Circle",
            rust_expr: "SkeletonShape::Circle",
        },
        VariantValue {
            label: "Rect",
            rust_expr: "SkeletonShape::Rect",
        },
    ],
    default_index: 0,
}];

const TOOLTIP_VARIANTS: &[VariantAxis] = &[VariantAxis {
    name: "position",
    display_name: "Position",
    rust_type: "TooltipPosition",
    values: &[
        VariantValue {
            label: "Top",
            rust_expr: "TooltipPosition::Top",
        },
        VariantValue {
            label: "Bottom",
            rust_expr: "TooltipPosition::Bottom",
        },
        VariantValue {
            label: "Left",
            rust_expr: "TooltipPosition::Left",
        },
        VariantValue {
            label: "Right",
            rust_expr: "TooltipPosition::Right",
        },
    ],
    default_index: 0,
}];

// --- Overlay ---

const MODAL_VARIANTS: &[VariantAxis] = &[VariantAxis {
    name: "size",
    display_name: "Size",
    rust_type: "ModalSize",
    values: &[
        VariantValue {
            label: "Sm",
            rust_expr: "ModalSize::Sm",
        },
        VariantValue {
            label: "Md",
            rust_expr: "ModalSize::Md",
        },
        VariantValue {
            label: "Lg",
            rust_expr: "ModalSize::Lg",
        },
        VariantValue {
            label: "Xl",
            rust_expr: "ModalSize::Xl",
        },
        VariantValue {
            label: "Full",
            rust_expr: "ModalSize::Full",
        },
    ],
    default_index: 1,
}];

const DRAWER_VARIANTS: &[VariantAxis] = &[VariantAxis {
    name: "position",
    display_name: "Position",
    rust_type: "DrawerPosition",
    values: &[
        VariantValue {
            label: "Left",
            rust_expr: "DrawerPosition::Left",
        },
        VariantValue {
            label: "Right",
            rust_expr: "DrawerPosition::Right",
        },
    ],
    default_index: 0,
}];

const THEME_TOGGLE_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "size",
        display_name: "Size",
        rust_type: "ToggleSize",
        values: &[
            VariantValue {
                label: "Sm",
                rust_expr: "ToggleSize::Sm",
            },
            VariantValue {
                label: "Md",
                rust_expr: "ToggleSize::Md",
            },
            VariantValue {
                label: "Lg",
                rust_expr: "ToggleSize::Lg",
            },
        ],
        default_index: 1,
    },
    VariantAxis {
        name: "mode",
        display_name: "Mode",
        rust_type: "ThemeToggleMode",
        values: &[
            VariantValue {
                label: "Light",
                rust_expr: "ThemeToggleMode::Light",
            },
            VariantValue {
                label: "Dark",
                rust_expr: "ThemeToggleMode::Dark",
            },
        ],
        default_index: 0,
    },
];

// --- Common Bool Prop Sets ---

const BOOL_DISABLED: &[BoolProp] = &[BoolProp {
    name: "disabled",
    description: "Prevent interaction",
    default: false,
}];

const BOOL_FORM_FULL: &[BoolProp] = &[
    BoolProp {
        name: "disabled",
        description: "Prevent interaction",
        default: false,
    },
    BoolProp {
        name: "readonly",
        description: "Read-only mode",
        default: false,
    },
    BoolProp {
        name: "required",
        description: "Mark as required",
        default: false,
    },
    BoolProp {
        name: "invalid",
        description: "Show error state",
        default: false,
    },
];

// Input adds the form bools plus its own focus/spellcheck toggles.
const INPUT_BOOL: &[BoolProp] = &[
    BoolProp {
        name: "disabled",
        description: "Prevent interaction",
        default: false,
    },
    BoolProp {
        name: "readonly",
        description: "Read-only mode",
        default: false,
    },
    BoolProp {
        name: "required",
        description: "Mark as required",
        default: false,
    },
    BoolProp {
        name: "invalid",
        description: "Show error state",
        default: false,
    },
    BoolProp {
        name: "autofocus",
        description: "Focus on mount",
        default: false,
    },
    BoolProp {
        name: "spellcheck",
        description: "Enable spellcheck",
        default: false,
    },
];

const BOOL_CHECK_FORM: &[BoolProp] = &[
    BoolProp {
        name: "checked",
        description: "Toggle checked state",
        default: false,
    },
    BoolProp {
        name: "disabled",
        description: "Prevent interaction",
        default: false,
    },
    BoolProp {
        name: "required",
        description: "Mark as required",
        default: false,
    },
    BoolProp {
        name: "invalid",
        description: "Show error state",
        default: false,
    },
];

const MODAL_BOOLS: &[BoolProp] = &[
    BoolProp {
        name: "open",
        description: "Show component",
        default: false,
    },
    BoolProp {
        name: "close_on_backdrop_click",
        description: "Close when the backdrop is clicked",
        default: true,
    },
];

const THEME_TOGGLE_BOOLS: &[BoolProp] = &[BoolProp {
    name: "show_label",
    description: "Show a text label next to the toggle",
    default: false,
}];

const BOOL_OPEN: &[BoolProp] = &[BoolProp {
    name: "open",
    description: "Show component",
    default: false,
}];

// ============================================================================
// Catalog Entries
// ============================================================================

const ENTRIES: &[ComponentEntry] = &[
    // --- Layout (order 1xx) ---
    ComponentEntry {
        name: "ChatScroll",
        slug: "chat-scroll",
        description: "Bottom-pinned autoscroll for chat logs and live feeds — pure CSS, no JS.",
        category: Category::Layout,
        order: 103,
        variants: &[],
        bool_props: &[],
        build_demo: chat_scroll_demo,
    },
    ComponentEntry {
        name: "Stack",
        slug: "stack",
        description: "Flexbox layout with configurable direction and spacing.",
        category: Category::Layout,
        order: 100,
        variants: STACK_VARIANTS,
        bool_props: &[BoolProp {
            name: "wrap",
            description: "Enable flex wrap",
            default: false,
        }],
        build_demo: stack_demo,
    },
    ComponentEntry {
        name: "Grid",
        slug: "grid",
        description: "CSS Grid layout with configurable columns.",
        category: Category::Layout,
        order: 101,
        variants: GRID_VARIANTS,
        bool_props: &[],
        build_demo: grid_demo,
    },
    ComponentEntry {
        name: "Container",
        slug: "container",
        description: "Constrained-width content wrapper.",
        category: Category::Layout,
        order: 102,
        variants: CONTAINER_VARIANTS,
        bool_props: &[
            BoolProp {
                name: "centered",
                description: "Center horizontally",
                default: false,
            },
            BoolProp {
                name: "padding",
                description: "Add horizontal padding",
                default: false,
            },
        ],
        build_demo: container_demo,
    },
    ComponentEntry {
        name: "Card",
        slug: "card",
        description: "Surface container with padding, border, and shadow.",
        category: Category::Layout,
        order: 103,
        variants: CARD_VARIANTS,
        bool_props: &[BoolProp {
            name: "bordered",
            description: "Show border",
            default: true,
        }],
        build_demo: card_demo,
    },
    ComponentEntry {
        name: "AppShell",
        slug: "app-shell",
        description: "Application shell with header, sidebar, and main content areas.",
        category: Category::Layout,
        order: 104,
        variants: &[],
        bool_props: &[],
        build_demo: app_shell_demo,
    },
    ComponentEntry {
        name: "Spacer",
        slug: "spacer",
        description: "Creates space between elements.",
        category: Category::Layout,
        order: 105,
        variants: SPACING_SIZE_VARIANTS,
        bool_props: &[BoolProp {
            name: "horizontal",
            description: "Horizontal spacing",
            default: false,
        }],
        build_demo: spacer_demo,
    },
    ComponentEntry {
        name: "Divider",
        slug: "divider",
        description: "Horizontal or vertical separator line.",
        category: Category::Layout,
        order: 106,
        variants: &[VariantAxis {
            name: "margin",
            display_name: "Margin",
            rust_type: "SpacingSize",
            values: &[
                VariantValue {
                    label: "None",
                    rust_expr: "SpacingSize::None",
                },
                VariantValue {
                    label: "Xs",
                    rust_expr: "SpacingSize::Xs",
                },
                VariantValue {
                    label: "Sm",
                    rust_expr: "SpacingSize::Sm",
                },
                VariantValue {
                    label: "Md",
                    rust_expr: "SpacingSize::Md",
                },
                VariantValue {
                    label: "Lg",
                    rust_expr: "SpacingSize::Lg",
                },
                VariantValue {
                    label: "Xl",
                    rust_expr: "SpacingSize::Xl",
                },
            ],
            default_index: 3,
        }],
        bool_props: &[BoolProp {
            name: "vertical",
            description: "Vertical orientation",
            default: false,
        }],
        build_demo: divider_demo,
    },
    // --- Forms (order 2xx) ---
    ComponentEntry {
        name: "Button",
        slug: "button",
        description: "Trigger actions with primary, secondary, ghost, and destructive variants.",
        category: Category::Forms,
        order: 200,
        variants: BUTTON_VARIANTS,
        bool_props: &[
            BoolProp {
                name: "disabled",
                description: "Prevent interaction",
                default: false,
            },
            BoolProp {
                name: "loading",
                description: "Show loading spinner",
                default: false,
            },
            BoolProp {
                name: "full_width",
                description: "Stretch to full width",
                default: false,
            },
            BoolProp {
                name: "icon",
                description: "Show a leading icon",
                default: false,
            },
        ],
        build_demo: button_demo,
    },
    ComponentEntry {
        name: "Input",
        slug: "input",
        description: "Text inputs with various types and sizes.",
        category: Category::Forms,
        order: 201,
        variants: INPUT_VARIANTS,
        bool_props: INPUT_BOOL,
        build_demo: input_demo,
    },
    ComponentEntry {
        name: "Textarea",
        slug: "textarea",
        description: "Multi-line text input with customizable rows.",
        category: Category::Forms,
        order: 202,
        variants: TEXTAREA_VARIANTS,
        bool_props: BOOL_FORM_FULL,
        build_demo: textarea_demo,
    },
    ComponentEntry {
        name: "Select",
        slug: "select",
        description: "Dropdown select with options.",
        category: Category::Forms,
        order: 203,
        variants: &[],
        bool_props: &[
            BoolProp {
                name: "disabled",
                description: "Prevent interaction",
                default: false,
            },
            BoolProp {
                name: "required",
                description: "Mark as required",
                default: false,
            },
            BoolProp {
                name: "invalid",
                description: "Show error state",
                default: false,
            },
        ],
        build_demo: select_demo,
    },
    ComponentEntry {
        name: "Checkbox",
        slug: "checkbox",
        description: "Boolean checkbox with optional label.",
        category: Category::Forms,
        order: 204,
        variants: &[],
        bool_props: &[
            BoolProp {
                name: "checked",
                description: "Toggle checked state",
                default: false,
            },
            BoolProp {
                name: "disabled",
                description: "Prevent interaction",
                default: false,
            },
            BoolProp {
                name: "required",
                description: "Mark as required",
                default: false,
            },
            BoolProp {
                name: "invalid",
                description: "Show error state",
                default: false,
            },
        ],
        build_demo: checkbox_demo,
    },
    ComponentEntry {
        name: "Radio",
        slug: "radio",
        description: "Radio buttons for mutually exclusive options.",
        category: Category::Forms,
        order: 205,
        variants: &[],
        bool_props: BOOL_CHECK_FORM,
        build_demo: radio_demo,
    },
    ComponentEntry {
        name: "Switch",
        slug: "switch",
        description: "Toggle switch for boolean states.",
        category: Category::Forms,
        order: 206,
        variants: &[],
        bool_props: BOOL_CHECK_FORM,
        build_demo: switch_demo,
    },
    ComponentEntry {
        name: "Slider",
        slug: "slider",
        description: "Range slider for numeric values.",
        category: Category::Forms,
        order: 207,
        variants: &[],
        bool_props: BOOL_DISABLED,
        build_demo: slider_demo,
    },
    ComponentEntry {
        name: "Label",
        slug: "label",
        description: "Form labels with optional required indicator.",
        category: Category::Forms,
        order: 208,
        variants: &[],
        bool_props: &[BoolProp {
            name: "required",
            description: "Show required indicator",
            default: false,
        }],
        build_demo: label_demo,
    },
    ComponentEntry {
        name: "Composer",
        slug: "composer",
        description: "Chat message bar: auto-growing field, Enter submits, Shift+Enter newline.",
        category: Category::Forms,
        order: 211,
        variants: &[],
        bool_props: &[BoolProp {
            name: "compact",
            description: "Single-row form factor (inline composers)",
            default: false,
        }],
        build_demo: composer_demo,
    },
    ComponentEntry {
        name: "FormField",
        slug: "form-field",
        description: "Composition wrapper with label, input, help text, and validation.",
        category: Category::Forms,
        order: 209,
        variants: &[],
        bool_props: &[
            BoolProp {
                name: "required",
                description: "Mark as required",
                default: false,
            },
            BoolProp {
                name: "error",
                description: "Show error message",
                default: false,
            },
        ],
        build_demo: form_field_demo,
    },
    // --- Data Display (order 3xx) ---
    ComponentEntry {
        name: "Text",
        slug: "text",
        description: "Typography component with semantic variants and colors.",
        category: Category::DataDisplay,
        order: 300,
        variants: TEXT_VARIANTS,
        bool_props: &[],
        build_demo: text_demo,
    },
    ComponentEntry {
        name: "Badge",
        slug: "badge",
        description: "Status indicators and labels.",
        category: Category::DataDisplay,
        order: 301,
        variants: BADGE_VARIANTS,
        bool_props: &[],
        build_demo: badge_demo,
    },
    ComponentEntry {
        name: "StatusDot",
        slug: "status-dot",
        description: "Presence/status dot with optional pulse and inline label.",
        category: Category::DataDisplay,
        order: 302,
        variants: STATUS_DOT_VARIANTS,
        bool_props: &[
            BoolProp {
                name: "pulse",
                description: "Pulse while live",
                default: true,
            },
            BoolProp {
                name: "label",
                description: "Show an inline label",
                default: false,
            },
        ],
        build_demo: status_dot_demo,
    },
    ComponentEntry {
        name: "Tag",
        slug: "tag",
        description: "Categorization labels with optional remove action.",
        category: Category::DataDisplay,
        order: 302,
        variants: TAG_VARIANTS,
        bool_props: &[BoolProp {
            name: "removable",
            description: "Show remove button",
            default: false,
        }],
        build_demo: tag_demo,
    },
    ComponentEntry {
        name: "Code",
        slug: "code",
        description: "Inline and block code display.",
        category: Category::DataDisplay,
        order: 303,
        variants: CODE_VARIANTS,
        bool_props: &[],
        build_demo: code_demo,
    },
    ComponentEntry {
        name: "Kbd",
        slug: "kbd",
        description: "Keyboard shortcut indicators.",
        category: Category::DataDisplay,
        order: 304,
        variants: &[],
        bool_props: &[],
        build_demo: kbd_demo,
    },
    ComponentEntry {
        name: "Table",
        slug: "table",
        description: "Data tables with headers and rows.",
        category: Category::DataDisplay,
        order: 305,
        variants: &[],
        bool_props: &[BoolProp {
            name: "striped",
            description: "Alternate row colors",
            default: false,
        }],
        build_demo: table_demo,
    },
    ComponentEntry {
        name: "Stat",
        slug: "stat",
        description: "Key metric display with trend indicator.",
        category: Category::DataDisplay,
        order: 306,
        variants: STAT_VARIANTS,
        bool_props: &[],
        build_demo: stat_demo,
    },
    ComponentEntry {
        name: "Avatar",
        slug: "avatar",
        description: "User avatars with image or fallback text.",
        category: Category::DataDisplay,
        order: 307,
        variants: AVATAR_SIZE,
        bool_props: &[],
        build_demo: avatar_demo,
    },
    ComponentEntry {
        name: "AvatarGroup",
        slug: "avatar-group",
        description: "Group of avatars with overflow count.",
        category: Category::DataDisplay,
        order: 308,
        variants: &[],
        bool_props: &[],
        build_demo: avatar_group_demo,
    },
    ComponentEntry {
        name: "Image",
        slug: "image",
        description: "Responsive images with fit and aspect controls.",
        category: Category::DataDisplay,
        order: 309,
        variants: IMAGE_VARIANTS,
        bool_props: &[
            BoolProp {
                name: "rounded",
                description: "Round corners",
                default: false,
            },
            BoolProp {
                name: "full_width",
                description: "Stretch to full width",
                default: false,
            },
        ],
        build_demo: image_demo,
    },
    ComponentEntry {
        name: "List",
        slug: "list",
        description: "Ordered and unordered lists.",
        category: Category::DataDisplay,
        order: 310,
        variants: &[],
        bool_props: &[BoolProp {
            name: "is_ordered",
            description: "Show as ordered list",
            default: false,
        }],
        build_demo: list_demo,
    },
    ComponentEntry {
        name: "Blockquote",
        slug: "blockquote",
        description: "Quoted text with optional citation.",
        category: Category::DataDisplay,
        order: 311,
        variants: &[],
        bool_props: &[],
        build_demo: blockquote_demo,
    },
    ComponentEntry {
        name: "Timeline",
        slug: "timeline",
        description: "Vertical timeline of events.",
        category: Category::DataDisplay,
        order: 312,
        variants: &[],
        bool_props: &[],
        build_demo: timeline_demo,
    },
    // --- Feedback (order 4xx) ---
    ComponentEntry {
        name: "Alert",
        slug: "alert",
        description: "Alert messages with different intent levels.",
        category: Category::Feedback,
        order: 400,
        variants: ALERT_VARIANTS,
        bool_props: &[],
        build_demo: alert_demo,
    },
    ComponentEntry {
        name: "Toast",
        slug: "toast",
        description: "Temporary notification messages.",
        category: Category::Feedback,
        order: 401,
        variants: TOAST_VARIANTS,
        bool_props: &[],
        build_demo: toast_demo,
    },
    ComponentEntry {
        name: "Spinner",
        slug: "spinner",
        description: "Loading indicators with size variants.",
        category: Category::Feedback,
        order: 402,
        variants: SPINNER_SIZE,
        bool_props: &[],
        build_demo: spinner_demo,
    },
    ComponentEntry {
        name: "Progress",
        slug: "progress",
        description: "Progress bars showing task completion.",
        category: Category::Feedback,
        order: 403,
        variants: PROGRESS_SIZE,
        bool_props: &[],
        build_demo: progress_demo,
    },
    ComponentEntry {
        name: "Skeleton",
        slug: "skeleton",
        description: "Loading placeholder that shows content shape.",
        category: Category::Feedback,
        order: 404,
        variants: SKELETON_VARIANTS,
        bool_props: &[],
        build_demo: skeleton_demo,
    },
    ComponentEntry {
        name: "EmptyState",
        slug: "empty-state",
        description: "Placeholder for empty content areas.",
        category: Category::Feedback,
        order: 405,
        variants: &[],
        bool_props: &[],
        build_demo: empty_state_demo,
    },
    ComponentEntry {
        name: "Tooltip",
        slug: "tooltip",
        description: "Contextual information on hover.",
        category: Category::Feedback,
        order: 406,
        variants: TOOLTIP_VARIANTS,
        bool_props: &[],
        build_demo: tooltip_demo,
    },
    // --- Navigation (order 5xx) ---
    ComponentEntry {
        name: "Link",
        slug: "link",
        description: "Navigation links with internal and external variants.",
        category: Category::Navigation,
        order: 500,
        variants: &[],
        bool_props: &[BoolProp {
            name: "external",
            description: "Open in new tab",
            default: false,
        }],
        build_demo: link_demo,
    },
    ComponentEntry {
        name: "NavMenu",
        slug: "nav-menu",
        description: "Navigation menu with active state.",
        category: Category::Navigation,
        order: 501,
        variants: &[],
        bool_props: &[],
        build_demo: nav_menu_demo,
    },
    ComponentEntry {
        name: "Breadcrumb",
        slug: "breadcrumb",
        description: "Navigation breadcrumb trails.",
        category: Category::Navigation,
        order: 502,
        variants: &[],
        bool_props: &[],
        build_demo: breadcrumb_demo,
    },
    ComponentEntry {
        name: "Tabs",
        slug: "tabs",
        description: "Tab navigation for content sections.",
        category: Category::Navigation,
        order: 503,
        variants: &[],
        bool_props: &[],
        build_demo: tabs_demo,
    },
    ComponentEntry {
        name: "StreamedContent",
        slug: "streamed-content",
        description:
            "Progressive content delivery: chunks stream in as a sentinel nears the viewport.",
        category: Category::DataDisplay,
        order: 313,
        variants: &[],
        bool_props: &[BoolProp {
            name: "loading",
            description: "More chunks remain; the sentinel spinner row is visible",
            default: true,
        }],
        build_demo: streamed_content_demo,
    },
    ComponentEntry {
        name: "Chip",
        slug: "chip",
        description: "Selectable chip for filters, view toggles, and inline pickers.",
        category: Category::Navigation,
        order: 504,
        variants: &[],
        bool_props: &[BoolProp {
            name: "active",
            description: "The chip is the current selection",
            default: true,
        }],
        build_demo: chip_demo,
    },
    ComponentEntry {
        name: "Pagination",
        slug: "pagination",
        description: "Page navigation for lists and tables.",
        category: Category::Navigation,
        order: 504,
        variants: &[],
        bool_props: &[],
        build_demo: pagination_demo,
    },
    ComponentEntry {
        name: "Footer",
        slug: "footer",
        description: "Page footer with logo, links, and copyright.",
        category: Category::Navigation,
        order: 505,
        variants: &[],
        bool_props: &[],
        build_demo: footer_demo,
    },
    // --- Overlay (order 6xx) ---
    ComponentEntry {
        name: "Modal",
        slug: "modal",
        description: "Dialog overlay for confirmations and forms.",
        category: Category::Overlay,
        order: 600,
        variants: MODAL_VARIANTS,
        bool_props: MODAL_BOOLS,
        build_demo: modal_demo,
    },
    ComponentEntry {
        name: "Drawer",
        slug: "drawer",
        description: "Slide-out panel from left or right edge.",
        category: Category::Overlay,
        order: 601,
        variants: DRAWER_VARIANTS,
        bool_props: BOOL_OPEN,
        build_demo: drawer_demo,
    },
    ComponentEntry {
        name: "DropdownMenu",
        slug: "dropdown",
        description: "Dropdown menu with actions.",
        category: Category::Overlay,
        order: 602,
        variants: &[],
        bool_props: BOOL_OPEN,
        build_demo: dropdown_demo,
    },
    ComponentEntry {
        name: "Accordion",
        slug: "accordion",
        description: "Collapsible content sections.",
        category: Category::Overlay,
        order: 603,
        variants: &[],
        bool_props: &[],
        build_demo: accordion_demo,
    },
    ComponentEntry {
        name: "CopyButton",
        slug: "copy-button",
        description: "One-click text copy to clipboard.",
        category: Category::Overlay,
        order: 604,
        variants: &[],
        bool_props: &[],
        build_demo: copy_button_demo,
    },
    ComponentEntry {
        name: "ThemeToggle",
        slug: "theme-toggle",
        description: "Toggle between light and dark modes.",
        category: Category::Overlay,
        order: 605,
        variants: THEME_TOGGLE_VARIANTS,
        bool_props: THEME_TOGGLE_BOOLS,
        build_demo: theme_toggle_demo,
    },
    ComponentEntry {
        name: "Stepper",
        slug: "stepper",
        description: "Multi-step progress indicator.",
        category: Category::Overlay,
        order: 606,
        variants: &[],
        bool_props: &[],
        build_demo: stepper_demo,
    },
];

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod codegen_tests {
    use super::*;

    /// Parentheses + string literals are balanced (catches truncated/garbled snippets).
    fn balanced(s: &str) -> bool {
        let (mut paren, mut bracket) = (0i32, 0i32);
        let mut in_str = false;
        let mut prev = ' ';
        for ch in s.chars() {
            if in_str {
                if ch == '"' && prev != '\\' {
                    in_str = false;
                }
            } else {
                match ch {
                    '"' => in_str = true,
                    '(' => paren += 1,
                    ')' => paren -= 1,
                    '[' => bracket += 1,
                    ']' => bracket -= 1,
                    _ => {}
                }
                if paren < 0 || bracket < 0 {
                    return false;
                }
            }
            prev = ch;
        }
        paren == 0 && bracket == 0 && !in_str
    }

    /// Every variant value must carry a non-empty `rust_type`/`rust_expr` — an empty one
    /// generates an argument-less `.method()` call that does not compile (the old
    /// `SM_MD_LG_SIZE` bug). Guards against that class of regression.
    #[test]
    fn variant_exprs_are_non_empty() {
        for entry in catalog() {
            for axis in entry.variants {
                assert!(
                    !axis.rust_type.is_empty(),
                    "{} axis `{}` has an empty rust_type",
                    entry.slug,
                    axis.name
                );
                for val in axis.values {
                    assert!(
                        !val.rust_expr.is_empty(),
                        "{} axis `{}` value `{}` has an empty rust_expr",
                        entry.slug,
                        axis.name,
                        val.label
                    );
                }
            }
        }
    }

    /// Generate every component's snippet across each variant value (one axis varied at a
    /// time) and both bool extremes, asserting it is structurally well-formed Rust.
    #[test]
    fn every_snippet_is_well_formed() {
        for entry in catalog() {
            // variant combinations: defaults, plus each axis swept through all its values.
            let defaults: Vec<usize> = entry.variants.iter().map(|a| a.default_index).collect();
            let mut combos = vec![defaults.clone()];
            for (ai, axis) in entry.variants.iter().enumerate() {
                for vi in 0..axis.values.len() {
                    let mut sel = defaults.clone();
                    sel[ai] = vi;
                    combos.push(sel);
                }
            }
            let nums: Vec<i32> = num_props(entry.slug).iter().map(|n| n.default).collect();
            let texts: Vec<&str> = text_props(entry.slug).iter().map(|t| t.default).collect();

            for all in [false, true] {
                let bools: Vec<bool> = entry.bool_props.iter().map(|_| all).collect();
                for variants in &combos {
                    let code = generate_code(
                        entry,
                        &CodeCtx {
                            variants,
                            nums: &nums,
                            texts: &texts,
                            bools: &bools,
                        },
                    );
                    assert!(
                        code.ends_with(".build()"),
                        "{}: snippet must end with .build():\n{code}",
                        entry.slug
                    );
                    assert!(
                        code.contains("::"),
                        "{}: snippet must reference a type:\n{code}",
                        entry.slug
                    );
                    assert!(
                        balanced(&code),
                        "{}: snippet has unbalanced delimiters:\n{code}",
                        entry.slug
                    );
                }
            }
        }
    }
}
