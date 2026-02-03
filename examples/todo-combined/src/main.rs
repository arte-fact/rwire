//! Todo Combined Example - Full TodoMVC with EventContext and ItemRef support.
//!
//! This example demonstrates all three storage types with full TodoMVC functionality:
//! 1. **Local Todo** - Instant toggles without server round-trip (UI state)
//! 2. **Memory Todo** - Server-side state with text input (session state)
//! 3. **Persisted Todo** - File-backed, survives refresh (data state)
//!
//! Key features demonstrated:
//! - Text input capture via EventContext
//! - Item-specific actions using ItemRef and iter_with_ref() (new API)
//! - Type-safe event binding with on_ref()
//!
//! Run with: `cargo run -p todo-combined`
//! Open: http://127.0.0.1:9000

use rwire::{
    el, handler, persist_task, renderer, El, ElementBuilder, Ev, IterWithRef, PersistError,
    PersistableType, Server, SqliteStore, State,
    // Styling system
    Button, ButtonSize, Input, Stack, Gap, Card, CardPadding,
    Theme, CapsuleConfig,
};
use serde::{Deserialize, Serialize};
use std::any::{Any, TypeId};
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// Local State - Client-side, instant response
// ============================================================================

/// Local state for UI interactions - toggle without server round-trip.
///
/// NOTE: Local state handlers require #[handler(local)] attribute and
/// can only use simple mutations (toggle, add, set).
#[derive(State, Default)]
#[storage(local)]
struct LocalUiState {
    item1_done: bool,
    item2_done: bool,
    item3_done: bool,
    show_completed: bool,
}

// Local handlers use #[handler(local)] and compile to client-side mutations.
// These handlers execute entirely in the browser without network round-trips.

#[handler(local)]
fn toggle_local_1(state: &mut LocalUiState) {
    state.item1_done = !state.item1_done;
}

#[handler(local)]
fn toggle_local_2(state: &mut LocalUiState) {
    state.item2_done = !state.item2_done;
}

#[handler(local)]
fn toggle_local_3(state: &mut LocalUiState) {
    state.item3_done = !state.item3_done;
}

#[handler(local)]
fn toggle_show_completed(state: &mut LocalUiState) {
    state.show_completed = !state.show_completed;
}

// ============================================================================
// Memory State - Server-side, per-session with text input support
// ============================================================================

/// Memory state for session-scoped todo items.
///
/// This state lives on the server and is lost when the connection closes.
/// Demonstrates text input capture and item-specific actions using ItemRef.
#[derive(State, Default)]
#[storage(memory)]
struct MemoryTodoState {
    items: Vec<MemoryTodoItem>,
    input_value: String,
}

/// A todo item - no ID needed since ItemRef tracks items by index.
#[derive(Clone, Default)]
struct MemoryTodoItem {
    text: String,
    done: bool,
}

/// Handler with EventContext - captures text input value
#[handler]
fn update_memory_input(state: &mut MemoryTodoState, ctx: &EventContext) {
    if let Some(text) = ctx.text() {
        state.input_value = text.to_string();
    }
}

/// Handler with EventContext - adds todo from input
#[handler]
fn add_memory_item(state: &mut MemoryTodoState, ctx: &EventContext) {
    // Try to get text from form field or direct input
    let text = ctx
        .field("todo")
        .or_else(|| ctx.text())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| {
            // Fallback to stored input value
            if !state.input_value.is_empty() {
                Some(std::mem::take(&mut state.input_value))
            } else {
                None
            }
        });

    if let Some(text) = text {
        // No ID needed - ItemRef tracks items by their index in the Vec
        state.items.push(MemoryTodoItem { text, done: false });
        state.input_value.clear();
    }
}

/// Handler with ItemRef - toggles specific item by index
/// This is the new cleaner API using on_ref() and ItemRef
#[handler]
fn toggle_memory_item(state: &mut MemoryTodoState, ctx: &EventContext) {
    if let Some(item_ref) = ctx.item_ref::<MemoryTodoItem>() {
        if let Some(item) = item_ref.get_mut(&mut state.items) {
            item.done = !item.done;
        }
    }
}

/// Handler with ItemRef - deletes specific item by index
#[handler]
fn delete_memory_item(state: &mut MemoryTodoState, ctx: &EventContext) {
    if let Some(idx) = ctx.item_index() {
        if idx < state.items.len() {
            state.items.remove(idx);
        }
    }
}

#[handler]
fn clear_memory_completed(state: &mut MemoryTodoState) {
    state.items.retain(|item| !item.done);
}

// ============================================================================
// Persisted State - SQLite-backed, survives restart
// ============================================================================

/// Persisted state for durable todo items.
///
/// This state demonstrates the persistence architecture:
/// - Memory-first: State lives in SharedServerState.shared_cache for instant access
/// - Background persistence: Dirty keys are persisted asynchronously via persist_task
/// - Normalized schema: Vec<T> fields become child tables (todos_items)
///
/// The state is keyed by session_id, so each browser session has its own todo list.
/// Data survives server restarts via SQLite persistence.
#[derive(State, Default, Clone, Serialize, Deserialize)]
#[storage(persisted, table = "todos", key = "session_id")]
struct PersistedTodoState {
    #[serde(default)]
    session_id: String,
    items: Vec<PersistedTodoItem>,
}

/// A persisted todo item - no ID needed since ItemRef tracks items by index.
#[derive(Clone, Default, Serialize, Deserialize)]
struct PersistedTodoItem {
    text: String,
    done: bool,
}

/// Handler with EventContext - adds todo from form submission
#[handler]
fn add_persisted_item(state: &mut PersistedTodoState, ctx: &EventContext) {
    let text = ctx
        .field("todo")
        .or_else(|| ctx.text())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    if let Some(text) = text {
        // No ID needed - ItemRef tracks items by their index in the Vec
        state.items.push(PersistedTodoItem {
            text: text.to_string(),
            done: false,
        });
    }
}

/// Handler with ItemRef - toggles specific item by index
#[handler]
fn toggle_persisted_item(state: &mut PersistedTodoState, ctx: &EventContext) {
    if let Some(item_ref) = ctx.item_ref::<PersistedTodoItem>() {
        if let Some(item) = item_ref.get_mut(&mut state.items) {
            item.done = !item.done;
        }
    }
}

/// Handler with ItemRef - deletes specific item by index
#[handler]
fn delete_persisted_item(state: &mut PersistedTodoState, ctx: &EventContext) {
    if let Some(idx) = ctx.item_index() {
        if idx < state.items.len() {
            state.items.remove(idx);
        }
    }
}

#[handler]
fn clear_persisted_completed(state: &mut PersistedTodoState) {
    state.items.retain(|item| !item.done);
}

// ============================================================================
// UI Components
// ============================================================================

fn build_app() -> ElementBuilder {
    Stack::column()
        .gap(Gap::Lg)
        .class("app")
        .children([
            el(El::H1).text("rwire Multi-State TodoMVC"),
            el(El::P)
                .class("subtitle")
                .text("Three todo lists demonstrating different storage strategies with EventContext"),
            Stack::row()
                .gap(Gap::Lg)
                .class("columns")
                .children([
                    build_local_column(),
                    build_memory_column(),
                    build_persisted_column(),
                ])
                .build(),
        ])
        .build()
}

fn build_local_column() -> ElementBuilder {
    Card::new()
        .class("column local")
        .child(
            Stack::column()
                .gap(Gap::Md)
                .children([
                    el(El::H2).text("Local State"),
                    el(El::P)
                        .class("description")
                        .text("Instant - no server round-trip"),
                    Stack::row()
                        .gap(Gap::Sm)
                        .wrap(true)
                        .class("controls")
                        .children([
                            Button::secondary("Toggle #1").on_click(toggle_local_1()),
                            Button::secondary("Toggle #2").on_click(toggle_local_2()),
                            Button::secondary("Toggle #3").on_click(toggle_local_3()),
                            Button::secondary("Toggle Show Done").on_click(toggle_show_completed()),
                        ])
                        .build(),
                    render_local_items(),
                ])
                .build(),
        )
        .build()
}

fn build_memory_column() -> ElementBuilder {
    Card::new()
        .class("column memory")
        .child(
            Stack::column()
                .gap(Gap::Md)
                .children([
                    el(El::H2).text("Memory State"),
                    el(El::P)
                        .class("description")
                        .text("Server-side with text input, lost on disconnect"),
                    // Input form for adding items
                    Stack::row()
                        .gap(Gap::Sm)
                        .class("input-row")
                        .children([
                            Input::text()
                                .placeholder("What needs to be done?")
                                .name("todo")
                                .on_input(update_memory_input()),
                            Button::primary("Add").on_click(add_memory_item()),
                        ])
                        .build(),
                    Stack::row()
                        .gap(Gap::Sm)
                        .class("controls")
                        .children([Button::secondary("Clear Done")
                            .on_click(clear_memory_completed())])
                        .build(),
                    render_memory_items(),
                ])
                .build(),
        )
        .build()
}

fn build_persisted_column() -> ElementBuilder {
    Card::new()
        .class("column persisted")
        .child(
            Stack::column()
                .gap(Gap::Md)
                .children([
                    el(El::H2).text("Persisted State"),
                    el(El::P)
                        .class("description")
                        .text("Memory-first with SQLite background sync"),
                    // Input form for adding items - use Form element for submit handling
                    el(El::Form)
                        .class("input-row rw-stack rw-stack-row rw-gap-sm")
                        .on(Ev::Submit, add_persisted_item())
                        .append([
                            Input::text()
                                .placeholder("What needs to be done?")
                                .name("todo")
                                .build(),
                            el(El::Button)
                                .attr("type", "submit")
                                .text("Add")
                                .class("rw-btn"),
                        ]),
                    Stack::row()
                        .gap(Gap::Sm)
                        .class("controls")
                        .children([Button::secondary("Clear Done")
                            .on_click(clear_persisted_completed())])
                        .build(),
                    render_persisted_items(),
                ])
                .build(),
        )
        .build()
}

// ============================================================================
// Renderers
// ============================================================================

// NOTE: The framework does NOT properly support nested renderers during updates!
// See docs/architecture-state.md for details.
//
// When a parent renderer re-renders, it clears its wrapper's children.
// If the parent contains a nested renderer, that nested wrapper is destroyed.
// The nested renderer's update then tries to find a non-existent element.
//
// WORKAROUND: Keep renderers flat (no nested renderers).
// Split logic into separate renderers that are siblings, not nested.

#[renderer]
fn render_local_items(state: &LocalUiState) -> ElementBuilder {
    let mut items = Vec::new();

    // Fixed items with toggleable state
    let item1 = ("Buy groceries", state.item1_done);
    let item2 = ("Clean house", state.item2_done);
    let item3 = ("Read book", state.item3_done);

    for (text, done) in [item1, item2, item3] {
        if !done || state.show_completed {
            let mark = if done { "[x]" } else { "[ ]" };
            items.push(format!("{} {}", mark, text));
        }
    }

    let status = if state.show_completed {
        "showing all"
    } else {
        "hiding completed"
    };

    let display = if items.is_empty() {
        format!("(all done) - {}", status)
    } else {
        format!("{} | {}", items.join(" | "), status)
    };

    el(El::Span).class("item-list").text(&display)
}

// Memory todo - split into 3 renderers (items list, count, empty state)
// These are SIBLINGS, not nested, to avoid the nested renderer bug.

#[renderer]
fn render_memory_items(_state: &MemoryTodoState) -> ElementBuilder {
    // Container with items list and footer as siblings
    el(El::Div)
        .class("todo-list")
        .append([render_memory_items_list(), render_memory_count()])
}

#[renderer]
fn render_memory_items_list(state: &MemoryTodoState) -> ElementBuilder {
    if state.items.is_empty() {
        return Card::new()
            .padding(CardPadding::Md)
            .class("empty")
            .child(el(El::P).text("No items - type above and click Add"))
            .build();
    }

    // Use iter_with_ref() for type-safe item references
    let items: Vec<ElementBuilder> = state
        .items
        .iter_with_ref()
        .map(|(item_ref, item)| {
            let class = if item.done { "item done" } else { "item" };
            Stack::row()
                .gap(Gap::Sm)
                .align(rwire::StackAlign::Center)
                .class(class)
                .children([
                    el(El::Span)
                        .class("checkbox")
                        .text(if item.done { "[x]" } else { "[ ]" })
                        .on_ref(Ev::Click, toggle_memory_item(), item_ref),
                    el(El::Span).class("text").text(&item.text),
                    Button::ghost("×")
                        .size(ButtonSize::Sm)
                        .class("delete")
                        .build()
                        .on_ref(Ev::Click, delete_memory_item(), item_ref),
                ])
                .build()
        })
        .collect();

    Stack::column()
        .gap(Gap::Xs)
        .class("items")
        .children(items)
        .build()
}

#[renderer]
fn render_memory_count(state: &MemoryTodoState) -> ElementBuilder {
    let count = state.items.iter().filter(|i| !i.done).count();
    let count_text = format!("{} item{} left", count, if count == 1 { "" } else { "s" });
    el(El::Div).class("footer").text(&count_text)
}

// Persisted todo - split into 3 renderers (same pattern)

#[renderer]
fn render_persisted_items(_state: &PersistedTodoState) -> ElementBuilder {
    el(El::Div)
        .class("todo-list")
        .append([render_persisted_items_list(), render_persisted_count()])
}

#[renderer]
fn render_persisted_items_list(state: &PersistedTodoState) -> ElementBuilder {
    if state.items.is_empty() {
        return Card::new()
            .padding(CardPadding::Md)
            .class("empty")
            .child(el(El::P).text("No items - type above and click Add"))
            .build();
    }

    // Use iter_with_ref() for type-safe item references
    let items: Vec<ElementBuilder> = state
        .items
        .iter_with_ref()
        .map(|(item_ref, item)| {
            let class = if item.done { "item done" } else { "item" };
            Stack::row()
                .gap(Gap::Sm)
                .align(rwire::StackAlign::Center)
                .class(class)
                .children([
                    el(El::Span)
                        .class("checkbox")
                        .text(if item.done { "[x]" } else { "[ ]" })
                        .on_ref(Ev::Click, toggle_persisted_item(), item_ref),
                    el(El::Span).class("text").text(&item.text),
                    Button::ghost("×")
                        .size(ButtonSize::Sm)
                        .class("delete")
                        .build()
                        .on_ref(Ev::Click, delete_persisted_item(), item_ref),
                ])
                .build()
        })
        .collect();

    Stack::column()
        .gap(Gap::Xs)
        .class("items")
        .children(items)
        .build()
}

#[renderer]
fn render_persisted_count(state: &PersistedTodoState) -> ElementBuilder {
    let count = state.items.iter().filter(|i| !i.done).count();
    let count_text = format!("{} item{} left", count, if count == 1 { "" } else { "s" });
    el(El::Div).class("footer").text(&count_text)
}

// ============================================================================
// Main
// ============================================================================

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("rwire Server - Todo Combined Demo");
    println!("==================================");
    println!();
    println!("Demonstrating three storage types with EventContext and ItemRef:");
    println!("  - Local:     Client-side, instant UI response (fixed items)");
    println!("  - Memory:    Server-side with text input (session-scoped)");
    println!("  - Persisted: SQLite-backed, survives server restart");
    println!();
    println!("Persistence architecture (for persisted state):");
    println!("  - Instant handler execution (memory-only mutation)");
    println!("  - Background persistence task (non-blocking DB writes)");
    println!("  - Cross-tab synchronization (broadcast to other connections)");
    println!("  - Graceful shutdown (flush dirty state before exit)");
    println!();
    println!("Open http://127.0.0.1:9000 in your browser");
    println!();

    // Set up SQLite persistence
    let store = SqliteStore::new("./todo.db")?;

    // Register the PersistedTodoState type with load/save functions
    store.register(PersistableType {
        table_name: "todos",
        schema: PersistedTodoState::SCHEMA,
        type_id: TypeId::of::<PersistedTodoState>(),
        key_field: "session_id",
        load_fn: load_todo_state,
        save_fn: save_todo_state,
        default_fn: || Box::new(PersistedTodoState::default()),
    });

    // Ensure database schema exists
    store.ensure_schema()?;

    // Create child table for todo items (not auto-generated by macro)
    {
        let conn = store.connection();
        let conn = conn.lock().unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS todos_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                parent_session_id TEXT NOT NULL,
                text TEXT NOT NULL,
                done INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (parent_session_id) REFERENCES todos(session_id) ON DELETE CASCADE
            )",
            [],
        )?;
    }
    println!("SQLite database initialized: ./todo.db");

    // Configure theme for styled capsule
    let theme = Theme::light();
    let config = CapsuleConfig::new().theme(theme);

    // Build server with persistence support and styling
    let mut server = Server::bind("127.0.0.1:9000")?
        .persist_interval(Duration::from_millis(100))
        .root(build_app)
        .capsule_config(config);

    // Get shared state for persist task and shutdown
    let shared = server.shared_state();

    // Hydrate state from database
    let hydrated = shared.hydrate(&store)?;
    if hydrated > 0 {
        println!("Hydrated {} persisted state entries from database", hydrated);
    }

    // Spawn background persist task
    let persist_shared = Arc::clone(&shared);
    let persist_store = store.clone();
    async_std::task::spawn(async move {
        persist_task(persist_shared, persist_store).await;
    });

    // Run server (blocks until shutdown)
    // Note: For graceful shutdown with Ctrl+C, you would need signal handling
    // which requires additional setup. For now, state is persisted periodically.
    server.run().await
}

// Load function for PersistedTodoState
fn load_todo_state(
    conn: &rwire::rusqlite::Connection,
    session_id: &str,
) -> Result<Option<Box<dyn Any + Send + Sync>>, PersistError> {
    // Load main record
    let result: Result<String, _> = conn.query_row(
        "SELECT session_id FROM todos WHERE session_id = ?",
        [session_id],
        |row| row.get(0),
    );

    match result {
        Ok(_) => {
            // Load items from child table
            let mut stmt = conn.prepare(
                "SELECT text, done FROM todos_items WHERE parent_session_id = ? ORDER BY rowid",
            )?;
            let items: Vec<PersistedTodoItem> = stmt
                .query_map([session_id], |row| {
                    Ok(PersistedTodoItem {
                        text: row.get(0)?,
                        done: row.get::<_, i32>(1)? != 0,
                    })
                })?
                .filter_map(|r| r.ok())
                .collect();

            Ok(Some(Box::new(PersistedTodoState {
                session_id: session_id.to_string(),
                items,
            })))
        }
        Err(rwire::rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(PersistError::Sqlite(e)),
    }
}

// Save function for PersistedTodoState
fn save_todo_state(
    conn: &rwire::rusqlite::Connection,
    session_id: &str,
    state: &dyn Any,
) -> Result<(), PersistError> {
    let state = state
        .downcast_ref::<PersistedTodoState>()
        .ok_or(PersistError::TypeMismatch)?;

    // Upsert main record
    conn.execute(
        "INSERT INTO todos (session_id) VALUES (?1)
         ON CONFLICT(session_id) DO NOTHING",
        [session_id],
    )?;

    // Delete existing items and re-insert (simpler than diffing)
    conn.execute(
        "DELETE FROM todos_items WHERE parent_session_id = ?",
        [session_id],
    )?;

    // Insert current items
    let mut stmt = conn.prepare(
        "INSERT INTO todos_items (parent_session_id, text, done) VALUES (?1, ?2, ?3)",
    )?;
    for item in &state.items {
        stmt.execute(rwire::rusqlite::params![session_id, &item.text, item.done as i32])?;
    }

    Ok(())
}
