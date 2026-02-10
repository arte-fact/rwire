---
title: Input
description: Text input with type variants, sizes, and validation states
order: 201
component: input
---

## Import

```rust
use rwire_components::{Input, InputType, InputSize};
```

## Usage

```rust
Input::text()
    .placeholder("Enter your name")
    .name("username")
    .build()

Input::password()
    .placeholder("Password")
    .required(true)
    .build()
```

## Type Constructors

```rust
Input::text().build()     // text input (default)
Input::password().build() // password input
Input::email().build()    // email input
Input::number().build()   // number input
Input::search().build()   // search input
```

## Sizes

```rust
Input::text().size(InputSize::Sm).build() // 28px height
Input::text().size(InputSize::Md).build() // 36px height (default)
Input::text().size(InputSize::Lg).build() // 44px height
```

## States

```rust
Input::text().disabled(true).build()
Input::text().readonly(true).build()
Input::text().required(true).build()
Input::text().invalid(true).build()
```

## Accessibility

- Renders a native `<input>` element with correct `type` attribute
- Pair with `Label` or `FormField` for accessible labelling
- Invalid state adds `aria-invalid="true"`
