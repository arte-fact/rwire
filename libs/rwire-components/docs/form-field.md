---
title: FormField
description: Form field wrapper with label, help text, and error messages
order: 209
component: form-field
---

## Import

```rust
use rwire_components::{FormField, Input};
```

## Usage

```rust
FormField::new()
    .label("Email")
    .input(Input::email().name("email").build())
    .help("We'll never share your email")
    .build()
```

## With Error

```rust
FormField::new()
    .label("Password")
    .input(Input::password().name("password").invalid(true).build())
    .error("Password must be at least 8 characters")
    .required(true)
    .build()
```

## Parts

```rust
FormField::new()
    .label("Name")       // top label text
    .input(input.build()) // the form control
    .help("Help text")   // subtle helper below input
    .error("Error msg")  // red error below input (overrides help)
    .required(true)      // adds asterisk to label
    .build()
```

## Accessibility

- Automatically associates label with input via ID
- Error messages use `aria-describedby` for screen reader announcements
- Required state is reflected in both label and input
