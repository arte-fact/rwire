---
title: Form Components
description: Button, Input, Select, Checkbox, Radio, Switch
order: 5
---
# Form Components

## Button

```rust
use rwire::components::{Button, ButtonIntent, ButtonSize};

// Convenience constructors (most common)
Button::primary("Save").on_click(save_handler())
Button::secondary("Cancel").on_click(cancel_handler())
Button::ghost("More").build()
Button::destructive("Delete").on_click(delete_handler())

// Full configuration
Button::new()
    .intent(ButtonIntent::Primary)
    .size(ButtonSize::Lg)
    .text("Submit")
    .disabled(true)
    .loading(true)
    .full_width(true)
    .build()
```

### Intents

| Intent | Use Case |
|--------|----------|
| `Primary` | Main action (solid accent) |
| `Secondary` | Alternative action (subtle bg, border) |
| `Ghost` | Minimal weight (transparent, text only) |
| `Destructive` | Dangerous actions (red) |

### Sizes

`Sm` (28px), `Md` (36px, default), `Lg` (44px). Loading state shows a CSS spinner overlay with `aria-busy="true"`.

---

## Input

```rust
use rwire::components::{Input, InputSize};

// Type-specific constructors
Input::text().placeholder("Name").build()
Input::email().placeholder("user@example.com").build()
Input::password().placeholder("Password").required(true).build()
Input::search().placeholder("Search...").build()
Input::number().value("0").build()

// With event handler
Input::text()
    .placeholder("Type here")
    .name("query")
    .on_input(handle_input())

// Validation state
Input::email()
    .value("bad")
    .invalid(true)
    .build()
```

Available types: `Text`, `Password`, `Email`, `Number`, `Search`, `Tel`, `Url`. Sizes: `Sm`, `Md` (default), `Lg`. Inputs get automatic pseudo-class styles for `:hover` (border emphasis), `:focus` (primary border), `::placeholder` (muted text), and `:disabled` (opacity + no pointer events).

---

## Textarea

```rust
use rwire::components::Textarea;

Textarea::new()
    .placeholder("Enter description")
    .rows(6)
    .build()

Textarea::new()
    .value("Existing text")
    .readonly(true)
    .build()
```

Defaults to 4 visible rows. Supports the same states as Input: `disabled`, `readonly`, `required`, `invalid`. Sizes: `Sm`, `Md` (default), `Lg`.

---

## Select

```rust
use rwire::components::Select;

Select::new()
    .option("us", "United States")
    .option("ca", "Canada")
    .option("uk", "United Kingdom")
    .value("us")
    .on_change(handle_country())
```

Renders a native `<select>` wrapped in a positioned container with a custom dropdown arrow SVG. Supports `disabled`, `required`, and `invalid` states.

---

## Checkbox

```rust
use rwire::components::Checkbox;

// Standalone
Checkbox::new().name("terms").on_change(handle_terms())

// With label (auto-generates ID for <label for="..."> association)
Checkbox::new()
    .label("Subscribe to newsletter")
    .checked(true)
    .build()
```

When a label is provided, the checkbox wraps in a `Stack::row()` with the `<label>` element. Pseudo-class styles handle `:hover` (border emphasis), `:checked` (primary bg), and `:focus-visible` (focus ring).

---

## Radio

```rust
use rwire::components::Radio;

// Radio group (same name attribute)
Radio::new().name("plan").value("free").label("Free Plan").build()
Radio::new().name("plan").value("pro").label("Pro Plan").checked(true).build()
Radio::new().name("plan").value("team").label("Team Plan").build()
```

Same label-association pattern as Checkbox. Group radios by setting the same `name` on each.

---

## Switch

```rust
use rwire::components::Switch;

Switch::new()
    .label("Enable notifications")
    .checked(true)
    .on_change(toggle_notifications())

Switch::new()
    .label("Dark mode")
    .disabled(true)
    .build()
```

Renders as a styled `<input type="checkbox" role="switch">` with a sliding thumb via `::after`. The thumb translates on `:checked`.

---

## FormField

Wrapper that adds a label, help text, and error message around any input.

```rust
use rwire::components::{FormField, Input};

FormField::new()
    .label("Email")
    .required(true)
    .input(Input::email().placeholder("you@example.com").build())
    .help("We'll never share your email")
    .build()

FormField::new()
    .label("Password")
    .input(Input::password().invalid(true).build())
    .error("Password must be at least 8 characters")
    .build()
```

FormField auto-generates an ID and wires the `<label for="">` to the input. Required fields show a red asterisk. Error text renders in the error color below the input.

---

## Event Binding Patterns

Form components offer shorthand methods that call `.build()` and attach a handler in one step:

```rust
// These are equivalent:
Button::primary("Save").on_click(handler())
Button::primary("Save").build().on(Ev::Click, handler())

// Input shortcuts:
Input::text().on_input(handler())   // fires on every keystroke
Input::text().on_change(handler())  // fires on blur/commit

// Select/Checkbox/Switch/Radio:
Select::new().option("a", "A").on_change(handler())
Checkbox::new().on_change(handler())
Switch::new().on_change(handler())
```

All event handlers are server round-trips. The browser sends the event over WebSocket, the server mutates state, and re-renders the affected regions.
