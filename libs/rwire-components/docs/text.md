---
title: Text
description: Typography component with semantic variants and color options
order: 300
component: text
---

## Import

```rust
use rwire_components::{Text, TextVariant, TextColor};
```

## Usage

```rust
Text::heading1("Welcome to rwire").build()
Text::body("Regular paragraph text").build()
Text::caption("Small helper text").muted().build()
```

## Variant Constructors

```rust
Text::heading1("Page Title").build()      // h1
Text::heading2("Section Title").build()   // h2
Text::heading3("Subsection").build()      // h3
Text::body("Paragraph text").build()      // body (default)
Text::body_small("Fine print").build()    // smaller body
Text::label("Field Label").build()        // label style
Text::caption("Helper text").build()      // caption style
```

## Colors

```rust
Text::body("Default color").build()
Text::body("High contrast").color(TextColor::High).build()
Text::body("Muted text").muted().build()
Text::body("Accent color").accent().build()
Text::body("Success").color(TextColor::Success).build()
Text::body("Warning").color(TextColor::Warning).build()
Text::body("Error").color(TextColor::Error).build()
```

## Accessibility

- Heading variants render semantic heading elements (h1-h3)
- Body and caption render paragraph or span elements
- Color variants maintain sufficient contrast ratios
