---
title: Badge
description: Status indicator with color variants
order: 301
component: badge
---

## Import

```rust
use rwire_components::{Badge, BadgeIntent};
```

## Usage

```rust
Badge::default_badge("New").build()
Badge::primary("Featured").build()
Badge::success("Active").build()
Badge::warning("Pending").build()
Badge::error("Failed").build()
```

## Variants

```rust
Badge::new().intent(BadgeIntent::Default).text("Default").build()
Badge::new().intent(BadgeIntent::Primary).text("Primary").build()
Badge::new().intent(BadgeIntent::Success).text("Success").build()
Badge::new().intent(BadgeIntent::Warning).text("Warning").build()
Badge::new().intent(BadgeIntent::Error).text("Error").build()
```

## Accessibility

- Badge is a visual indicator; ensure important status info is also conveyed in text
- Use alongside descriptive labels for screen reader context
