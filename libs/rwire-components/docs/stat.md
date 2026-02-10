---
title: Stat
description: Metric display with value, label, and optional trend indicator
order: 306
component: stat
---

## Import

```rust
use rwire_components::{Stat, StatTrend};
```

## Usage

```rust
Stat::new("1,234")
    .label("Active Users")
    .trend(StatTrend::Up, "+12%")
    .build()
```

## With Description

```rust
Stat::new("$45,231")
    .label("Revenue")
    .description("Monthly recurring revenue")
    .trend(StatTrend::Up, "+8.2%")
    .build()
```

## Trend Variants

```rust
Stat::new("99.9%").label("Uptime").trend(StatTrend::Up, "+0.1%").build()
Stat::new("23ms").label("Latency").trend(StatTrend::Down, "-5ms").build()
Stat::new("150").label("Users").trend(StatTrend::Neutral, "No change").build()
```

## Accessibility

- Value is prominently displayed with large text
- Trend uses color coding: green (up), red (down), muted (neutral)
- Label provides context for the metric value
