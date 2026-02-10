---
title: Accordion
description: Collapsible content sections controlled by server state
order: 603
component: accordion
---

## Import

```rust
use rwire_components::{Accordion, AccordionItem};
```

## Usage

```rust
#[derive(State, Default)]
#[storage(memory)]
struct DocsState {
    open_sections: Vec<bool>,
}

#[renderer]
fn render_faq(state: &DocsState) -> ElementBuilder {
    Accordion::new()
        .item(AccordionItem::new("Getting Started")
            .open(state.open_sections.get(0).copied().unwrap_or(false))
            .on_toggle(toggle_section_0())
            .content(el(El::P).text("Welcome to rwire!")))
        .item(AccordionItem::new("API Reference")
            .open(state.open_sections.get(1).copied().unwrap_or(false))
            .on_toggle(toggle_section_1())
            .content(el(El::P).text("See the API docs.")))
        .build()
}
```

## Accordion Items

```rust
AccordionItem::new("Section Title")
    .open(true)                        // expanded state
    .on_toggle(toggle_handler())       // toggle handler
    .content(el(El::P).text("Body"))   // collapsible content
```

## Accessibility

- Each section has a clickable trigger that toggles content visibility
- Open/closed state is server-controlled
- Content is hidden when collapsed
