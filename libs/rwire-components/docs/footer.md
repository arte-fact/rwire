---
title: Footer
description: Multi-column footer with logo area, link columns, and copyright
order: 505
component: footer
---

## Import

```rust
use rwire_components::{Footer, FooterColumn};
```

## Usage

```rust
Footer::new()
    .logo(el(El::Span).text("rwire"))
    .tagline("Server-side UI framework")
    .column(FooterColumn::new("Docs")
        .link("Getting Started", "/docs/getting-started")
        .link("API Reference", "/docs/api"))
    .column(FooterColumn::new("Community")
        .external_link("GitHub", "https://github.com/example/rwire")
        .external_link("Discord", "https://discord.gg/example"))
    .copyright("2026 rwire contributors")
    .build()
```

## Link Columns

```rust
// Internal links use client-side routing
FooterColumn::new("Product")
    .link("Features", "/features")
    .link("Pricing", "/pricing")

// External links open in new tabs
FooterColumn::new("Social")
    .external_link("Twitter", "https://twitter.com/example")
    .external_link("GitHub", "https://github.com/example")
```

## Minimal Footer

```rust
Footer::new()
    .copyright("2026 My App")
    .build()
```

## Accessibility

- Internal links use client-side routing
- External links include `target="_blank"` and `rel="noopener noreferrer"`
- Footer renders within appropriate semantic structure
