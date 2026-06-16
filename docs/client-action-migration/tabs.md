# Tabs — Client Action Migration

**File**: `libs/rwire-components/src/tabs.rs`
**Primitive**: Selector
**Tier**: 2 (High Impact)
**Complexity**: High

## Current Behavior

Tabs uses a server-owned `active_index: usize`. In `build()`, the tab list renders all buttons but only the active tab gets accent styling. Only the active content panel is rendered — inactive panels are not in the DOM at all.

```rust
// Current — server controls which tab is active
Tabs::new()
    .tab(Tab::new("Overview", overview.build()))
    .tab(Tab::new("Settings", settings.build()))
    .active(state.active_tab)
    .build()
```

Tab buttons have no click handlers — switching tabs requires a custom handler that sets `active_tab` and triggers a full re-render.

**Latency**: Tab switch requires round-trip + server re-renders all content panels.

## Target State

```rust
#[derive(Selector)]
enum MyTab {
    #[default]
    Overview,
    Settings,
}

fn settings_page() -> ElementBuilder {
    Tabs::new()
        .tab(Tab::new("Overview", overview.build()).variant(MyTab::Overview))
        .tab(Tab::new("Settings", settings.build()).variant(MyTab::Settings))
        .client_select::<MyTab>()
        .build()
}
```

All panels are in the DOM. Clicking a tab button instantly shows its panel and highlights the button.

## Implementation

### 1. Tab stores Selector variant value

```rust
pub struct Tab {
    label: Cow<'static, str>,
    content: ElementBuilder,
    variant_value: Option<u8>,  // NEW
}

impl Tab {
    pub fn variant<S: Selector>(mut self, variant: S) -> Self {
        self.variant_value = Some(variant.variant_value());
        self
    }
}
```

### 2. Tabs stores Selector TypeId

```rust
pub struct Tabs {
    tabs: Vec<Tab>,
    active_index: usize,
    selector_type_id: Option<TypeId>,  // NEW
    selector_default: Option<u8>,      // NEW
}

impl Tabs {
    pub fn client_select<S: Selector>(mut self) -> Self {
        self.selector_type_id = Some(TypeId::of::<S>());
        self.selector_default = Some(S::default_value());
        self
    }
}
```

### 3. All panels always emitted

Currently only the active panel is rendered:
```rust
if let Some(active_tab) = self.tabs.into_iter().nth(self.active_index) { ... }
```

After:
```rust
for (idx, tab) in self.tabs.into_iter().enumerate() {
    let mut panel = el(El::Div)
        .st([St::PySm, St::Px0])
        .at(At::Role, Av::RoleTabpanel);

    if let Some(selector_type_id) = self.selector_type_id {
        let variant_val = tab.variant_value.unwrap_or(idx as u8);
        panel = panel
            .st([St::DisplayNone])
            .when_eq_by_id(selector_type_id, variant_val, St::DisplayBlock);
    } else if idx != self.active_index {
        panel = panel.st([St::DisplayNone]);
    }

    panel = panel.append([tab.content]);
    container = container.append([panel]);
}
```

### 4. Tab buttons get select trigger

```rust
for (idx, tab) in self.tabs.iter().enumerate() {
    // ... build button with styling ...

    if let Some(selector_type_id) = self.selector_type_id {
        let variant_val = tab.variant_value.unwrap_or(idx as u8);
        button = button
            .select_by_id(selector_type_id, variant_val, Ev::Click);
        // Active styling via selector
        button = button
            .st([St::TextMedium, St::BorderB2Transparent])  // default inactive
            .when_eq_by_id(selector_type_id, variant_val, St::TextAccent)
            .when_eq_by_id(selector_type_id, variant_val, St::BorderB2Accent);
    } else {
        // Existing server-controlled path
        if is_active { ... } else { ... }
    }
}
```

### 5. aria-selected tracking

Like accordion's `aria-expanded`, `aria-selected` is currently set at build time. With client actions, it won't dynamically toggle. Accept this limitation for now.

## Design Note: when_eq multi-token

Like Accordion, Tabs needs multiple tokens per selector match. The `.when_eq()` method currently takes a single St. Need either multiple calls or a `.when_eq_all()` variant.

## Alternative: Index-based Selector

For apps with dynamic tab counts, a user-defined Selector enum doesn't work. Support an index-based fallback:

```rust
// Dynamic tabs — no enum needed
Tabs::new()
    .tab(Tab::new("Tab 1", content1.build()))
    .tab(Tab::new("Tab 2", content2.build()))
    .client_select_index()  // uses internal index-based selector
    .build()
```

This requires the component to manage its own selector index internally. Lower priority — the enum approach covers most use cases.

## Testing

- `test_tabs_client_select` — verify selector_type_id set
- `test_tabs_all_panels_rendered` — all content panels in DOM
- `test_tabs_only_active_visible` — inactive panels have DisplayNone, active has DisplayBlock
- `test_tabs_buttons_have_select` — each button triggers its variant
- `test_tabs_server_mode_unchanged` — `.active(1)` still works as before
