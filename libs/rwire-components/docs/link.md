---
title: Link
description: Anchor element with styling for internal and external links
order: 500
component: link
---

## Import

```rust
use rwire_components::Link;
```

## Usage

```rust
Link::new("/about").text("About Us").build()
Link::external("https://example.com").text("Example").build()
```

## Internal Links

```rust
// Standard internal navigation
Link::new("/docs/getting-started").text("Getting Started").build()
```

## External Links

```rust
// Opens in new tab with rel="noopener noreferrer"
Link::external("https://github.com/example/rwire")
    .text("GitHub")
    .build()
```

## Accessibility

- Renders a native `<a>` element
- External links open in a new tab with security attributes
- Accent-colored text with no underline by default
