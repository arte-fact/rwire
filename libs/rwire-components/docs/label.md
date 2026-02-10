---
title: Label
description: Form label with optional required indicator
order: 208
component: label
---

## Import

```rust
use rwire_components::Label;
```

## Usage

```rust
Label::new("Email").build()
Label::new("Password").required(true).build()
```

## Required Indicator

```rust
// Adds a visual asterisk (*) to the label
Label::new("Username").required(true).build()
```

## Accessibility

- Renders a native `<label>` element
- Use `.attr("for", "input-id")` on the built element to associate with an input
- Prefer `FormField` for automatic label-input association
