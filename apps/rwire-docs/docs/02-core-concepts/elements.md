---
title: Elements
description: Building DOM trees with the fluent el() API
order: 4
---

# Elements

```rust
use rwire::{el, El, Ev, St};

fn greeting() -> ElementBuilder {
    el(El::Div).class("greeting").append([
        el(El::H1).text("Hello, rwire"),
        el(El::P).text("Server-rendered, binary-encoded UI."),
        el(El::Button)
            .text("Click me")
            .on(Ev::Click, handle_click()),
    ])
}
```

The `el()` function creates an `ElementBuilder` for the given element type. Chain methods to set attributes, append children, and bind events. The builder compiles down to binary opcodes -- no HTML string generation.

## Element Types

The `El` enum covers standard HTML elements:

| Category | Elements |
|----------|----------|
| Layout | `Div`, `Span`, `Section`, `Article`, `Nav`, `Header`, `Footer` |
| Text | `H1`, `H2`, `H3`, `P`, `A`, `Label`, `Legend` |
| Form | `Input`, `Textarea`, `Select`, `Option`, `Button`, `Form`, `Fieldset` |
| List | `Ul`, `Ol`, `Li` |
| Other | `Hr`, `Svg`, `Path` |

Each element type is a single byte on the wire. Only the types your app actually uses are included in the generated JavaScript runtime.

## Builder Methods

```rust
el(El::Input)
    .class("search-input")          // CSS class
    .text("placeholder text")        // textContent
    .attr("type", "text")            // HTML attribute (string)
    .at(At::Placeholder, Av::None)   // Binary-encoded attribute
    .on(Ev::Input, handle_input())   // Event handler
    .append([child1, child2])        // Child elements
```

## Style Tokens

Instead of writing CSS class strings, use the `St` enum for type-safe styling:

```rust
el(El::Div).st([
    St::DisplayFlex,
    St::ItemsCenter,
    St::GapMd,
    St::BgSurface,
    St::RoundedMd,
])
```

Style tokens compile to single-class CSS rules. The server encodes them as varint numbers (1-2 bytes each), and the browser maps them to generated CSS classes. The CSS rule for each token is delivered lazily over the WebSocket the first time it's used, so the client only receives the styles your app actually renders.

## Pseudo-Class Styles

Add hover, focus, and active styles with dedicated methods:

```rust
el(El::Button)
    .st([St::BgPrimary, St::TextOnPrimary, St::PxMd, St::PySm])
    .hover([St::BgPrimaryHover])
    .focus([St::RingFocus, St::OutlineNone])
```

Each pseudo-class group generates a CSS rule (e.g. `.h1u772:hover { ... }`), delivered lazily over the wire the first time it's used.

## Appending Children

`.append()` accepts anything that implements `IntoIterator<Item = ElementBuilder>`, so arrays, vectors, and iterator chains all work:

```rust
el(El::Ul).append(
    items.iter().map(|item| {
        el(El::Li).text(&item.name)
    })
)
```
