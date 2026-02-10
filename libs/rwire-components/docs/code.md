---
title: Code
description: Inline code and code blocks for documentation and code display
order: 303
component: code
---

## Import

```rust
use rwire_components::{Code, CodeMode};
```

## Usage

```rust
// Inline code
Code::inline("let x = 42").build()

// Code block
Code::block("fn main() {\n    println!(\"Hello\");\n}")
    .language("rust")
    .build()
```

## Modes

```rust
Code::inline("npm install").build()  // inline span
Code::block("multi\nline\ncode").build() // block with pre/code
```

## Language Label

```rust
// Adds a language label to code blocks
Code::block("SELECT * FROM users")
    .language("sql")
    .build()
```

## Accessibility

- Inline code renders as a `<code>` element
- Block code renders with monospace font and preserves whitespace
