# Phase 6: Extended Components

## Goal

Expand the component library with additional commonly-needed UI components.

## Components in This Phase

### Form Components
| Component | Purpose |
|-----------|---------|
| Textarea | Multi-line text input |
| Checkbox | Boolean toggle |
| Radio | Single selection |
| Switch | Toggle switch |
| Select | Dropdown selection |
| Label | Form field label |
| FormField | Label + input + error wrapper |

### Data Display
| Component | Purpose |
|-----------|---------|
| Table | Data tables |
| Avatar | User image with fallback |
| Progress | Progress bar |
| Spinner | Loading indicator |

### Feedback
| Component | Purpose |
|-----------|---------|
| Alert | Info/success/warning/error messages |
| Toast | Temporary notifications (requires state) |

### Navigation
| Component | Purpose |
|-----------|---------|
| Tabs | Tab navigation |
| Breadcrumb | Path navigation |
| Pagination | Page navigation |

## Implementation Patterns

Each component follows the established pattern:

1. **Builder struct** with variant fields
2. **Fluent API** for configuration
3. **`compute_class()`** for CSS class generation
4. **`build()`** method that registers with ComponentRegistry
5. **Separate `*_css.rs`** file with component styles

### Example: Checkbox Component

**File: `rwire/src/components/checkbox.rs`**

```rust
use crate::{el, El, Ev, ElementBuilder};
use std::borrow::Cow;

#[derive(Clone, Debug, Default)]
pub struct Checkbox {
    checked: bool,
    indeterminate: bool,
    disabled: bool,
    label: Option<Cow<'static, str>>,
    name: Option<Cow<'static, str>>,
    id: Option<Cow<'static, str>>,
}

impl Checkbox {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn label(mut self, label: impl Into<Cow<'static, str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn id(mut self, id: impl Into<Cow<'static, str>>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn build(self) -> ElementBuilder {
        crate::components::registry::mark_component_used(
            crate::components::registry::ComponentType::Checkbox
        );

        let checkbox_id = self.id.clone()
            .unwrap_or_else(|| Cow::Owned(format!("cb-{}", rand_id())));

        let mut input = el(El::Input)
            .class("rw-checkbox")
            .attr("type", "checkbox")
            .id(&checkbox_id);

        if self.checked {
            input = input.attr("checked", "");
        }
        if self.disabled {
            input = input.attr("disabled", "");
        }
        if let Some(name) = self.name {
            input = input.attr("name", &name);
        }

        if let Some(label_text) = self.label {
            el(El::Label)
                .class("rw-checkbox-label")
                .attr("for", &checkbox_id)
                .append([
                    input,
                    el(El::Span).class("rw-checkbox-text").text(&label_text),
                ])
        } else {
            input
        }
    }

    pub fn on_change(self, handler: impl Into<crate::HandlerFn>) -> ElementBuilder {
        // Note: Need to handle the label wrapper case
        self.build().on(Ev::Change, handler.into())
    }
}

fn rand_id() -> String {
    // Simple pseudo-random ID for checkbox/label association
    format!("{:x}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u32)
}
```

**File: `rwire/src/components/checkbox_css.rs`**

```rust
pub fn checkbox_css() -> &'static str {
    r#"/* Checkbox */
.rw-checkbox{width:1rem;height:1rem;margin:0;accent-color:var(--rw-accent-9);cursor:pointer}
.rw-checkbox:disabled{cursor:not-allowed;opacity:.5}
.rw-checkbox-label{display:inline-flex;align-items:center;gap:var(--rw-space-2);cursor:pointer}
.rw-checkbox-label:has(.rw-checkbox:disabled){cursor:not-allowed;opacity:.5}
.rw-checkbox-text{font-size:var(--rw-text-sm);color:var(--rw-text-high)}
"#
}
```

### Example: Alert Component

**File: `rwire/src/components/alert.rs`**

```rust
use crate::{el, El, ElementBuilder};
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlertIntent {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Clone, Debug, Default)]
pub struct Alert {
    intent: AlertIntent,
    title: Option<Cow<'static, str>>,
    children: Vec<ElementBuilder>,
}

impl Alert {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn info() -> Self {
        Self::new().intent(AlertIntent::Info)
    }

    pub fn success() -> Self {
        Self::new().intent(AlertIntent::Success)
    }

    pub fn warning() -> Self {
        Self::new().intent(AlertIntent::Warning)
    }

    pub fn error() -> Self {
        Self::new().intent(AlertIntent::Error)
    }

    pub fn intent(mut self, intent: AlertIntent) -> Self {
        self.intent = intent;
        self
    }

    pub fn title(mut self, title: impl Into<Cow<'static, str>>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn child(mut self, child: ElementBuilder) -> Self {
        self.children.push(child);
        self
    }

    pub fn message(self, message: impl Into<Cow<'static, str>>) -> Self {
        self.child(el(El::P).class("rw-alert-message").text(&message.into()))
    }

    fn compute_class(&self) -> &'static str {
        match self.intent {
            AlertIntent::Info => "rw-alert rw-alert-info",
            AlertIntent::Success => "rw-alert rw-alert-success",
            AlertIntent::Warning => "rw-alert rw-alert-warning",
            AlertIntent::Error => "rw-alert rw-alert-error",
        }
    }

    pub fn build(self) -> ElementBuilder {
        crate::components::registry::mark_component_used(
            crate::components::registry::ComponentType::Alert
        );

        let mut builder = el(El::Div)
            .class(self.compute_class())
            .attr("role", "alert");

        if let Some(title) = self.title {
            builder = builder.append([
                el(El::Div).class("rw-alert-title").text(&title)
            ]);
        }

        for child in self.children {
            builder = builder.append([child]);
        }

        builder
    }
}
```

**File: `rwire/src/components/alert_css.rs`**

```rust
pub fn alert_css() -> &'static str {
    r#"/* Alert */
.rw-alert{padding:var(--rw-space-3) var(--rw-space-4);border-radius:var(--rw-radius-md);border:1px solid transparent}
.rw-alert-title{font-weight:var(--rw-font-semibold);margin-bottom:var(--rw-space-1)}
.rw-alert-message{margin:0;font-size:var(--rw-text-sm)}
.rw-alert-info{background:var(--rw-accent-2);border-color:var(--rw-accent-6);color:var(--rw-accent-11)}
.rw-alert-success{background:var(--rw-green-2);border-color:var(--rw-green-6);color:var(--rw-green-11)}
.rw-alert-warning{background:var(--rw-amber-2);border-color:var(--rw-amber-6);color:var(--rw-amber-11)}
.rw-alert-error{background:var(--rw-red-2);border-color:var(--rw-red-6);color:var(--rw-red-11)}
"#
}
```

### Example: Spinner Component

**File: `rwire/src/components/spinner.rs`**

```rust
use crate::{el, El, ElementBuilder};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpinnerSize {
    Sm,
    #[default]
    Md,
    Lg,
}

#[derive(Clone, Debug, Default)]
pub struct Spinner {
    size: SpinnerSize,
}

impl Spinner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn size(mut self, size: SpinnerSize) -> Self {
        self.size = size;
        self
    }

    pub fn sm() -> Self {
        Self::new().size(SpinnerSize::Sm)
    }

    pub fn lg() -> Self {
        Self::new().size(SpinnerSize::Lg)
    }

    fn compute_class(&self) -> &'static str {
        match self.size {
            SpinnerSize::Sm => "rw-spinner rw-spinner-sm",
            SpinnerSize::Md => "rw-spinner",
            SpinnerSize::Lg => "rw-spinner rw-spinner-lg",
        }
    }

    pub fn build(self) -> ElementBuilder {
        crate::components::registry::mark_component_used(
            crate::components::registry::ComponentType::Spinner
        );

        el(El::Span)
            .class(self.compute_class())
            .attr("role", "status")
            .attr("aria-label", "Loading")
    }
}
```

**File: `rwire/src/components/spinner_css.rs`**

```rust
pub fn spinner_css() -> &'static str {
    r#"/* Spinner */
.rw-spinner{display:inline-block;width:1.25rem;height:1.25rem;border:2px solid var(--rw-border-subtle);border-top-color:var(--rw-accent-9);border-radius:50%;animation:rw-spin .6s linear infinite}
.rw-spinner-sm{width:1rem;height:1rem;border-width:1.5px}
.rw-spinner-lg{width:1.75rem;height:1.75rem;border-width:3px}
@keyframes rw-spin{to{transform:rotate(360deg)}}
"#
}
```

## Deliverables

### Form Components
- [ ] `rwire/src/components/textarea.rs` + CSS
- [ ] `rwire/src/components/checkbox.rs` + CSS
- [ ] `rwire/src/components/radio.rs` + CSS
- [ ] `rwire/src/components/switch.rs` + CSS
- [ ] `rwire/src/components/select.rs` + CSS
- [ ] `rwire/src/components/label.rs` + CSS
- [ ] `rwire/src/components/form_field.rs` + CSS

### Data Display
- [ ] `rwire/src/components/table.rs` + CSS
- [ ] `rwire/src/components/avatar.rs` + CSS
- [ ] `rwire/src/components/progress.rs` + CSS
- [ ] `rwire/src/components/spinner.rs` + CSS

### Feedback
- [ ] `rwire/src/components/alert.rs` + CSS

### Navigation
- [ ] `rwire/src/components/tabs.rs` + CSS
- [ ] `rwire/src/components/breadcrumb.rs` + CSS
- [ ] `rwire/src/components/pagination.rs` + CSS

## Size Budget

| Category | Components | Max Total CSS |
|----------|------------|---------------|
| Form | 7 components | 2KB |
| Data Display | 4 components | 1.2KB |
| Feedback | 1 component | 400 bytes |
| Navigation | 3 components | 1KB |
| **Phase 6 Total** | 15 components | ~4.6KB |

## Component Interaction Patterns

### Forms with Handlers

```rust
// Checkbox with server-side state
Checkbox::new()
    .label("Remember me")
    .checked(state.remember_me)
    .build()
    .on(Ev::Change, toggle_remember())

// Radio group
Stack::column()
    .gap(Gap::Sm)
    .children([
        Radio::new()
            .name("plan")
            .value("free")
            .label("Free")
            .checked(state.plan == "free")
            .build()
            .on(Ev::Change, select_plan()),
        Radio::new()
            .name("plan")
            .value("pro")
            .label("Pro")
            .checked(state.plan == "pro")
            .build()
            .on(Ev::Change, select_plan()),
    ])
    .build()
```

### Data Display

```rust
// Avatar with fallback
Avatar::new()
    .src(&user.avatar_url)
    .fallback(&user.initials)
    .size(AvatarSize::Lg)
    .build()

// Progress bar
Progress::new()
    .value(75)
    .max(100)
    .build()

// Loading state
if state.loading {
    Spinner::new().build()
} else {
    render_content(state)
}
```

### Navigation

```rust
// Tabs
Tabs::new()
    .items([
        TabItem::new("overview", "Overview"),
        TabItem::new("settings", "Settings"),
        TabItem::new("users", "Users"),
    ])
    .active(&state.active_tab)
    .on_change(change_tab())
    .build()

// Breadcrumb
Breadcrumb::new()
    .items([
        ("Home", Some("/".into())),
        ("Products", Some("/products".into())),
        ("Widget", None), // Current page, no link
    ])
    .build()
```

## Future Considerations

### Components Not in This Phase

These require more complex state management or are less commonly needed:

- **Modal/Dialog** — needs portal rendering, focus trap
- **Dropdown Menu** — needs positioning, click-outside
- **Tooltip** — needs positioning, delay logic
- **Accordion** — needs expand/collapse animation
- **Carousel** — needs slide management
- **Calendar/DatePicker** — complex date logic

These may be added in future phases or as separate optional packages.

## Summary

After Phase 6, rwire will have a complete foundation for building production UIs:

| Category | Components |
|----------|------------|
| Layout | Stack, Card |
| Form | Button, Input, Textarea, Checkbox, Radio, Switch, Select, Label, FormField |
| Data | Badge, Table, Avatar, Progress, Spinner |
| Feedback | Alert |
| Navigation | Tabs, Breadcrumb, Pagination |
| **Total** | ~20 components |

Total CSS budget: **~10KB unminified, ~3KB gzipped**
