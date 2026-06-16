# Accordion — Client Action Migration

**File**: `libs/rwire-components/src/accordion.rs`
**Primitive**: Target (one per item)
**Tier**: 1 (High Impact)
**Complexity**: High

## Current Behavior

Each `AccordionItem` has an `open: bool` field. In `build()`, open items get `compute_content_open_tokens()` (visible, padded) and closed items get `compute_content_closed_tokens()` (DisplayNone, MaxH0). Toggle requires a server handler per item.

```rust
// Current — one handler per item, server round-trip for each
Accordion::new()
    .item(AccordionItem::new("Getting Started")
        .open(state.open_sections.get(0).copied().unwrap_or(false))
        .on_toggle(toggle_section_0())
        .content(el(El::P).text("Welcome!")))
    .item(AccordionItem::new("API Reference")
        .open(state.open_sections.get(1).copied().unwrap_or(false))
        .on_toggle(toggle_section_1())
        .content(el(El::P).text("See the docs.")))
    .build()
```

**Pain**: N items means N handlers, N state fields, N server round-trips for expand/collapse.

## Target State

```rust
#[derive(Target)]
struct FaqSection0;
#[derive(Target)]
struct FaqSection1;

fn faq() -> ElementBuilder {
    Accordion::new()
        .item(AccordionItem::new("Getting Started")
            .client_toggle::<FaqSection0>()
            .content(el(El::P).text("Welcome!")))
        .item(AccordionItem::new("API Reference")
            .client_toggle::<FaqSection1>()
            .content(el(El::P).text("See the docs.")))
        .build()
}
```

Zero server handlers. Each section toggles independently in the browser.

## Implementation

### 1. AccordionItem stores Target TypeId

```rust
pub struct AccordionItem {
    title: Cow<'static, str>,
    open: bool,
    on_toggle: Option<HandlerSpec>,
    target_type_id: Option<TypeId>,  // NEW
    content: Option<ElementBuilder>,
}

impl AccordionItem {
    pub fn client_toggle<T: Target>(mut self) -> Self {
        self.target_type_id = Some(TypeId::of::<T>());
        self
    }
}
```

### 2. Build always emits both open and closed content

Currently:
```rust
let content_tokens = if item.open {
    Self::compute_content_open_tokens()
} else {
    Self::compute_content_closed_tokens()
};
```

After:
```rust
let mut content_el = el(El::Div);

if let Some(target_type_id) = item.target_type_id {
    // Client-action mode: start closed, open when target is true
    content_el = content_el
        .st(Self::compute_content_closed_tokens())
        .when_by_id(target_type_id, St::DisplayBlock)
        .when_by_id(target_type_id, St::PxMd)
        .when_by_id(target_type_id, St::PbMd);
    // ... but this needs multi-token .when() — see design note
} else {
    let content_tokens = if item.open {
        Self::compute_content_open_tokens()
    } else {
        Self::compute_content_closed_tokens()
    };
    content_el = content_el.st(content_tokens);
}
```

### Design Note: Multi-token when()

The current `.when::<T>(st: St)` takes a single St token. Accordion needs to swap multiple tokens (DisplayNone → visible + padding). Options:

1. **Multiple `.when()` calls** — `.when::<T>(St::DisplayBlock).when::<T>(St::PxMd).when::<T>(St::PbMd)` — works but verbose
2. **`.when_all::<T>(tokens: &[St])`** — new builder method for multiple tokens per binding
3. **Composite token** — add `St::AccordionContentOpen` that combines the styles

Recommendation: option 2 (`.when_all`) — most ergonomic and generalizes to other components.

### 3. Trigger button gets toggle

```rust
if let Some(target_type_id) = item.target_type_id {
    trigger = trigger.toggle_by_id(target_type_id, Ev::Click);
} else if let Some(handler) = item.on_toggle {
    trigger = trigger.on(Ev::Click, handler);
}
```

### 4. aria-expanded tracking

Currently `aria-expanded` is set at build time based on `item.open`. With client actions, the attribute needs to toggle too. This requires either:
- A new opcode for toggling attributes (future work)
- Accepting that `aria-expanded` stays static until the server re-renders

For now, accept the limitation. Add a note about future `TOGGLE_ATTR` opcode.

## Exclusive Mode (Future)

Some accordions allow only one section open at a time. This could be modeled as a Selector:

```rust
#[derive(Selector)]
enum FaqSection {
    #[default]
    None,
    GettingStarted,
    ApiReference,
}
```

This is a future enhancement — start with independent Targets per item.

## Testing

- `test_accordion_client_toggle` — item stores target_type_id
- `test_accordion_content_always_emitted` — content DOM present even when Target is false
- `test_accordion_server_mode_unchanged` — `open(false)` + `on_toggle()` still works
- `test_accordion_trigger_has_toggle` — trigger button gets toggle binding
