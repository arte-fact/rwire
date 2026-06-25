---
title: Data Display Components
description: Card, Badge, Table, Code, Timeline, Stat
order: 4
---
# Data Display Components

## Card

Surface container with padding, border, and shadow.

```rust
use rwire_components::{Card, CardPadding, CardShadow};

// Default: medium padding, small shadow, bordered
Card::new()
    .child(content)
    .build()

// Large padding, no border, medium shadow
Card::new()
    .padding(CardPadding::Lg)
    .shadow(CardShadow::Md)
    .bordered(false)
    .child(content)
    .build()
```

| Prop | Options | Default |
|------|---------|---------|
| `padding` | `None`, `Sm`, `Md`, `Lg` | `Md` |
| `shadow` | `None`, `Sm`, `Md`, `Lg` | `Sm` |
| `bordered` | `bool` | `true` |

---

## Badge

Inline status indicator with color variants.

```rust
use rwire_components::Badge;

Badge::success("Active").build()
Badge::warning("Pending").build()
Badge::error("Failed").build()
Badge::primary("New").build()
Badge::default_badge("Draft").build()
```

| Intent | Background | Text Color |
|--------|-----------|------------|
| `Default` | Emphasis | High |
| `Primary` | PrimarySubtle | OnPrimarySubtle |
| `Success` | Green | Green |
| `Warning` | Amber | Amber |
| `Error` | Red | Red |

---

## Table

Semantic HTML table with headers, rows, and optional striping.

```rust
use rwire_components::{Table, TableRow};

Table::new()
    .headers(["Name", "Email", "Role"])
    .row(TableRow::new().cells(["Alice", "alice@example.com", "Admin"]))
    .row(TableRow::new().cells(["Bob", "bob@example.com", "User"]))
    .striped(true)
    .build()
```

Renders proper `<table>`, `<thead>`, `<tbody>`, `<tr>`, `<th>`, and `<td>` elements. Striped mode applies alternating row backgrounds via `:nth-child(even)`.

---

## Code

Inline code spans and multi-line code blocks.

```rust
use rwire_components::Code;

// Inline code (renders <code>)
Code::inline("let x = 42").build()

// Code block with language label (renders <pre><code>)
Code::block("fn main() {\n    println!(\"Hello\");\n}")
    .language("rust")
    .build()
```

Inline code gets a subtle background, monospace font, and rounded corners. Code blocks add horizontal scroll overflow and an optional language label above the block.

---

## Stat

Metric display with value, label, and optional trend indicator.

```rust
use rwire_components::{Stat, StatTrend};

Stat::new("1,234")
    .label("Active Users")
    .trend(StatTrend::Up, "+12%")
    .build()

Stat::new("99.9%")
    .label("Uptime")
    .description("Last 30 days")
    .trend(StatTrend::Neutral, "0%")
    .build()
```

Trends render with an arrow indicator: `Up` shows green with an up arrow, `Down` shows red with a down arrow, `Neutral` shows muted text.

---

## Timeline

Vertical event timeline with status dots, timestamps, and descriptions.

```rust
use rwire_components::{Timeline, TimelineItem};

Timeline::new()
    .item(TimelineItem::new("Deployed to production")
        .description("v2.1.0 release")
        .time("2m ago")
        .active(true))
    .item(TimelineItem::new("Tests passed").time("5m ago"))
    .item(TimelineItem::new("PR merged").time("10m ago"))
    .build()
```

Active items get an accent-colored dot. A connecting line runs between items (hidden on the last item). Each item supports an optional description and timestamp.

---

## Text

Typography component for headings, body copy, and captions with consistent sizing and color.

```rust
use rwire_components::{Text, TextVariant, TextColor};

Text::heading1("Introduction").build()
Text::body("Welcome to the documentation.").build()
Text::caption("Last updated today").muted().build()

// Explicit variant + color
Text::new()
    .variant(TextVariant::Body)
    .color(TextColor::Accent)
    .build()
```

Constructors: `Text::heading1/2/3()`, `Text::body()`, `Text::body_small()`, `Text::caption()`.
Variants: `Body`, `BodySmall`, `Label`, `Caption`. Colors: `Default`, `High`, `Muted`, `Accent`,
`Success`, `Warning`, `Error` (or the `.muted()` shorthand).
