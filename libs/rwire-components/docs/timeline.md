---
title: Timeline
description: Vertical event timeline with timestamps and status dots
order: 312
component: timeline
---

## Import

```rust
use rwire_components::{Timeline, TimelineItem};
```

## Usage

```rust
Timeline::new()
    .item(TimelineItem::new("Deployed to production")
        .time("2m ago")
        .active(true))
    .item(TimelineItem::new("Tests passed")
        .time("5m ago"))
    .item(TimelineItem::new("PR merged")
        .time("10m ago"))
    .build()
```

## Item Options

```rust
TimelineItem::new("Event Title")
    .description("Additional details about this event")
    .time("Jan 15, 2026")
    .active(true) // highlights as current/active step
```

## Accessibility

- Timeline items are visually connected with a vertical line
- Active items are highlighted with accent color
- Time labels provide temporal context
