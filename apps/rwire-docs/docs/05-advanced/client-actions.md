---
title: Client Actions
description: Zero-latency UI with Targets and Selectors
order: 6
---
# Client Actions

Client actions provide instant DOM reactivity entirely in the browser. No server round-trips, no state serialization -- just CSS class toggling driven by two primitives:

- **Targets** -- boolean toggles (show/hide a modal, expand a section)
- **Selectors** -- exclusive choice (active tab, current step)

Both use the existing `St` token system. When a target is toggled or a selector changes, bound elements automatically gain or lose CSS classes.

## Targets

A Target is a named boolean. Derive it on a unit struct:

```rust
use rwire::Target;

#[derive(Target)]
pub struct ModalOpen;
```

### Binding elements

Use `.when()` to add a style token when the target is true, and `.unless()` for the inverse:

```rust
// Modal overlay -- hidden by default, shown when ModalOpen is true
el(El::Div)
    .st([St::DisplayNone])
    .when::<ModalOpen>(St::DisplayFlex)

// Backdrop -- visible by default, hidden when ModalOpen is false
el(El::Div)
    .st([St::DisplayBlock])
    .unless::<ModalOpen>(St::DisplayNone)
```

### Triggering toggles

Use `.toggle()` to flip the target on an event:

```rust
el(El::Button)
    .text("Open Modal")
    .toggle::<ModalOpen>(Ev::Click)
```

Clicking this button flips `ModalOpen` and instantly updates every element bound to it -- the modal appears, the backdrop shows, all without touching the server.

### Default value

Targets default to `false`. To start as `true`, implement the trait manually:

```rust
pub struct SidebarOpen;

impl rwire::action::Target for SidebarOpen {
    fn default_value() -> bool { true }
}
```

### Timed toggles

Use `.toggle_timed()` for auto-reverting toggles. The target flips to `true` on the event, then automatically reverts to `false` after the specified duration:

```rust
#[derive(Target)]
struct CopyFeedback;

el(El::Button)
    .text("Copy")
    .toggle_timed::<CopyFeedback>(Ev::Click, 2000)  // reverts after 2s
```

This is useful for copy-to-clipboard feedback, flash messages, or any temporary visual indicator.

## Selectors

A Selector is a named enum where exactly one variant is active at a time. Derive it on an enum with unit variants:

```rust
use rwire::Selector;

#[derive(Selector)]
pub enum ActiveTab {
    #[default]
    Home,
    Settings,
    Profile,
}
```

The `#[default]` attribute marks the initially active variant.

### Binding elements

Use `.when_eq()` to add a style token when the selector matches a specific variant:

```rust
// Tab panels -- only the active one is visible
el(El::Div).st([St::DisplayNone]).when_eq(ActiveTab::Home, St::DisplayBlock)
el(El::Div).st([St::DisplayNone]).when_eq(ActiveTab::Settings, St::DisplayBlock)
el(El::Div).st([St::DisplayNone]).when_eq(ActiveTab::Profile, St::DisplayBlock)

// Tab buttons -- highlight the active one
el(El::Button).when_eq(ActiveTab::Home, St::BgPrimary)
el(El::Button).when_eq(ActiveTab::Settings, St::BgPrimary)
```

### Triggering selection

Use `.select()` to set the selector value on an event:

```rust
el(El::Button).text("Home").select(ActiveTab::Home, Ev::Click)
el(El::Button).text("Settings").select(ActiveTab::Settings, Ev::Click)
el(El::Button).text("Profile").select(ActiveTab::Profile, Ev::Click)
```

When a user clicks "Settings":
1. The selector value changes to `Settings`
2. The Home panel loses `DisplayBlock`, the Settings panel gains it
3. The Home button loses `BgPrimary`, the Settings button gains it

All updates happen in the same frame, with zero network traffic.

## Combining with Server Handlers

Client actions handle instant visual feedback. For actions that also need server logic, combine `.toggle()` or `.select()` with `.on()`:

```rust
el(El::Button)
    .text("Settings")
    .select(ActiveTab::Settings, Ev::Click)       // instant tab switch
    .on(Ev::Click, load_settings().debounce(200))  // lazy-load content
```

The tab switches instantly while the server loads data in the background. When the handler completes, the renderer updates with the new content.

## Combining Targets and Selectors

An element can use both targets and selectors, and mix them with regular styles, pseudo-classes, and breakpoints:

```rust
el(El::Div)
    .st([St::DisplayNone, St::BgSurface, St::P4, St::RoundedMd])
    .when::<ModalOpen>(St::DisplayFlex)
    .hover([St::BgSurfaceHover])
    .sm([St::P6])
```

## Use Cases

| Pattern | Primitive | Declaration | Trigger |
|---------|-----------|-------------|---------|
| Modal | Target | `#[derive(Target)] struct ModalOpen;` | `.toggle::<ModalOpen>(Ev::Click)` |
| Dropdown | Target | `#[derive(Target)] struct DropdownOpen;` | `.toggle::<DropdownOpen>(Ev::Click)` |
| Sidebar | Target | `#[derive(Target)] struct SidebarOpen;` | `.toggle::<SidebarOpen>(Ev::Click)` |
| Copy feedback | Target | `#[derive(Target)] struct CopyFeedback;` | `.toggle_timed::<CopyFeedback>(Ev::Click, 2000)` |
| Accordion | Multiple targets | One target per section | `.toggle::<Section1>(Ev::Click)` |
| Tabs | Selector | `#[derive(Selector)] enum Tab { A, B }` | `.select(Tab::A, Ev::Click)` |
| Multi-step form | Selector | `#[derive(Selector)] enum Step { One, Two }` | `.select(Step::Two, Ev::Click)` |
| View switcher | Selector | `#[derive(Selector)] enum View { Grid, List }` | `.select(View::Grid, Ev::Click)` |

## How It Works

Client actions compile down to binary opcodes that the JS runtime processes at page load:

1. `INIT_TARGET` / `INIT_SELECTOR` -- set default values
2. `BIND_TARGET` / `BIND_SELECTOR` -- register which elements gain/lose which CSS classes
3. `BIND_TOGGLE` / `BIND_SELECT` -- attach event listeners that update state and re-apply classes
4. `BIND_TOGGLE_TIMED` -- like toggle, but schedules a revert after a timeout

The JS runtime is ~250 bytes, tree-shaken out entirely when no targets or selectors are used.
