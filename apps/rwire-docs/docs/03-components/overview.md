---
title: Component Overview
description: rwire's component philosophy and builder pattern
order: 1
---
# Component Overview

```rust
use rwire_components::*;

// Every component follows the same pattern:
// Constructor -> fluent setters -> .build()
Button::primary("Submit")
    .size(ButtonSize::Lg)
    .on_click(save_handler())

Card::new()
    .padding(CardPadding::Lg)
    .shadow(CardShadow::Md)
    .child(content)
    .build()
```

## Import

A single glob import brings in all components, their enums, and helper types:

```rust
use rwire_components::*;
```

## Builder Pattern

Components are builder structs that produce `ElementBuilder` instances. The pattern is always:

1. **Constructor** -- `ComponentName::new()` for defaults, or a named constructor like `Button::primary("Save")`, `Input::email()`, `Badge::success("Active")`.
2. **Fluent setters** -- chain `.size()`, `.intent()`, `.disabled()`, and other options.
3. **Build** -- call `.build()` to get an `ElementBuilder` you can compose into your tree. Some components offer shorthand terminals like `.on_click(handler)` that build and attach an event in one step.

```rust
// Verbose
Button::new()
    .intent(ButtonIntent::Destructive)
    .size(ButtonSize::Sm)
    .text("Delete")
    .build()
    .on(Ev::Click, delete_handler())

// Shorthand -- .on_click() calls .build() internally
Button::destructive("Delete")
    .size(ButtonSize::Sm)
    .on_click(delete_handler())
```

## Style Tokens

Components use `St` tokens (a `#[repr(u16)]` enum) for all styling. Tokens map to atomic CSS classes that are tree-shaken at startup -- only the tokens your app actually uses ship to the browser.

```rust
// Components compute their own tokens internally:
let tokens = Button::primary("Save").compute_tokens();
// => [St::DisplayInlineFlex, St::BgPrimary, St::TextOnPrimary, ...]

// You can add extra tokens with .st():
Card::new()
    .child(content)
    .build()
    .st([St::MtLg])  // add margin-top on the ElementBuilder
```

Pseudo-class styles (hover, focus, disabled) are applied automatically by each component via `.hover()`, `.focus_visible()`, and `.disabled_style()`.

## Escape Hatches

Every component exposes `.class()` for adding a custom CSS class string and `.attr()` on the resulting `ElementBuilder` for arbitrary HTML attributes:

```rust
Button::primary("Custom")
    .class("my-special-button")
    .build()
    .attr("data-testid", "submit-btn")
```

## Composition

Components compose naturally because they all return `ElementBuilder`:

```rust
Stack::column()
    .gap(Gap::Lg)
    .children([
        Card::new()
            .child(
                Stack::row().gap(Gap::Sm).children([
                    Input::text().placeholder("Search...").build(),
                    Button::primary("Go").build(),
                ]).build()
            )
            .build(),
        Stack::row().gap(Gap::Sm).children([
            Badge::success("Active").build(),
            Badge::warning("Pending").build(),
        ]).build(),
    ])
    .build()
```

## Full Component List

rwire ships 50+ components organized into categories:

- **Layout**: Stack, Grid, Container, AppShell, Spacer, Divider
- **Navigation**: Link, NavMenu, Breadcrumb, Tabs, Pagination
- **Data Display**: Card, Badge, Table, Code, Stat, Timeline, Prose, Tag, Avatar, Image, Kbd
- **Forms**: Button, Input, Textarea, Select, Checkbox, Radio, Switch, FormField, Slider
- **Feedback**: Alert, Toast, Spinner, Progress, Skeleton, Modal, Drawer, EmptyState, Tooltip
- **Utilities**: CopyButton, ThemeToggle, Accordion, Blockquote, Text, List, Stepper, TableOfContents
