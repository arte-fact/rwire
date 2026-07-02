//! Todolist example - demonstrates reactive state with lists and filtering.

use rwire::capsule_gen::CapsuleConfig;
use rwire::theme::Theme;
use rwire::{handler, renderer, theme, ElementBuilder, Server, State};
use rwire_components::{Badge, BadgeIntent, Button, Card, Container, Divider, Gap, Stack, Text};
use rwire_themes::palettes;

#[derive(State, Default)]
#[storage(memory)]
struct TodoState {
    show_completed: bool,
    filter_mode: i32, // 0=all, 1=active, 2=completed
    items: Vec<TodoItem>,
    next_id: i32,
}

#[derive(Clone, Default)]
struct TodoItem {
    text: String,
    completed: bool,
}

// Handlers
#[handler]
fn toggle_show_completed(s: &mut TodoState) {
    s.show_completed = !s.show_completed;
}
#[handler]
fn set_filter_all(s: &mut TodoState) {
    s.filter_mode = 0;
}
#[handler]
fn set_filter_active(s: &mut TodoState) {
    s.filter_mode = 1;
}
#[handler]
fn set_filter_completed(s: &mut TodoState) {
    s.filter_mode = 2;
}

#[handler]
fn add_item(s: &mut TodoState) {
    s.items.push(TodoItem {
        text: format!("Todo item #{}", s.next_id),
        completed: false,
    });
    s.next_id += 1;
}

#[handler]
fn toggle_first(s: &mut TodoState) {
    if let Some(i) = s.items.first_mut() {
        i.completed = !i.completed;
    }
}
#[handler]
fn toggle_second(s: &mut TodoState) {
    if let Some(i) = s.items.get_mut(1) {
        i.completed = !i.completed;
    }
}
#[handler]
fn clear_completed(s: &mut TodoState) {
    s.items.retain(|i| !i.completed);
}
#[handler]
fn remove_first(s: &mut TodoState) {
    if !s.items.is_empty() {
        s.items.remove(0);
    }
}

#[theme]
fn app_theme() -> Theme {
    Theme::dark().palette(palettes::nord())
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Server::bind("127.0.0.1:9000")?
        .root(app)
        .capsule_config(CapsuleConfig::new())
        .theme(app_theme())
        .run()
        .await
}

fn app() -> ElementBuilder {
    Container::new()
        .child(
            Stack::column()
                .gap(Gap::Lg)
                .children([
                    Text::heading1("rwire Todo List").build(),
                    Text::body("Demonstrating reactive state updates")
                        .muted()
                        .build(),
                    controls(),
                    Divider::horizontal().build(),
                    render_stats(),
                    render_items(),
                ])
                .build(),
        )
        .build()
}

fn controls() -> ElementBuilder {
    Card::new()
        .child(
            Stack::column()
                .gap(Gap::Md)
                .children([
                    Text::label("Actions").build(),
                    Stack::row()
                        .gap(Gap::Sm)
                        .wrap(true)
                        .children([
                            Button::primary("Add Item").on_click(add_item()),
                            Button::secondary("Toggle First").on_click(toggle_first()),
                            Button::secondary("Toggle Second").on_click(toggle_second()),
                            Button::secondary("Remove First").on_click(remove_first()),
                            Button::destructive("Clear Completed").on_click(clear_completed()),
                        ])
                        .build(),
                    Divider::horizontal().build(),
                    Text::label("Filters").build(),
                    Stack::row()
                        .gap(Gap::Sm)
                        .wrap(true)
                        .children([
                            Button::ghost("All").on_click(set_filter_all()),
                            Button::ghost("Active").on_click(set_filter_active()),
                            Button::ghost("Completed").on_click(set_filter_completed()),
                            Button::ghost("Toggle Show Completed")
                                .on_click(toggle_show_completed()),
                        ])
                        .build(),
                ])
                .build(),
        )
        .build()
}

#[renderer]
fn render_stats(state: &TodoState) -> ElementBuilder {
    let total = state.items.len();
    let completed = state.items.iter().filter(|i| i.completed).count();
    let filter = ["all", "active", "completed"][state.filter_mode as usize];
    let show = if state.show_completed { "show" } else { "hide" };

    Stack::row()
        .gap(Gap::Sm)
        .align_center()
        .wrap(true)
        .children([
            Badge::default_badge(format!("{} total", total)).build(),
            Badge::primary(format!("{} active", total - completed)).build(),
            Badge::success(format!("{} done", completed)).build(),
            Badge::new()
                .intent(BadgeIntent::Warning)
                .text(format!("Filter: {}", filter))
                .build(),
            Badge::new().text(format!("Completed: {}", show)).build(),
        ])
        .build()
}

#[renderer]
fn render_items(state: &TodoState) -> ElementBuilder {
    let items: Vec<_> = state
        .items
        .iter()
        .filter(|i| match state.filter_mode {
            1 => !i.completed,
            2 => i.completed,
            _ => state.show_completed || !i.completed,
        })
        .take(5)
        .collect();

    if items.is_empty() {
        return Text::body("(no items)").muted().build();
    }

    Stack::column()
        .gap(Gap::Xs)
        .children(items.iter().map(|i| {
            let (intent, mark) = if i.completed {
                (BadgeIntent::Success, "[x]")
            } else {
                (BadgeIntent::Default, "[ ]")
            };
            Badge::new()
                .intent(intent)
                .text(format!("{} {}", mark, i.text))
                .build()
        }))
        .build()
}
