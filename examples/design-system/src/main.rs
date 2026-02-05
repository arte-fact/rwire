//! rwire Design System Documentation
//!
//! Interactive showcase of all rwire components.

use rwire::components::*;
use rwire::{el, handler, renderer, El, ElementBuilder, Server, State};
use rwire::capsule_gen::CapsuleConfig;
use rwire::theme::{Theme, ThemeMode};

#[derive(State, Default)]
#[storage(memory)]
struct DesignSystemState {
    active_section: usize, // 0=forms, 1=data, 2=feedback
    checkbox_checked: bool,
    switch_checked: bool,
    radio_selection: usize, // 0=free, 1=pro, 2=enterprise
    progress_value: u32,
    active_tab: usize,
    current_page: usize,
    theme_mode: ThemeMode,
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("rwire Design System Documentation");
    println!("Open http://127.0.0.1:9000 in your browser");
    println!();

    let capsule_config = CapsuleConfig::new()
        .theme(Theme::default());

    Server::bind("127.0.0.1:9000")?
        .root(root)
        .capsule_config(capsule_config)
        .run()
        .await
}

#[renderer]
fn root(state: &DesignSystemState) -> ElementBuilder {
    el(El::Div)
        .attr("data-theme", state.theme_mode.as_str())
        .append([
            Stack::column()
                .gap(Gap::Md)
                .children([
                    render_header(),
                    render_nav(),
                    render_content(),
                ])
                .build()
        ])
}

#[renderer]
fn render_header(state: &DesignSystemState) -> ElementBuilder {
    Card::new()
        .child(
            Stack::row()
                .gap(Gap::Md)
                .justify(StackJustify::Between)
                .align(StackAlign::Center)
                .children([
                    Stack::column()
                        .gap(Gap::Sm)
                        .children([
                            el(El::H1).text("rwire Design System"),
                            el(El::P).text("Interactive documentation for all rwire components"),
                        ])
                        .build(),
                    ThemeToggle::new()
                        .mode(match state.theme_mode {
                            ThemeMode::Light => ThemeToggleMode::Light,
                            ThemeMode::Dark => ThemeToggleMode::Dark,
                        })
                        .on_toggle(toggle_theme())
                        .build(),
                ])
                .build(),
        )
        .build()
}

#[renderer]
fn render_nav(state: &DesignSystemState) -> ElementBuilder {
    Card::new()
        .child(
            Stack::row()
                .gap(Gap::Sm)
                .children([
                    Button::new()
                        .intent(if state.active_section == 0 {
                            ButtonIntent::Primary
                        } else {
                            ButtonIntent::Secondary
                        })
                        .text("Form Components")
                        .on_click(set_section_forms()),
                    Button::new()
                        .intent(if state.active_section == 1 {
                            ButtonIntent::Primary
                        } else {
                            ButtonIntent::Secondary
                        })
                        .text("Data Display")
                        .on_click(set_section_data()),
                    Button::new()
                        .intent(if state.active_section == 2 {
                            ButtonIntent::Primary
                        } else {
                            ButtonIntent::Secondary
                        })
                        .text("Feedback & Navigation")
                        .on_click(set_section_feedback()),
                ])
                .build(),
        )
        .build()
}

#[handler]
fn toggle_theme(state: &mut DesignSystemState) {
    state.theme_mode = match state.theme_mode {
        ThemeMode::Light => ThemeMode::Dark,
        ThemeMode::Dark => ThemeMode::Light,
    };
}

#[renderer]
fn render_content(state: &DesignSystemState) -> ElementBuilder {
    match state.active_section {
        1 => render_data_display_section(),
        2 => render_feedback_section(),
        _ => render_forms_section(),
    }
}

// ============================================================================
// Form Components Section
// ============================================================================

#[renderer]
fn render_forms_section(state: &DesignSystemState) -> ElementBuilder {
    Stack::column()
        .gap(Gap::Lg)
        .children([
            el(El::H2).text("Form Components"),

            // Label
            render_component_card(
                "Label",
                "Form labels with optional required indicator",
                Stack::column()
                    .gap(Gap::Md)
                    .children([
                        Label::new("Username").build(),
                        Label::new("Email").required(true).build(),
                        Label::new("Password").build(),
                    ])
                    .build(),
            ),

            // Input
            render_component_card(
                "Input",
                "Text inputs with various types and sizes",
                Stack::column()
                    .gap(Gap::Md)
                    .children([
                        Input::text().placeholder("Enter your name").build(),
                        Input::email().placeholder("user@example.com").build(),
                        Input::password().placeholder("Password").build(),
                        Input::search()
                            .placeholder("Search...")
                            .size(InputSize::Sm)
                            .build(),
                    ])
                    .build(),
            ),

            // Textarea
            render_component_card(
                "Textarea",
                "Multi-line text input with customizable rows",
                Textarea::new()
                    .placeholder("Enter a description...")
                    .rows(4)
                    .build(),
            ),

            // Checkbox
            render_component_card(
                "Checkbox",
                "Boolean checkbox with optional label",
                Stack::column()
                    .gap(Gap::Md)
                    .children([
                        Checkbox::new()
                            .label("Subscribe to newsletter")
                            .checked(state.checkbox_checked)
                            .on_change(toggle_checkbox()),
                        el(El::P).text(&format!(
                            "Newsletter: {}",
                            if state.checkbox_checked { "Yes" } else { "No" }
                        )),
                    ])
                    .build(),
            ),

            // Radio
            render_component_card(
                "Radio",
                "Radio buttons for mutually exclusive options",
                render_radio_examples(state),
            ),

            // Switch
            render_component_card(
                "Switch",
                "Toggle switch for boolean states",
                Stack::column()
                    .gap(Gap::Md)
                    .children([
                        Switch::new()
                            .label("Enable notifications")
                            .checked(state.switch_checked)
                            .on_change(toggle_switch()),
                        el(El::P).text(&format!(
                            "Notifications: {}",
                            if state.switch_checked { "On" } else { "Off" }
                        )),
                    ])
                    .build(),
            ),

            // Select
            render_component_card(
                "Select",
                "Dropdown select with options",
                Select::new()
                    .option("us", "United States")
                    .option("uk", "United Kingdom")
                    .option("ca", "Canada")
                    .option("au", "Australia")
                    .build(),
            ),

            // FormField
            render_component_card(
                "FormField",
                "Composition component wrapping inputs with labels and validation",
                Stack::column()
                    .gap(Gap::Md)
                    .children([
                        FormField::new()
                            .label("Email")
                            .input(Input::email().build())
                            .required(true)
                            .help("We'll never share your email")
                            .build(),
                        FormField::new()
                            .label("Password")
                            .input(Input::password().build())
                            .required(true)
                            .error("Password must be at least 8 characters")
                            .build(),
                    ])
                    .build(),
            ),
        ])
        .build()
}

fn render_radio_examples(state: &DesignSystemState) -> ElementBuilder {
    Stack::column()
        .gap(Gap::Md)
        .children([
            Radio::new()
                .name("plan")
                .value("free")
                .label("Free Plan")
                .checked(state.radio_selection == 0)
                .on_change(select_radio_free()),
            Radio::new()
                .name("plan")
                .value("pro")
                .label("Pro Plan ($19/mo)")
                .checked(state.radio_selection == 1)
                .on_change(select_radio_pro()),
            Radio::new()
                .name("plan")
                .value("enterprise")
                .label("Enterprise Plan (Contact us)")
                .checked(state.radio_selection == 2)
                .on_change(select_radio_enterprise()),
            el(El::P).text(&format!(
                "Selected: {}",
                match state.radio_selection {
                    1 => "Pro",
                    2 => "Enterprise",
                    _ => "Free",
                }
            )),
        ])
        .build()
}

// ============================================================================
// Data Display Section
// ============================================================================

#[renderer]
fn render_data_display_section(state: &DesignSystemState) -> ElementBuilder {
    Stack::column()
        .gap(Gap::Lg)
        .children([
            el(El::H2).text("Data Display Components"),

            // Avatar
            render_component_card(
                "Avatar",
                "User avatars with image or fallback text",
                Stack::row()
                    .gap(Gap::Md)
                    .children([
                        Avatar::new().fallback("JD").size(AvatarSize::Sm).build(),
                        Avatar::new().fallback("AB").size(AvatarSize::Md).build(),
                        Avatar::new().fallback("XY").size(AvatarSize::Lg).build(),
                    ])
                    .build(),
            ),

            // Progress
            render_component_card(
                "Progress",
                "Progress bars showing task completion",
                Stack::column()
                    .gap(Gap::Md)
                    .children([
                        Progress::new().value(state.progress_value).max(100).build(),
                        Stack::row()
                            .gap(Gap::Sm)
                            .children([
                                Button::secondary("Decrease")
                                    .size(ButtonSize::Sm)
                                    .on_click(decrease_progress()),
                                Button::secondary("Increase")
                                    .size(ButtonSize::Sm)
                                    .on_click(increase_progress()),
                            ])
                            .build(),
                        el(El::P).text(&format!("{}% complete", state.progress_value)),
                    ])
                    .build(),
            ),

            // Spinner
            render_component_card(
                "Spinner",
                "Loading indicators with size variants",
                Stack::row()
                    .gap(Gap::Md)
                    .children([
                        Spinner::new().size(SpinnerSize::Sm).build(),
                        Spinner::new().size(SpinnerSize::Md).build(),
                        Spinner::new().size(SpinnerSize::Lg).build(),
                    ])
                    .build(),
            ),

            // Table
            render_component_card(
                "Table",
                "Data tables with headers and rows",
                Table::new()
                    .headers(["Name", "Role", "Status"])
                    .row(
                        TableRow::new()
                            .cell("Alice Johnson")
                            .cell("Engineer")
                            .cell("Active"),
                    )
                    .row(
                        TableRow::new()
                            .cell("Bob Smith")
                            .cell("Designer")
                            .cell("Active"),
                    )
                    .row(
                        TableRow::new()
                            .cell("Carol White")
                            .cell("Manager")
                            .cell("Away"),
                    )
                    .build(),
            ),

            // Badge
            render_component_card(
                "Badge",
                "Status indicators and labels",
                Stack::row()
                    .gap(Gap::Sm)
                    .children([
                        Badge::default_badge("Default").build(),
                        Badge::primary("Primary").build(),
                        Badge::success("Success").build(),
                        Badge::warning("Warning").build(),
                        Badge::error("Error").build(),
                    ])
                    .build(),
            ),
        ])
        .build()
}

// ============================================================================
// Feedback & Navigation Section
// ============================================================================

#[renderer]
fn render_feedback_section(state: &DesignSystemState) -> ElementBuilder {
    Stack::column()
        .gap(Gap::Lg)
        .children([
            el(El::H2).text("Feedback & Navigation Components"),

            // Alert
            render_component_card(
                "Alert",
                "Alert messages with different intent levels",
                Stack::column()
                    .gap(Gap::Md)
                    .children([
                        Alert::info()
                            .title("Information")
                            .message("This is an informational message.")
                            .build(),
                        Alert::success()
                            .title("Success!")
                            .message("Your changes have been saved successfully.")
                            .build(),
                        Alert::warning()
                            .title("Warning")
                            .message("Your session will expire in 5 minutes.")
                            .build(),
                        Alert::error()
                            .title("Error")
                            .message("Failed to save changes. Please try again.")
                            .build(),
                    ])
                    .build(),
            ),

            // Breadcrumb
            render_component_card(
                "Breadcrumb",
                "Navigation breadcrumb trails",
                Stack::column()
                    .gap(Gap::Md)
                    .children([
                        Breadcrumb::new()
                            .item("Home", Some("/"))
                            .item("Docs", Some("/docs"))
                            .item("Components", None::<&str>)
                            .build(),
                        Breadcrumb::new()
                            .item("Products", Some("/products"))
                            .item("Electronics", Some("/electronics"))
                            .item("Laptops", None::<&str>)
                            .build(),
                    ])
                    .build(),
            ),

            // Tabs
            render_component_card(
                "Tabs",
                "Tab navigation for content sections",
                Tabs::new()
                    .tab(Tab::new("Overview", el(El::P).text("Overview tab content")))
                    .tab(Tab::new("Settings", el(El::P).text("Settings tab content")))
                    .tab(Tab::new("Activity", el(El::P).text("Activity tab content")))
                    .active(state.active_tab)
                    .build(),
            ),

            // Pagination
            render_component_card(
                "Pagination",
                "Page navigation for lists and tables",
                Stack::column()
                    .gap(Gap::Md)
                    .children([
                        Pagination::new()
                            .current_page(state.current_page.max(1))
                            .total_pages(10)
                            .build(),
                        Stack::row()
                            .gap(Gap::Sm)
                            .children([
                                Button::secondary("Previous")
                                    .size(ButtonSize::Sm)
                                    .disabled(state.current_page <= 1)
                                    .on_click(previous_page()),
                                Button::secondary("Next")
                                    .size(ButtonSize::Sm)
                                    .disabled(state.current_page >= 10)
                                    .on_click(next_page()),
                            ])
                            .build(),
                        el(El::P).text(&format!("Page {} of 10", state.current_page.max(1))),
                    ])
                    .build(),
            ),
        ])
        .build()
}

// ============================================================================
// Utility Functions
// ============================================================================

fn render_component_card(title: &str, description: &str, example: ElementBuilder) -> ElementBuilder {
    Card::new()
        .child(
            Stack::column()
                .gap(Gap::Md)
                .children([
                    Stack::column()
                        .gap(Gap::Xs)
                        .children([
                            el(El::H1).text(title),
                            el(El::P).text(description),
                        ])
                        .build(),
                    example,
                ])
                .build(),
        )
        .build()
}

// ============================================================================
// Handlers
// ============================================================================

#[handler]
fn set_section_forms(state: &mut DesignSystemState) {
    state.active_section = 0;
}

#[handler]
fn set_section_data(state: &mut DesignSystemState) {
    state.active_section = 1;
}

#[handler]
fn set_section_feedback(state: &mut DesignSystemState) {
    state.active_section = 2;
}

#[handler]
fn toggle_checkbox(state: &mut DesignSystemState) {
    state.checkbox_checked = !state.checkbox_checked;
}

#[handler]
fn select_radio_free(state: &mut DesignSystemState) {
    state.radio_selection = 0;
}

#[handler]
fn select_radio_pro(state: &mut DesignSystemState) {
    state.radio_selection = 1;
}

#[handler]
fn select_radio_enterprise(state: &mut DesignSystemState) {
    state.radio_selection = 2;
}

#[handler]
fn toggle_switch(state: &mut DesignSystemState) {
    state.switch_checked = !state.switch_checked;
}

#[handler]
fn increase_progress(state: &mut DesignSystemState) {
    state.progress_value = (state.progress_value + 10).min(100);
}

#[handler]
fn decrease_progress(state: &mut DesignSystemState) {
    state.progress_value = state.progress_value.saturating_sub(10);
}

#[handler]
fn next_page(state: &mut DesignSystemState) {
    state.current_page = (state.current_page + 1).min(10);
}

#[handler]
fn previous_page(state: &mut DesignSystemState) {
    state.current_page = state.current_page.saturating_sub(1);
}
