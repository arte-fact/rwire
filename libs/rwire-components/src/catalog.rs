//! Component catalog with metadata for auto-generating documentation.
//!
//! Each component self-describes its variants, boolean props, and provides
//! a `build_demo` function that renders a live preview from selections.

use rwire::{el, El, ElementBuilder};

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
// Layout: Stack, Grid, Container, Card, AppShell, Spacer, Divider
// ============================================================================

fn stack_demo(variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{Gap, Stack, StackAlign, StackDirection, StackJustify};
    let dir = match v(variants, 0) {
        1 => StackDirection::Row,
        _ => StackDirection::Column,
    };
    let gap = match v(variants, 1) {
        0 => Gap::None, 1 => Gap::Xs, 2 => Gap::Sm,
        4 => Gap::Lg, 5 => Gap::Xl, _ => Gap::Md,
    };
    let align = match v(variants, 2) {
        1 => StackAlign::Start, 2 => StackAlign::Center,
        3 => StackAlign::End, _ => StackAlign::Stretch,
    };
    let justify = match v(variants, 3) {
        1 => StackJustify::Center, 2 => StackJustify::End,
        3 => StackJustify::Between, 4 => StackJustify::Around,
        _ => StackJustify::Start,
    };
    let wrap = b(bools, 0);
    let item = || {
        el(El::Div)
            .st([rwire::St::BgMuted, rwire::St::RoundedMd, rwire::St::PMd, rwire::St::TextSm])
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
        0 => GridColumns::Auto, 1 => GridColumns::Fixed1, 2 => GridColumns::Fixed2,
        4 => GridColumns::Fixed4, _ => GridColumns::Fixed3,
    };
    let gap = match v(variants, 1) {
        0 => Gap::None, 1 => Gap::Xs, 2 => Gap::Sm,
        4 => Gap::Lg, 5 => Gap::Xl, _ => Gap::Md,
    };
    let item = |n: &str| {
        el(El::Div)
            .st([rwire::St::BgMuted, rwire::St::RoundedMd, rwire::St::PMd, rwire::St::TextSm])
            .text(n)
    };
    Grid::new()
        .columns(cols)
        .gap(gap)
        .children([item("1"), item("2"), item("3"), item("4"), item("5"), item("6")])
        .build()
}

fn container_demo(variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{Container, ContainerSize};
    let size = match v(variants, 0) {
        0 => ContainerSize::Sm, 1 => ContainerSize::Md, 2 => ContainerSize::Lg,
        3 => ContainerSize::Xl, 4 => ContainerSize::Full, _ => ContainerSize::Md,
    };
    Container::new()
        .size(size)
        .centered(b(bools, 0))
        .padding(b(bools, 1))
        .child(
            el(El::Div)
                .st([rwire::St::BgMuted, rwire::St::RoundedMd, rwire::St::PMd, rwire::St::TextSm])
                .text("Container content"),
        )
        .build()
}

fn card_demo(variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{Card, CardPadding, CardShadow};
    let padding = match v(variants, 0) {
        0 => CardPadding::None, 1 => CardPadding::Sm,
        3 => CardPadding::Lg, _ => CardPadding::Md,
    };
    let shadow = match v(variants, 1) {
        0 => CardShadow::None, 2 => CardShadow::Md,
        3 => CardShadow::Lg, _ => CardShadow::Sm,
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
        0 => SpacingSize::None, 1 => SpacingSize::Xs, 2 => SpacingSize::Sm,
        4 => SpacingSize::Lg, 5 => SpacingSize::Xl, _ => SpacingSize::Md,
    };
    let horizontal = b(bools, 0);
    let mut spacer = Spacer::new(size);
    if horizontal {
        spacer = spacer.horizontal();
    }
    let line = || {
        el(El::Div)
            .st([rwire::St::BgMuted, rwire::St::RoundedMd, rwire::St::PMd, rwire::St::TextSm])
            .text("Content")
    };
    if horizontal {
        el(El::Div)
            .st([rwire::St::DisplayFlex, rwire::St::FlexRow, rwire::St::ItemsCenter])
            .append([line(), spacer.build(), line()])
    } else {
        el(El::Div).append([line(), spacer.build(), line()])
    }
}

fn divider_demo(variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{Divider, SpacingSize};
    let margin = match v(variants, 0) {
        0 => SpacingSize::None, 1 => SpacingSize::Xs, 2 => SpacingSize::Sm,
        4 => SpacingSize::Lg, 5 => SpacingSize::Xl, _ => SpacingSize::Md,
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
            .st([rwire::St::DisplayFlex, rwire::St::FlexRow, rwire::St::ItemsCenter, rwire::St::HFull])
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
        1 => ButtonIntent::Secondary, 2 => ButtonIntent::Ghost,
        3 => ButtonIntent::Destructive, _ => ButtonIntent::Primary,
    };
    let size = match v(variants, 1) {
        0 => ButtonSize::Sm, 2 => ButtonSize::Lg, _ => ButtonSize::Md,
    };
    Button::new()
        .intent(intent)
        .size(size)
        .text("Button")
        .disabled(b(bools, 0))
        .loading(b(bools, 1))
        .full_width(b(bools, 2))
        .build()
}

fn input_demo(variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{Input, InputSize, InputType};
    let input_type = match v(variants, 0) {
        1 => InputType::Password, 2 => InputType::Email,
        3 => InputType::Number, 4 => InputType::Search,
        5 => InputType::Tel, 6 => InputType::Url,
        _ => InputType::Text,
    };
    let size = match v(variants, 1) {
        0 => InputSize::Sm, 2 => InputSize::Lg, _ => InputSize::Md,
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
    use crate::Textarea;
    use crate::textarea::TextareaSize;
    let size = match v(variants, 0) {
        0 => TextareaSize::Sm, 2 => TextareaSize::Lg, _ => TextareaSize::Md,
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
    use crate::{Radio, Stack, Gap};
    Stack::column()
        .gap(Gap::Sm)
        .children([
            Radio::new()
                .name("plan").value("free").label("Free Plan")
                .checked(!b(bools, 0)).disabled(b(bools, 1)).build(),
            Radio::new()
                .name("plan").value("pro").label("Pro Plan")
                .checked(b(bools, 0)).disabled(b(bools, 1)).build(),
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
        .min(0).max(100).value(50).step(1)
        .label("Volume")
        .disabled(b(bools, 0))
        .build()
}

fn label_demo(_variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::Label;
    Label::new("Email address")
        .required(b(bools, 0))
        .build()
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
        1 => TextVariant::BodySmall, 2 => TextVariant::Heading1,
        3 => TextVariant::Heading2, 4 => TextVariant::Heading3,
        5 => TextVariant::Label, 6 => TextVariant::Caption,
        _ => TextVariant::Body,
    };
    let color = match v(variants, 1) {
        1 => TextColor::High, 2 => TextColor::Muted, 3 => TextColor::Accent,
        4 => TextColor::Success, 5 => TextColor::Warning, 6 => TextColor::Error,
        _ => TextColor::Default,
    };
    Text::new()
        .variant(variant)
        .color(color)
        .content("The quick brown fox jumps over the lazy dog.")
        .build()
}

fn badge_demo(variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{Badge, BadgeIntent};
    let intent = match v(variants, 0) {
        1 => BadgeIntent::Primary, 2 => BadgeIntent::Success,
        3 => BadgeIntent::Warning, 4 => BadgeIntent::Error,
        _ => BadgeIntent::Default,
    };
    Badge::new().intent(intent).text("Badge").build()
}

fn tag_demo(variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{Tag, TagIntent};
    let intent = match v(variants, 0) {
        1 => TagIntent::Primary, 2 => TagIntent::Success,
        3 => TagIntent::Warning, 4 => TagIntent::Error,
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
        1 => Code::block("fn main() {\n    println!(\"Hello, world!\");\n}").language("rust").build(),
        _ => Code::inline("cargo run").build(),
    }
}

fn kbd_demo(_variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{Kbd, Stack, Gap};
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
        .row(TableRow::new().cell("Alice").cell("Engineer").cell("Active"))
        .row(TableRow::new().cell("Bob").cell("Designer").cell("Active"))
        .row(TableRow::new().cell("Carol").cell("Manager").cell("Away"))
        .striped(b(bools, 0))
        .build()
}

fn stat_demo(variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{Stat, StatTrend};
    let trend = match v(variants, 0) {
        1 => StatTrend::Down, 2 => StatTrend::Neutral, _ => StatTrend::Up,
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
        0 => AvatarSize::Sm, 2 => AvatarSize::Lg, _ => AvatarSize::Md,
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
        1 => ImageFit::Contain, 2 => ImageFit::Fill, 3 => ImageFit::None,
        _ => ImageFit::Cover,
    };
    let mut img = Image::new("https://picsum.photos/400/300")
        .alt("Sample image")
        .fit(fit);
    if b(bools, 0) { img = img.rounded(); }
    if b(bools, 1) { img = img.full_width(); }
    img.build()
}

fn list_demo(_variants: &[usize], bools: &[bool]) -> ElementBuilder {
    use crate::{List, ListItem};
    let mut list = if b(bools, 0) { List::ordered() } else { List::unordered() };
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
        .item(TimelineItem::new("Order placed").description("Jan 1, 2026").active(true))
        .item(TimelineItem::new("Shipped").description("Jan 3, 2026").active(true))
        .item(TimelineItem::new("Delivered").description("Pending"))
        .build()
}

// ============================================================================
// Feedback: Alert, Toast, Spinner, Progress, Skeleton, EmptyState, Tooltip
// ============================================================================

fn alert_demo(variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{Alert, AlertIntent};
    let intent = match v(variants, 0) {
        1 => AlertIntent::Success, 2 => AlertIntent::Warning,
        3 => AlertIntent::Error, _ => AlertIntent::Info,
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
        1 => ToastIntent::Success, 2 => ToastIntent::Warning,
        3 => ToastIntent::Error, _ => ToastIntent::Info,
    };
    Toast::new("Toast notification message")
        .intent(intent)
        .build()
}

fn spinner_demo(variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{Spinner, SpinnerSize};
    let size = match v(variants, 0) {
        0 => SpinnerSize::Sm, 2 => SpinnerSize::Lg, _ => SpinnerSize::Md,
    };
    Spinner::new().size(size).label("Loading...").build()
}

fn progress_demo(_variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::Progress;
    Progress::new().value(65).max(100).label("Upload progress").build()
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
        1 => TooltipPosition::Bottom, 2 => TooltipPosition::Left,
        3 => TooltipPosition::Right, _ => TooltipPosition::Top,
    };
    Tooltip::new("Tooltip text")
        .position(position)
        .child(
            el(El::Button)
                .st([
                    rwire::St::BgPrimary, rwire::St::TextOnPrimary,
                    rwire::St::PxMd, rwire::St::PySm, rwire::St::RoundedMd,
                    rwire::St::BorderNone, rwire::St::CursorPointer,
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
        Link::external("https://example.com").text("External Link").build()
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
    Pagination::new()
        .current_page(3)
        .total_pages(10)
        .build()
}

fn footer_demo(_variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{Footer, FooterColumn};
    Footer::new()
        .logo(el(El::Span).st([rwire::St::TextLg, rwire::St::FontBold]).text("Brand"))
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
        0 => ModalSize::Sm, 2 => ModalSize::Lg,
        3 => ModalSize::Xl, 4 => ModalSize::Full,
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

fn theme_toggle_demo(variants: &[usize], _bools: &[bool]) -> ElementBuilder {
    use crate::{ThemeToggle, ThemeToggleMode, ToggleSize};
    let size = match v(variants, 0) {
        0 => ToggleSize::Sm, 2 => ToggleSize::Lg, _ => ToggleSize::Md,
    };
    ThemeToggle::new()
        .size(size)
        .mode(ThemeToggleMode::Light)
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
        name: "direction", display_name: "Direction", rust_type: "StackDirection",
        values: &[
            VariantValue { label: "Column", rust_expr: "StackDirection::Column" },
            VariantValue { label: "Row", rust_expr: "StackDirection::Row" },
        ],
        default_index: 0,
    },
    VariantAxis {
        name: "gap", display_name: "Gap", rust_type: "Gap",
        values: &[
            VariantValue { label: "None", rust_expr: "Gap::None" },
            VariantValue { label: "Xs", rust_expr: "Gap::Xs" },
            VariantValue { label: "Sm", rust_expr: "Gap::Sm" },
            VariantValue { label: "Md", rust_expr: "Gap::Md" },
            VariantValue { label: "Lg", rust_expr: "Gap::Lg" },
            VariantValue { label: "Xl", rust_expr: "Gap::Xl" },
        ],
        default_index: 3,
    },
    VariantAxis {
        name: "align", display_name: "Align", rust_type: "StackAlign",
        values: &[
            VariantValue { label: "Stretch", rust_expr: "StackAlign::Stretch" },
            VariantValue { label: "Start", rust_expr: "StackAlign::Start" },
            VariantValue { label: "Center", rust_expr: "StackAlign::Center" },
            VariantValue { label: "End", rust_expr: "StackAlign::End" },
        ],
        default_index: 0,
    },
    VariantAxis {
        name: "justify", display_name: "Justify", rust_type: "StackJustify",
        values: &[
            VariantValue { label: "Start", rust_expr: "StackJustify::Start" },
            VariantValue { label: "Center", rust_expr: "StackJustify::Center" },
            VariantValue { label: "End", rust_expr: "StackJustify::End" },
            VariantValue { label: "Between", rust_expr: "StackJustify::Between" },
            VariantValue { label: "Around", rust_expr: "StackJustify::Around" },
        ],
        default_index: 0,
    },
];

const GRID_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "columns", display_name: "Columns", rust_type: "GridColumns",
        values: &[
            VariantValue { label: "Auto", rust_expr: "GridColumns::Auto" },
            VariantValue { label: "1", rust_expr: "GridColumns::Fixed1" },
            VariantValue { label: "2", rust_expr: "GridColumns::Fixed2" },
            VariantValue { label: "3", rust_expr: "GridColumns::Fixed3" },
            VariantValue { label: "4", rust_expr: "GridColumns::Fixed4" },
        ],
        default_index: 3,
    },
    VariantAxis {
        name: "gap", display_name: "Gap", rust_type: "Gap",
        values: &[
            VariantValue { label: "None", rust_expr: "Gap::None" },
            VariantValue { label: "Xs", rust_expr: "Gap::Xs" },
            VariantValue { label: "Sm", rust_expr: "Gap::Sm" },
            VariantValue { label: "Md", rust_expr: "Gap::Md" },
            VariantValue { label: "Lg", rust_expr: "Gap::Lg" },
            VariantValue { label: "Xl", rust_expr: "Gap::Xl" },
        ],
        default_index: 3,
    },
];

const CONTAINER_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "size", display_name: "Size", rust_type: "ContainerSize",
        values: &[
            VariantValue { label: "Sm", rust_expr: "ContainerSize::Sm" },
            VariantValue { label: "Md", rust_expr: "ContainerSize::Md" },
            VariantValue { label: "Lg", rust_expr: "ContainerSize::Lg" },
            VariantValue { label: "Xl", rust_expr: "ContainerSize::Xl" },
            VariantValue { label: "Full", rust_expr: "ContainerSize::Full" },
        ],
        default_index: 1,
    },
];

const CARD_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "padding", display_name: "Padding", rust_type: "CardPadding",
        values: &[
            VariantValue { label: "None", rust_expr: "CardPadding::None" },
            VariantValue { label: "Sm", rust_expr: "CardPadding::Sm" },
            VariantValue { label: "Md", rust_expr: "CardPadding::Md" },
            VariantValue { label: "Lg", rust_expr: "CardPadding::Lg" },
        ],
        default_index: 2,
    },
    VariantAxis {
        name: "shadow", display_name: "Shadow", rust_type: "CardShadow",
        values: &[
            VariantValue { label: "None", rust_expr: "CardShadow::None" },
            VariantValue { label: "Sm", rust_expr: "CardShadow::Sm" },
            VariantValue { label: "Md", rust_expr: "CardShadow::Md" },
            VariantValue { label: "Lg", rust_expr: "CardShadow::Lg" },
        ],
        default_index: 1,
    },
];

const SPACING_SIZE_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "size", display_name: "Size", rust_type: "SpacingSize",
        values: &[
            VariantValue { label: "None", rust_expr: "SpacingSize::None" },
            VariantValue { label: "Xs", rust_expr: "SpacingSize::Xs" },
            VariantValue { label: "Sm", rust_expr: "SpacingSize::Sm" },
            VariantValue { label: "Md", rust_expr: "SpacingSize::Md" },
            VariantValue { label: "Lg", rust_expr: "SpacingSize::Lg" },
            VariantValue { label: "Xl", rust_expr: "SpacingSize::Xl" },
        ],
        default_index: 3,
    },
];

// --- Forms ---

const BUTTON_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "intent", display_name: "Intent", rust_type: "ButtonIntent",
        values: &[
            VariantValue { label: "Primary", rust_expr: "ButtonIntent::Primary" },
            VariantValue { label: "Secondary", rust_expr: "ButtonIntent::Secondary" },
            VariantValue { label: "Ghost", rust_expr: "ButtonIntent::Ghost" },
            VariantValue { label: "Destructive", rust_expr: "ButtonIntent::Destructive" },
        ],
        default_index: 0,
    },
    VariantAxis {
        name: "size", display_name: "Size", rust_type: "ButtonSize",
        values: &[
            VariantValue { label: "Sm", rust_expr: "ButtonSize::Sm" },
            VariantValue { label: "Md", rust_expr: "ButtonSize::Md" },
            VariantValue { label: "Lg", rust_expr: "ButtonSize::Lg" },
        ],
        default_index: 1,
    },
];

const INPUT_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "type", display_name: "Type", rust_type: "InputType",
        values: &[
            VariantValue { label: "Text", rust_expr: "InputType::Text" },
            VariantValue { label: "Password", rust_expr: "InputType::Password" },
            VariantValue { label: "Email", rust_expr: "InputType::Email" },
            VariantValue { label: "Number", rust_expr: "InputType::Number" },
            VariantValue { label: "Search", rust_expr: "InputType::Search" },
            VariantValue { label: "Tel", rust_expr: "InputType::Tel" },
            VariantValue { label: "Url", rust_expr: "InputType::Url" },
        ],
        default_index: 0,
    },
    VariantAxis {
        name: "size", display_name: "Size", rust_type: "InputSize",
        values: &[
            VariantValue { label: "Sm", rust_expr: "InputSize::Sm" },
            VariantValue { label: "Md", rust_expr: "InputSize::Md" },
            VariantValue { label: "Lg", rust_expr: "InputSize::Lg" },
        ],
        default_index: 1,
    },
];

const TEXTAREA_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "size", display_name: "Size", rust_type: "TextareaSize",
        values: &[
            VariantValue { label: "Sm", rust_expr: "TextareaSize::Sm" },
            VariantValue { label: "Md", rust_expr: "TextareaSize::Md" },
            VariantValue { label: "Lg", rust_expr: "TextareaSize::Lg" },
        ],
        default_index: 1,
    },
];

// --- Data Display ---

const TEXT_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "variant", display_name: "Variant", rust_type: "TextVariant",
        values: &[
            VariantValue { label: "Body", rust_expr: "TextVariant::Body" },
            VariantValue { label: "BodySmall", rust_expr: "TextVariant::BodySmall" },
            VariantValue { label: "Heading1", rust_expr: "TextVariant::Heading1" },
            VariantValue { label: "Heading2", rust_expr: "TextVariant::Heading2" },
            VariantValue { label: "Heading3", rust_expr: "TextVariant::Heading3" },
            VariantValue { label: "Label", rust_expr: "TextVariant::Label" },
            VariantValue { label: "Caption", rust_expr: "TextVariant::Caption" },
        ],
        default_index: 0,
    },
    VariantAxis {
        name: "color", display_name: "Color", rust_type: "TextColor",
        values: &[
            VariantValue { label: "Default", rust_expr: "TextColor::Default" },
            VariantValue { label: "High", rust_expr: "TextColor::High" },
            VariantValue { label: "Muted", rust_expr: "TextColor::Muted" },
            VariantValue { label: "Accent", rust_expr: "TextColor::Accent" },
            VariantValue { label: "Success", rust_expr: "TextColor::Success" },
            VariantValue { label: "Warning", rust_expr: "TextColor::Warning" },
            VariantValue { label: "Error", rust_expr: "TextColor::Error" },
        ],
        default_index: 0,
    },
];

const BADGE_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "intent", display_name: "Intent", rust_type: "BadgeIntent",
        values: &[
            VariantValue { label: "Default", rust_expr: "BadgeIntent::Default" },
            VariantValue { label: "Primary", rust_expr: "BadgeIntent::Primary" },
            VariantValue { label: "Success", rust_expr: "BadgeIntent::Success" },
            VariantValue { label: "Warning", rust_expr: "BadgeIntent::Warning" },
            VariantValue { label: "Error", rust_expr: "BadgeIntent::Error" },
        ],
        default_index: 0,
    },
];

const TAG_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "intent", display_name: "Intent", rust_type: "TagIntent",
        values: &[
            VariantValue { label: "Default", rust_expr: "TagIntent::Default" },
            VariantValue { label: "Primary", rust_expr: "TagIntent::Primary" },
            VariantValue { label: "Success", rust_expr: "TagIntent::Success" },
            VariantValue { label: "Warning", rust_expr: "TagIntent::Warning" },
            VariantValue { label: "Error", rust_expr: "TagIntent::Error" },
        ],
        default_index: 0,
    },
];

const CODE_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "mode", display_name: "Mode", rust_type: "CodeMode",
        values: &[
            VariantValue { label: "Inline", rust_expr: "CodeMode::Inline" },
            VariantValue { label: "Block", rust_expr: "CodeMode::Block" },
        ],
        default_index: 0,
    },
];

const STAT_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "trend", display_name: "Trend", rust_type: "StatTrend",
        values: &[
            VariantValue { label: "Up", rust_expr: "StatTrend::Up" },
            VariantValue { label: "Down", rust_expr: "StatTrend::Down" },
            VariantValue { label: "Neutral", rust_expr: "StatTrend::Neutral" },
        ],
        default_index: 0,
    },
];

const SM_MD_LG_SIZE: &[VariantAxis] = &[
    VariantAxis {
        name: "size", display_name: "Size", rust_type: "",
        values: &[
            VariantValue { label: "Sm", rust_expr: "" },
            VariantValue { label: "Md", rust_expr: "" },
            VariantValue { label: "Lg", rust_expr: "" },
        ],
        default_index: 1,
    },
];

const IMAGE_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "fit", display_name: "Fit", rust_type: "ImageFit",
        values: &[
            VariantValue { label: "Cover", rust_expr: "ImageFit::Cover" },
            VariantValue { label: "Contain", rust_expr: "ImageFit::Contain" },
            VariantValue { label: "Fill", rust_expr: "ImageFit::Fill" },
            VariantValue { label: "None", rust_expr: "ImageFit::None" },
        ],
        default_index: 0,
    },
];

// --- Feedback ---

const ALERT_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "intent", display_name: "Intent", rust_type: "AlertIntent",
        values: &[
            VariantValue { label: "Info", rust_expr: "AlertIntent::Info" },
            VariantValue { label: "Success", rust_expr: "AlertIntent::Success" },
            VariantValue { label: "Warning", rust_expr: "AlertIntent::Warning" },
            VariantValue { label: "Error", rust_expr: "AlertIntent::Error" },
        ],
        default_index: 0,
    },
];

const TOAST_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "intent", display_name: "Intent", rust_type: "ToastIntent",
        values: &[
            VariantValue { label: "Info", rust_expr: "ToastIntent::Info" },
            VariantValue { label: "Success", rust_expr: "ToastIntent::Success" },
            VariantValue { label: "Warning", rust_expr: "ToastIntent::Warning" },
            VariantValue { label: "Error", rust_expr: "ToastIntent::Error" },
        ],
        default_index: 0,
    },
];

const SKELETON_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "shape", display_name: "Shape", rust_type: "SkeletonShape",
        values: &[
            VariantValue { label: "Text", rust_expr: "SkeletonShape::Text" },
            VariantValue { label: "Circle", rust_expr: "SkeletonShape::Circle" },
            VariantValue { label: "Rect", rust_expr: "SkeletonShape::Rect" },
        ],
        default_index: 0,
    },
];

const TOOLTIP_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "position", display_name: "Position", rust_type: "TooltipPosition",
        values: &[
            VariantValue { label: "Top", rust_expr: "TooltipPosition::Top" },
            VariantValue { label: "Bottom", rust_expr: "TooltipPosition::Bottom" },
            VariantValue { label: "Left", rust_expr: "TooltipPosition::Left" },
            VariantValue { label: "Right", rust_expr: "TooltipPosition::Right" },
        ],
        default_index: 0,
    },
];

// --- Overlay ---

const MODAL_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "size", display_name: "Size", rust_type: "ModalSize",
        values: &[
            VariantValue { label: "Sm", rust_expr: "ModalSize::Sm" },
            VariantValue { label: "Md", rust_expr: "ModalSize::Md" },
            VariantValue { label: "Lg", rust_expr: "ModalSize::Lg" },
            VariantValue { label: "Xl", rust_expr: "ModalSize::Xl" },
            VariantValue { label: "Full", rust_expr: "ModalSize::Full" },
        ],
        default_index: 1,
    },
];

const DRAWER_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "position", display_name: "Position", rust_type: "DrawerPosition",
        values: &[
            VariantValue { label: "Left", rust_expr: "DrawerPosition::Left" },
            VariantValue { label: "Right", rust_expr: "DrawerPosition::Right" },
        ],
        default_index: 0,
    },
];

const THEME_TOGGLE_VARIANTS: &[VariantAxis] = &[
    VariantAxis {
        name: "size", display_name: "Size", rust_type: "ToggleSize",
        values: &[
            VariantValue { label: "Sm", rust_expr: "ToggleSize::Sm" },
            VariantValue { label: "Md", rust_expr: "ToggleSize::Md" },
            VariantValue { label: "Lg", rust_expr: "ToggleSize::Lg" },
        ],
        default_index: 1,
    },
];

// --- Common Bool Prop Sets ---

const BOOL_DISABLED: &[BoolProp] = &[
    BoolProp { name: "disabled", description: "Prevent interaction", default: false },
];

const BOOL_FORM_FULL: &[BoolProp] = &[
    BoolProp { name: "disabled", description: "Prevent interaction", default: false },
    BoolProp { name: "readonly", description: "Read-only mode", default: false },
    BoolProp { name: "required", description: "Mark as required", default: false },
    BoolProp { name: "invalid", description: "Show error state", default: false },
];

const BOOL_CHECK_DISABLED: &[BoolProp] = &[
    BoolProp { name: "checked", description: "Toggle checked state", default: false },
    BoolProp { name: "disabled", description: "Prevent interaction", default: false },
];

const BOOL_OPEN: &[BoolProp] = &[
    BoolProp { name: "open", description: "Show component", default: false },
];

// ============================================================================
// Catalog Entries
// ============================================================================

const ENTRIES: &[ComponentEntry] = &[
    // --- Layout (order 1xx) ---
    ComponentEntry {
        name: "Stack", slug: "stack",
        description: "Flexbox layout with configurable direction and spacing.",
        category: Category::Layout, order: 100,
        variants: STACK_VARIANTS,
        bool_props: &[BoolProp { name: "wrap", description: "Enable flex wrap", default: false }],
        build_demo: stack_demo,
    },
    ComponentEntry {
        name: "Grid", slug: "grid",
        description: "CSS Grid layout with configurable columns.",
        category: Category::Layout, order: 101,
        variants: GRID_VARIANTS,
        bool_props: &[],
        build_demo: grid_demo,
    },
    ComponentEntry {
        name: "Container", slug: "container",
        description: "Constrained-width content wrapper.",
        category: Category::Layout, order: 102,
        variants: CONTAINER_VARIANTS,
        bool_props: &[
            BoolProp { name: "centered", description: "Center horizontally", default: false },
            BoolProp { name: "padding", description: "Add horizontal padding", default: false },
        ],
        build_demo: container_demo,
    },
    ComponentEntry {
        name: "Card", slug: "card",
        description: "Surface container with padding, border, and shadow.",
        category: Category::Layout, order: 103,
        variants: CARD_VARIANTS,
        bool_props: &[BoolProp { name: "bordered", description: "Show border", default: true }],
        build_demo: card_demo,
    },
    ComponentEntry {
        name: "AppShell", slug: "app-shell",
        description: "Application shell with header, sidebar, and main content areas.",
        category: Category::Layout, order: 104,
        variants: &[],
        bool_props: &[],
        build_demo: app_shell_demo,
    },
    ComponentEntry {
        name: "Spacer", slug: "spacer",
        description: "Creates space between elements.",
        category: Category::Layout, order: 105,
        variants: SPACING_SIZE_VARIANTS,
        bool_props: &[BoolProp { name: "horizontal", description: "Horizontal spacing", default: false }],
        build_demo: spacer_demo,
    },
    ComponentEntry {
        name: "Divider", slug: "divider",
        description: "Horizontal or vertical separator line.",
        category: Category::Layout, order: 106,
        variants: &[VariantAxis {
            name: "margin", display_name: "Margin", rust_type: "SpacingSize",
            values: &[
                VariantValue { label: "None", rust_expr: "SpacingSize::None" },
                VariantValue { label: "Xs", rust_expr: "SpacingSize::Xs" },
                VariantValue { label: "Sm", rust_expr: "SpacingSize::Sm" },
                VariantValue { label: "Md", rust_expr: "SpacingSize::Md" },
                VariantValue { label: "Lg", rust_expr: "SpacingSize::Lg" },
                VariantValue { label: "Xl", rust_expr: "SpacingSize::Xl" },
            ],
            default_index: 3,
        }],
        bool_props: &[BoolProp { name: "vertical", description: "Vertical orientation", default: false }],
        build_demo: divider_demo,
    },

    // --- Forms (order 2xx) ---
    ComponentEntry {
        name: "Button", slug: "button",
        description: "Trigger actions with primary, secondary, ghost, and destructive variants.",
        category: Category::Forms, order: 200,
        variants: BUTTON_VARIANTS,
        bool_props: &[
            BoolProp { name: "disabled", description: "Prevent interaction", default: false },
            BoolProp { name: "loading", description: "Show loading spinner", default: false },
            BoolProp { name: "full_width", description: "Stretch to full width", default: false },
        ],
        build_demo: button_demo,
    },
    ComponentEntry {
        name: "Input", slug: "input",
        description: "Text inputs with various types and sizes.",
        category: Category::Forms, order: 201,
        variants: INPUT_VARIANTS,
        bool_props: BOOL_FORM_FULL,
        build_demo: input_demo,
    },
    ComponentEntry {
        name: "Textarea", slug: "textarea",
        description: "Multi-line text input with customizable rows.",
        category: Category::Forms, order: 202,
        variants: TEXTAREA_VARIANTS,
        bool_props: BOOL_FORM_FULL,
        build_demo: textarea_demo,
    },
    ComponentEntry {
        name: "Select", slug: "select",
        description: "Dropdown select with options.",
        category: Category::Forms, order: 203,
        variants: &[],
        bool_props: &[
            BoolProp { name: "disabled", description: "Prevent interaction", default: false },
            BoolProp { name: "required", description: "Mark as required", default: false },
            BoolProp { name: "invalid", description: "Show error state", default: false },
        ],
        build_demo: select_demo,
    },
    ComponentEntry {
        name: "Checkbox", slug: "checkbox",
        description: "Boolean checkbox with optional label.",
        category: Category::Forms, order: 204,
        variants: &[],
        bool_props: &[
            BoolProp { name: "checked", description: "Toggle checked state", default: false },
            BoolProp { name: "disabled", description: "Prevent interaction", default: false },
            BoolProp { name: "required", description: "Mark as required", default: false },
            BoolProp { name: "invalid", description: "Show error state", default: false },
        ],
        build_demo: checkbox_demo,
    },
    ComponentEntry {
        name: "Radio", slug: "radio",
        description: "Radio buttons for mutually exclusive options.",
        category: Category::Forms, order: 205,
        variants: &[],
        bool_props: BOOL_CHECK_DISABLED,
        build_demo: radio_demo,
    },
    ComponentEntry {
        name: "Switch", slug: "switch",
        description: "Toggle switch for boolean states.",
        category: Category::Forms, order: 206,
        variants: &[],
        bool_props: BOOL_CHECK_DISABLED,
        build_demo: switch_demo,
    },
    ComponentEntry {
        name: "Slider", slug: "slider",
        description: "Range slider for numeric values.",
        category: Category::Forms, order: 207,
        variants: &[],
        bool_props: BOOL_DISABLED,
        build_demo: slider_demo,
    },
    ComponentEntry {
        name: "Label", slug: "label",
        description: "Form labels with optional required indicator.",
        category: Category::Forms, order: 208,
        variants: &[],
        bool_props: &[BoolProp { name: "required", description: "Show required indicator", default: false }],
        build_demo: label_demo,
    },
    ComponentEntry {
        name: "FormField", slug: "form-field",
        description: "Composition wrapper with label, input, help text, and validation.",
        category: Category::Forms, order: 209,
        variants: &[],
        bool_props: &[
            BoolProp { name: "required", description: "Mark as required", default: false },
            BoolProp { name: "error", description: "Show error message", default: false },
        ],
        build_demo: form_field_demo,
    },

    // --- Data Display (order 3xx) ---
    ComponentEntry {
        name: "Text", slug: "text",
        description: "Typography component with semantic variants and colors.",
        category: Category::DataDisplay, order: 300,
        variants: TEXT_VARIANTS,
        bool_props: &[],
        build_demo: text_demo,
    },
    ComponentEntry {
        name: "Badge", slug: "badge",
        description: "Status indicators and labels.",
        category: Category::DataDisplay, order: 301,
        variants: BADGE_VARIANTS,
        bool_props: &[],
        build_demo: badge_demo,
    },
    ComponentEntry {
        name: "Tag", slug: "tag",
        description: "Categorization labels with optional remove action.",
        category: Category::DataDisplay, order: 302,
        variants: TAG_VARIANTS,
        bool_props: &[BoolProp { name: "removable", description: "Show remove button", default: false }],
        build_demo: tag_demo,
    },
    ComponentEntry {
        name: "Code", slug: "code",
        description: "Inline and block code display.",
        category: Category::DataDisplay, order: 303,
        variants: CODE_VARIANTS,
        bool_props: &[],
        build_demo: code_demo,
    },
    ComponentEntry {
        name: "Kbd", slug: "kbd",
        description: "Keyboard shortcut indicators.",
        category: Category::DataDisplay, order: 304,
        variants: &[],
        bool_props: &[],
        build_demo: kbd_demo,
    },
    ComponentEntry {
        name: "Table", slug: "table",
        description: "Data tables with headers and rows.",
        category: Category::DataDisplay, order: 305,
        variants: &[],
        bool_props: &[BoolProp { name: "striped", description: "Alternate row colors", default: false }],
        build_demo: table_demo,
    },
    ComponentEntry {
        name: "Stat", slug: "stat",
        description: "Key metric display with trend indicator.",
        category: Category::DataDisplay, order: 306,
        variants: STAT_VARIANTS,
        bool_props: &[],
        build_demo: stat_demo,
    },
    ComponentEntry {
        name: "Avatar", slug: "avatar",
        description: "User avatars with image or fallback text.",
        category: Category::DataDisplay, order: 307,
        variants: SM_MD_LG_SIZE,
        bool_props: &[],
        build_demo: avatar_demo,
    },
    ComponentEntry {
        name: "AvatarGroup", slug: "avatar-group",
        description: "Group of avatars with overflow count.",
        category: Category::DataDisplay, order: 308,
        variants: &[],
        bool_props: &[],
        build_demo: avatar_group_demo,
    },
    ComponentEntry {
        name: "Image", slug: "image",
        description: "Responsive images with fit and aspect controls.",
        category: Category::DataDisplay, order: 309,
        variants: IMAGE_VARIANTS,
        bool_props: &[
            BoolProp { name: "rounded", description: "Round corners", default: false },
            BoolProp { name: "full_width", description: "Stretch to full width", default: false },
        ],
        build_demo: image_demo,
    },
    ComponentEntry {
        name: "List", slug: "list",
        description: "Ordered and unordered lists.",
        category: Category::DataDisplay, order: 310,
        variants: &[],
        bool_props: &[BoolProp { name: "ordered", description: "Show as ordered list", default: false }],
        build_demo: list_demo,
    },
    ComponentEntry {
        name: "Blockquote", slug: "blockquote",
        description: "Quoted text with optional citation.",
        category: Category::DataDisplay, order: 311,
        variants: &[],
        bool_props: &[],
        build_demo: blockquote_demo,
    },
    ComponentEntry {
        name: "Timeline", slug: "timeline",
        description: "Vertical timeline of events.",
        category: Category::DataDisplay, order: 312,
        variants: &[],
        bool_props: &[],
        build_demo: timeline_demo,
    },

    // --- Feedback (order 4xx) ---
    ComponentEntry {
        name: "Alert", slug: "alert",
        description: "Alert messages with different intent levels.",
        category: Category::Feedback, order: 400,
        variants: ALERT_VARIANTS,
        bool_props: &[],
        build_demo: alert_demo,
    },
    ComponentEntry {
        name: "Toast", slug: "toast",
        description: "Temporary notification messages.",
        category: Category::Feedback, order: 401,
        variants: TOAST_VARIANTS,
        bool_props: &[],
        build_demo: toast_demo,
    },
    ComponentEntry {
        name: "Spinner", slug: "spinner",
        description: "Loading indicators with size variants.",
        category: Category::Feedback, order: 402,
        variants: SM_MD_LG_SIZE,
        bool_props: &[],
        build_demo: spinner_demo,
    },
    ComponentEntry {
        name: "Progress", slug: "progress",
        description: "Progress bars showing task completion.",
        category: Category::Feedback, order: 403,
        variants: &[],
        bool_props: &[],
        build_demo: progress_demo,
    },
    ComponentEntry {
        name: "Skeleton", slug: "skeleton",
        description: "Loading placeholder that shows content shape.",
        category: Category::Feedback, order: 404,
        variants: SKELETON_VARIANTS,
        bool_props: &[],
        build_demo: skeleton_demo,
    },
    ComponentEntry {
        name: "EmptyState", slug: "empty-state",
        description: "Placeholder for empty content areas.",
        category: Category::Feedback, order: 405,
        variants: &[],
        bool_props: &[],
        build_demo: empty_state_demo,
    },
    ComponentEntry {
        name: "Tooltip", slug: "tooltip",
        description: "Contextual information on hover.",
        category: Category::Feedback, order: 406,
        variants: TOOLTIP_VARIANTS,
        bool_props: &[],
        build_demo: tooltip_demo,
    },

    // --- Navigation (order 5xx) ---
    ComponentEntry {
        name: "Link", slug: "link",
        description: "Navigation links with internal and external variants.",
        category: Category::Navigation, order: 500,
        variants: &[],
        bool_props: &[BoolProp { name: "external", description: "Open in new tab", default: false }],
        build_demo: link_demo,
    },
    ComponentEntry {
        name: "NavMenu", slug: "nav-menu",
        description: "Navigation menu with active state.",
        category: Category::Navigation, order: 501,
        variants: &[],
        bool_props: &[],
        build_demo: nav_menu_demo,
    },
    ComponentEntry {
        name: "Breadcrumb", slug: "breadcrumb",
        description: "Navigation breadcrumb trails.",
        category: Category::Navigation, order: 502,
        variants: &[],
        bool_props: &[],
        build_demo: breadcrumb_demo,
    },
    ComponentEntry {
        name: "Tabs", slug: "tabs",
        description: "Tab navigation for content sections.",
        category: Category::Navigation, order: 503,
        variants: &[],
        bool_props: &[],
        build_demo: tabs_demo,
    },
    ComponentEntry {
        name: "Pagination", slug: "pagination",
        description: "Page navigation for lists and tables.",
        category: Category::Navigation, order: 504,
        variants: &[],
        bool_props: &[],
        build_demo: pagination_demo,
    },
    ComponentEntry {
        name: "Footer", slug: "footer",
        description: "Page footer with logo, links, and copyright.",
        category: Category::Navigation, order: 505,
        variants: &[],
        bool_props: &[],
        build_demo: footer_demo,
    },

    // --- Overlay (order 6xx) ---
    ComponentEntry {
        name: "Modal", slug: "modal",
        description: "Dialog overlay for confirmations and forms.",
        category: Category::Overlay, order: 600,
        variants: MODAL_VARIANTS,
        bool_props: BOOL_OPEN,
        build_demo: modal_demo,
    },
    ComponentEntry {
        name: "Drawer", slug: "drawer",
        description: "Slide-out panel from left or right edge.",
        category: Category::Overlay, order: 601,
        variants: DRAWER_VARIANTS,
        bool_props: BOOL_OPEN,
        build_demo: drawer_demo,
    },
    ComponentEntry {
        name: "Dropdown", slug: "dropdown",
        description: "Dropdown menu with actions.",
        category: Category::Overlay, order: 602,
        variants: &[],
        bool_props: BOOL_OPEN,
        build_demo: dropdown_demo,
    },
    ComponentEntry {
        name: "Accordion", slug: "accordion",
        description: "Collapsible content sections.",
        category: Category::Overlay, order: 603,
        variants: &[],
        bool_props: &[],
        build_demo: accordion_demo,
    },
    ComponentEntry {
        name: "CopyButton", slug: "copy-button",
        description: "One-click text copy to clipboard.",
        category: Category::Overlay, order: 604,
        variants: &[],
        bool_props: &[],
        build_demo: copy_button_demo,
    },
    ComponentEntry {
        name: "ThemeToggle", slug: "theme-toggle",
        description: "Toggle between light and dark modes.",
        category: Category::Overlay, order: 605,
        variants: THEME_TOGGLE_VARIANTS,
        bool_props: &[],
        build_demo: theme_toggle_demo,
    },
    ComponentEntry {
        name: "Stepper", slug: "stepper",
        description: "Multi-step progress indicator.",
        category: Category::Overlay, order: 606,
        variants: &[],
        bool_props: &[],
        build_demo: stepper_demo,
    },
];
