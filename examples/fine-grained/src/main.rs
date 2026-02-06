//! Fine-grained reactivity demo with TypeId filtering.
//!
//! Each state type is a separate struct. Handlers only trigger re-renders
//! of synced elements bound to the same state type. Watch the server console
//! to verify that clicking "+" only calls render_counter(), not all renderers.

use rwire::capsule_gen::CapsuleConfig;
use rwire::components::{
    Badge, BadgeIntent, Button, Card, Container, Divider, Gap, Stack, Text,
};
use rwire::{handler, renderer, ElementBuilder, Server, State};

// --- Four separate state types for TypeId filtering ---

#[derive(State, Default)]
#[storage(memory)]
struct CounterState {
    counter: i32,
}

#[derive(State, Default)]
#[storage(memory)]
struct NameState {
    name: String,
}

#[derive(State, Default)]
#[storage(memory)]
struct EnabledState {
    enabled: bool,
}

#[derive(State, Default)]
#[storage(memory)]
struct ItemsState {
    items: Vec<String>,
}

// --- Handlers (each targets one state type) ---

#[handler] fn increment(s: &mut CounterState) { s.counter += 1; }
#[handler] fn decrement(s: &mut CounterState) { s.counter -= 1; }
#[handler] fn set_alice(s: &mut NameState) { s.name = "Alice".into(); }
#[handler] fn set_bob(s: &mut NameState) { s.name = "Bob".into(); }
#[handler] fn toggle(s: &mut EnabledState) { s.enabled = !s.enabled; }
#[handler] fn add_item(s: &mut ItemsState) { s.items.push(format!("Item {}", s.items.len() + 1)); }
#[handler] fn remove_item(s: &mut ItemsState) { s.items.pop(); }

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Fine-Grained Reactivity with TypeId Filtering");
    println!("Watch this console: each click should only trigger ONE renderer!");
    Server::bind("127.0.0.1:9000")?
        .root(app)
        .capsule_config(CapsuleConfig::dark_nord())
        .run()
        .await
}

fn app() -> ElementBuilder {
    Container::new().child(
        Stack::column().gap(Gap::Lg).children([
            Text::heading1("Fine-Grained Reactivity").build(),
            Text::body("Each handler only re-renders its own state type.").muted().build(),
            Divider::horizontal().build(),
            section("Counter", "CounterState",
                Stack::row().gap(Gap::Sm).children([
                    Button::secondary("-").on_click(decrement()),
                    Button::secondary("+").on_click(increment()),
                ]).build(),
                render_counter()),
            section("Name", "NameState",
                Stack::row().gap(Gap::Sm).children([
                    Button::secondary("Alice").on_click(set_alice()),
                    Button::secondary("Bob").on_click(set_bob()),
                ]).build(),
                render_name()),
            section("Enabled", "EnabledState",
                Button::secondary("Toggle").on_click(toggle()),
                render_enabled()),
            section("Items", "ItemsState",
                Stack::row().gap(Gap::Sm).children([
                    Button::secondary("Add").on_click(add_item()),
                    Button::secondary("Remove").on_click(remove_item()),
                ]).build(),
                render_items()),
        ]).build()
    ).build()
}

fn section(title: &'static str, state_type: &'static str, controls: ElementBuilder, renderer: ElementBuilder) -> ElementBuilder {
    Card::new().child(
        Stack::column().gap(Gap::Md).children([
            Text::heading3(title).build(),
            Text::caption(format!("state: {}", state_type)).build(),
            controls,
            renderer,
        ]).build()
    ).build()
}

#[renderer]
fn render_counter(s: &CounterState) -> ElementBuilder {
    println!("  render_counter()");
    Stack::row().gap(Gap::Sm).align_center().children([
        Text::label("Counter:").build(),
        Badge::primary(s.counter.to_string()).build(),
    ]).build()
}

#[renderer]
fn render_name(s: &NameState) -> ElementBuilder {
    println!("  render_name()");
    let display = if s.name.is_empty() { "(not set)".into() } else { s.name.clone() };
    Stack::row().gap(Gap::Sm).align_center().children([
        Text::label("Name:").build(),
        Badge::default_badge(display).build(),
    ]).build()
}

#[renderer]
fn render_enabled(s: &EnabledState) -> ElementBuilder {
    println!("  render_enabled()");
    let (status, intent) = if s.enabled { ("ON", BadgeIntent::Success) } else { ("OFF", BadgeIntent::Error) };
    Stack::row().gap(Gap::Sm).align_center().children([
        Text::label("Enabled:").build(),
        Badge::new().intent(intent).text(status).build(),
    ]).build()
}

#[renderer]
fn render_items(s: &ItemsState) -> ElementBuilder {
    println!("  render_items()");
    let text = if s.items.is_empty() { "(empty)".into() } else { s.items.join(", ") };
    Stack::row().gap(Gap::Sm).align_center().children([
        Text::label("Items:").build(),
        Badge::default_badge(text).build(),
    ]).build()
}
