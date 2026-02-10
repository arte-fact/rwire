---
title: Local Mutations
description: Client-side state for instant UI feedback
order: 3
---
# Local Mutations

By default, every event in rwire round-trips to the server over WebSocket. For UI interactions that need instant feedback -- toggling a menu, switching tabs, expanding an accordion -- you can use local state that mutates directly in the browser.

```rust
#[derive(State, Default)]
#[storage(local)]
struct UIState {
    menu_open: bool,
    active_tab: u8,
}
```

## How It Works

The `#[storage(local)]` attribute tells the framework to:

1. Initialize the state as JSON in the browser at connection time
2. Compile handler mutations into binary opcodes (toggle, increment, set)
3. Execute those mutations in the JS runtime without a server round-trip

The capsule includes a mutation interpreter (~150 bytes of JS) that processes these opcodes locally.

## Supported Mutations

Local handlers support a focused set of mutation operations that the macro analyzes at compile time:

```rust
#[handler]
fn toggle_menu(state: &mut UIState) {
    state.menu_open = !state.menu_open;  // TOGGLE opcode
}

#[handler]
fn next_tab(state: &mut UIState) {
    state.active_tab += 1;  // ADD_I8 opcode
}
```

The `#[handler]` macro inspects the function body and emits the corresponding mutation opcodes. Field toggles, integer add/subtract, and direct assignments are supported.

## Common Use Cases

### Toggle Menus

```rust
#[derive(State, Default)]
#[storage(local)]
struct NavState {
    sidebar_open: bool,
}

#[handler]
fn toggle_sidebar(state: &mut NavState) {
    state.sidebar_open = !state.sidebar_open;
}

fn nav_button() -> ElementBuilder {
    el(El::Button)
        .text("Menu")
        .on(Ev::Click, toggle_sidebar())
}
```

### Tab Selection

```rust
#[derive(State, Default)]
#[storage(local)]
struct TabState {
    active: u8,
}

#[handler]
fn select_tab_0(state: &mut TabState) { state.active = 0; }
#[handler]
fn select_tab_1(state: &mut TabState) { state.active = 1; }
#[handler]
fn select_tab_2(state: &mut TabState) { state.active = 2; }
```

### Accordion / Disclosure

```rust
#[derive(State, Default)]
#[storage(local)]
struct AccordionState {
    expanded: bool,
}

#[handler]
fn toggle_expand(state: &mut AccordionState) {
    state.expanded = !state.expanded;
}
```

## Hybrid Patterns

Combine local and server state for the best of both worlds. Use local state for immediate UI feedback and server state for persistent data:

```rust
#[derive(State, Default)]
#[storage(local)]
struct FormUI {
    show_advanced: bool,
}

#[derive(State, Default)]
#[storage(memory)]
struct FormData {
    name: String,
    email: String,
}

fn settings_page() -> ElementBuilder {
    el(El::Div).append([
        el(El::Button)
            .text("Advanced Options")
            .on(Ev::Click, toggle_advanced()),  // instant, no round-trip
        render_form(),  // server-rendered, round-trips on submit
    ])
}
```

Local state handles the toggle instantly in the browser. Form submission goes to the server where validation and persistence happen.

## When Not to Use Local State

Local state is limited to simple mutations on primitive fields. Use server state (`#[storage(memory)]` or `#[storage(persisted)]`) when you need:

- Complex logic (conditionals, loops, external data)
- Validation or authorization checks
- Shared state across multiple clients
- Persistence across page reloads
