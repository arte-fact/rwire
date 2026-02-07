---
title: Input
description: Text input field with types and validation
order: 2
---
# Input

Text input fields for collecting user data. Supports various HTML input types.

## Usage

```rust
use rwire::components::{Input, InputSize, InputType};

// Basic text input
Input::text()
    .placeholder("Enter your name")
    .build()

// Email input with small size
Input::email()
    .placeholder("user@example.com")
    .size(InputSize::Sm)
    .build()

// Password input
Input::password()
    .placeholder("Password")
    .build()
```

## Input Types

| Type | Constructor | Description |
|------|------------|-------------|
| Text | `Input::text()` | General text input |
| Email | `Input::email()` | Email address |
| Password | `Input::password()` | Hidden password field |
| Search | `Input::search()` | Search input |
| Number | `Input::number()` | Numeric input |
| Tel | `Input::tel()` | Phone number |
| Url | `Input::url()` | URL input |

## Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `placeholder` | `&str` | — | Placeholder text |
| `size` | `InputSize` | `Md` | Size variant |
| `disabled` | `bool` | `false` | Disable the input |
| `required` | `bool` | `false` | Mark as required |
| `value` | `&str` | — | Current value |
| `on_input` | `HandlerSpec` | — | Input change handler |
