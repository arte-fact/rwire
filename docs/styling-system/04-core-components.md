# Phase 4: Core Components

## Goal

Implement essential layout and form components following the variant system pattern.

## Components in This Phase

| Component | Purpose | Variants |
|-----------|---------|----------|
| Stack | Flexbox layout | direction, gap, align |
| Input | Text input | size, state |
| Card | Surface container | padding, shadow |
| Badge | Status indicator | intent, size |

## rwire Philosophy Alignment

| Principle | How This Phase Aligns |
|-----------|----------------------|
| Zero runtime | All layout computed server-side |
| Minimal bandwidth | Layout classes are single tokens in symbol table |
| Minimal capsule | Components share base utilities |

## Implementation

### Step 4.1: Stack Component

**File: `rwire/src/components/stack.rs`**

```rust
//! Stack layout component.
//!
//! Flexbox-based layout with configurable direction and spacing.

use crate::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Stack direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StackDirection {
    #[default]
    Column,
    Row,
}

/// Stack alignment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StackAlign {
    #[default]
    Stretch,
    Start,
    Center,
    End,
}

/// Stack justify.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StackJustify {
    #[default]
    Start,
    Center,
    End,
    Between,
    Around,
}

/// Gap size using spacing tokens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Gap {
    None,
    Xs,  // space-1
    Sm,  // space-2
    #[default]
    Md,  // space-4
    Lg,  // space-6
    Xl,  // space-8
}

impl Gap {
    fn class_suffix(&self) -> Option<&'static str> {
        match self {
            Gap::None => Some("gap-0"),
            Gap::Xs => Some("gap-xs"),
            Gap::Sm => Some("gap-sm"),
            Gap::Md => None, // Default
            Gap::Lg => Some("gap-lg"),
            Gap::Xl => Some("gap-xl"),
        }
    }
}

/// Stack layout builder.
#[derive(Clone, Debug, Default)]
pub struct Stack {
    direction: StackDirection,
    gap: Gap,
    align: StackAlign,
    justify: StackJustify,
    wrap: bool,
    extra_class: Option<Cow<'static, str>>,
    children: Vec<ElementBuilder>,
}

impl Stack {
    pub fn new() -> Self {
        Self::default()
    }

    /// Horizontal stack (row direction).
    pub fn row() -> Self {
        Self::new().direction(StackDirection::Row)
    }

    /// Vertical stack (column direction).
    pub fn column() -> Self {
        Self::new().direction(StackDirection::Column)
    }

    pub fn direction(mut self, direction: StackDirection) -> Self {
        self.direction = direction;
        self
    }

    pub fn gap(mut self, gap: Gap) -> Self {
        self.gap = gap;
        self
    }

    pub fn align(mut self, align: StackAlign) -> Self {
        self.align = align;
        self
    }

    pub fn justify(mut self, justify: StackJustify) -> Self {
        self.justify = justify;
        self
    }

    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Add children to the stack.
    pub fn children(mut self, children: impl IntoIterator<Item = ElementBuilder>) -> Self {
        self.children.extend(children);
        self
    }

    /// Add a single child.
    pub fn child(mut self, child: ElementBuilder) -> Self {
        self.children.push(child);
        self
    }

    fn compute_class(&self) -> String {
        let mut classes = vec!["rw-stack"];

        if self.direction == StackDirection::Row {
            classes.push("rw-stack-row");
        }

        if let Some(suffix) = self.gap.class_suffix() {
            classes.push(match suffix {
                "gap-0" => "rw-gap-0",
                "gap-xs" => "rw-gap-xs",
                "gap-sm" => "rw-gap-sm",
                "gap-lg" => "rw-gap-lg",
                "gap-xl" => "rw-gap-xl",
                _ => unreachable!(),
            });
        }

        match self.align {
            StackAlign::Stretch => {}
            StackAlign::Start => classes.push("rw-items-start"),
            StackAlign::Center => classes.push("rw-items-center"),
            StackAlign::End => classes.push("rw-items-end"),
        }

        match self.justify {
            StackJustify::Start => {}
            StackJustify::Center => classes.push("rw-justify-center"),
            StackJustify::End => classes.push("rw-justify-end"),
            StackJustify::Between => classes.push("rw-justify-between"),
            StackJustify::Around => classes.push("rw-justify-around"),
        }

        if self.wrap {
            classes.push("rw-flex-wrap");
        }

        if let Some(ref extra) = self.extra_class {
            let mut result = classes.join(" ");
            result.push(' ');
            result.push_str(extra);
            return result;
        }

        classes.join(" ")
    }

    pub fn build(self) -> ElementBuilder {
        let class = self.compute_class();
        let mut builder = el(El::Div).class(&class);

        for child in self.children {
            builder = builder.append([child]);
        }

        builder
    }
}
```

**File: `rwire/src/components/stack_css.rs`**

```rust
pub fn stack_css() -> &'static str {
    r#"/* Stack layout */
.rw-stack{display:flex;flex-direction:column;gap:var(--rw-space-4)}
.rw-stack-row{flex-direction:row}
.rw-gap-0{gap:0}.rw-gap-xs{gap:var(--rw-space-1)}.rw-gap-sm{gap:var(--rw-space-2)}.rw-gap-lg{gap:var(--rw-space-6)}.rw-gap-xl{gap:var(--rw-space-8)}
.rw-items-start{align-items:flex-start}.rw-items-center{align-items:center}.rw-items-end{align-items:flex-end}
.rw-justify-center{justify-content:center}.rw-justify-end{justify-content:flex-end}.rw-justify-between{justify-content:space-between}.rw-justify-around{justify-content:space-around}
.rw-flex-wrap{flex-wrap:wrap}
"#
}
```

### Step 4.2: Input Component

**File: `rwire/src/components/input.rs`**

```rust
//! Input component.

use crate::{el, El, Ev, ElementBuilder};
use std::borrow::Cow;

/// Input type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputType {
    #[default]
    Text,
    Password,
    Email,
    Number,
    Search,
    Tel,
    Url,
}

impl InputType {
    fn as_str(&self) -> &'static str {
        match self {
            InputType::Text => "text",
            InputType::Password => "password",
            InputType::Email => "email",
            InputType::Number => "number",
            InputType::Search => "search",
            InputType::Tel => "tel",
            InputType::Url => "url",
        }
    }
}

/// Input size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputSize {
    Sm,
    #[default]
    Md,
    Lg,
}

/// Input builder.
#[derive(Clone, Debug, Default)]
pub struct Input {
    input_type: InputType,
    size: InputSize,
    placeholder: Option<Cow<'static, str>>,
    value: Option<Cow<'static, str>>,
    name: Option<Cow<'static, str>>,
    id: Option<Cow<'static, str>>,
    disabled: bool,
    readonly: bool,
    required: bool,
    invalid: bool,
    extra_class: Option<Cow<'static, str>>,
}

impl Input {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text() -> Self {
        Self::new()
    }

    pub fn password() -> Self {
        Self::new().input_type(InputType::Password)
    }

    pub fn email() -> Self {
        Self::new().input_type(InputType::Email)
    }

    pub fn number() -> Self {
        Self::new().input_type(InputType::Number)
    }

    pub fn input_type(mut self, input_type: InputType) -> Self {
        self.input_type = input_type;
        self
    }

    pub fn size(mut self, size: InputSize) -> Self {
        self.size = size;
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<Cow<'static, str>>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn value(mut self, value: impl Into<Cow<'static, str>>) -> Self {
        self.value = Some(value.into());
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

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    fn compute_class(&self) -> String {
        let mut classes = vec!["rw-input"];

        match self.size {
            InputSize::Sm => classes.push("rw-input-sm"),
            InputSize::Md => {}
            InputSize::Lg => classes.push("rw-input-lg"),
        }

        if self.invalid {
            classes.push("rw-input-invalid");
        }

        if let Some(ref extra) = self.extra_class {
            let mut result = classes.join(" ");
            result.push(' ');
            result.push_str(extra);
            return result;
        }

        classes.join(" ")
    }

    pub fn build(self) -> ElementBuilder {
        let class = self.compute_class();
        let mut builder = el(El::Input)
            .class(&class)
            .attr("type", self.input_type.as_str());

        if let Some(placeholder) = self.placeholder {
            builder = builder.attr("placeholder", &placeholder);
        }
        if let Some(value) = self.value {
            builder = builder.attr("value", &value);
        }
        if let Some(name) = self.name {
            builder = builder.attr("name", &name);
        }
        if let Some(id) = &self.id {
            builder = builder.id(id);
        }
        if self.disabled {
            builder = builder.attr("disabled", "");
        }
        if self.readonly {
            builder = builder.attr("readonly", "");
        }
        if self.required {
            builder = builder.attr("required", "");
        }
        if self.invalid {
            builder = builder.attr("aria-invalid", "true");
        }

        builder
    }

    /// Build with input handler (debounced by default).
    pub fn on_input(self, handler: impl Into<crate::HandlerFn>) -> ElementBuilder {
        self.build().on_debounced(Ev::Input, handler.into(), 150)
    }
}
```

**File: `rwire/src/components/input_css.rs`**

```rust
pub fn input_css() -> &'static str {
    r#"/* Input */
.rw-input{display:block;width:100%;height:2.25rem;padding:0 var(--rw-space-3);font-size:var(--rw-text-sm);line-height:var(--rw-leading-normal);color:var(--rw-text-high);background:var(--rw-bg-app);border:1px solid var(--rw-border-default);border-radius:var(--rw-radius-md);transition:border-color .15s,box-shadow .15s}
.rw-input::placeholder{color:var(--rw-text-muted)}
.rw-input:hover{border-color:var(--rw-border-emphasis)}
.rw-input:focus{outline:none;border-color:var(--rw-accent-8);box-shadow:0 0 0 3px var(--rw-accent-4)}
.rw-input:disabled{opacity:.5;cursor:not-allowed;background:var(--rw-bg-muted)}
.rw-input-sm{height:1.75rem;padding:0 var(--rw-space-2);font-size:var(--rw-text-xs)}
.rw-input-lg{height:2.75rem;padding:0 var(--rw-space-4);font-size:var(--rw-text-base)}
.rw-input-invalid{border-color:var(--rw-red-8)}
.rw-input-invalid:focus{border-color:var(--rw-red-8);box-shadow:0 0 0 3px var(--rw-red-4)}
"#
}
```

### Step 4.3: Card Component

**File: `rwire/src/components/card.rs`**

```rust
//! Card component.

use crate::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Card padding.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CardPadding {
    None,
    Sm,
    #[default]
    Md,
    Lg,
}

/// Card shadow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CardShadow {
    None,
    #[default]
    Sm,
    Md,
    Lg,
}

/// Card builder.
#[derive(Clone, Debug, Default)]
pub struct Card {
    padding: CardPadding,
    shadow: CardShadow,
    bordered: bool,
    extra_class: Option<Cow<'static, str>>,
    children: Vec<ElementBuilder>,
}

impl Card {
    pub fn new() -> Self {
        Self {
            bordered: true,
            ..Self::default()
        }
    }

    pub fn padding(mut self, padding: CardPadding) -> Self {
        self.padding = padding;
        self
    }

    pub fn shadow(mut self, shadow: CardShadow) -> Self {
        self.shadow = shadow;
        self
    }

    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = ElementBuilder>) -> Self {
        self.children.extend(children);
        self
    }

    pub fn child(mut self, child: ElementBuilder) -> Self {
        self.children.push(child);
        self
    }

    fn compute_class(&self) -> String {
        let mut classes = vec!["rw-card"];

        match self.padding {
            CardPadding::None => classes.push("rw-p-0"),
            CardPadding::Sm => classes.push("rw-p-sm"),
            CardPadding::Md => {}
            CardPadding::Lg => classes.push("rw-p-lg"),
        }

        match self.shadow {
            CardShadow::None => classes.push("rw-shadow-none"),
            CardShadow::Sm => {}
            CardShadow::Md => classes.push("rw-shadow-md"),
            CardShadow::Lg => classes.push("rw-shadow-lg"),
        }

        if !self.bordered {
            classes.push("rw-border-none");
        }

        if let Some(ref extra) = self.extra_class {
            let mut result = classes.join(" ");
            result.push(' ');
            result.push_str(extra);
            return result;
        }

        classes.join(" ")
    }

    pub fn build(self) -> ElementBuilder {
        let class = self.compute_class();
        let mut builder = el(El::Div).class(&class);

        for child in self.children {
            builder = builder.append([child]);
        }

        builder
    }
}
```

**File: `rwire/src/components/card_css.rs`**

```rust
pub fn card_css() -> &'static str {
    r#"/* Card */
.rw-card{background:var(--rw-bg-app);border:1px solid var(--rw-border-subtle);border-radius:var(--rw-radius-lg);padding:var(--rw-space-4);box-shadow:var(--rw-shadow-sm)}
.rw-p-0{padding:0}.rw-p-sm{padding:var(--rw-space-2)}.rw-p-lg{padding:var(--rw-space-6)}
.rw-shadow-none{box-shadow:none}.rw-shadow-md{box-shadow:var(--rw-shadow-md)}.rw-shadow-lg{box-shadow:var(--rw-shadow-lg)}
.rw-border-none{border:none}
"#
}
```

### Step 4.4: Badge Component

**File: `rwire/src/components/badge.rs`**

```rust
//! Badge component.

use crate::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Badge intent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BadgeIntent {
    #[default]
    Default,
    Primary,
    Success,
    Warning,
    Error,
}

/// Badge builder.
#[derive(Clone, Debug, Default)]
pub struct Badge {
    intent: BadgeIntent,
    text: Option<Cow<'static, str>>,
    extra_class: Option<Cow<'static, str>>,
}

impl Badge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(mut self, text: impl Into<Cow<'static, str>>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn intent(mut self, intent: BadgeIntent) -> Self {
        self.intent = intent;
        self
    }

    pub fn primary(text: impl Into<Cow<'static, str>>) -> Self {
        Self::new().intent(BadgeIntent::Primary).text(text)
    }

    pub fn success(text: impl Into<Cow<'static, str>>) -> Self {
        Self::new().intent(BadgeIntent::Success).text(text)
    }

    pub fn warning(text: impl Into<Cow<'static, str>>) -> Self {
        Self::new().intent(BadgeIntent::Warning).text(text)
    }

    pub fn error(text: impl Into<Cow<'static, str>>) -> Self {
        Self::new().intent(BadgeIntent::Error).text(text)
    }

    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    fn compute_class(&self) -> String {
        let mut classes = vec!["rw-badge"];

        match self.intent {
            BadgeIntent::Default => {}
            BadgeIntent::Primary => classes.push("rw-badge-primary"),
            BadgeIntent::Success => classes.push("rw-badge-success"),
            BadgeIntent::Warning => classes.push("rw-badge-warning"),
            BadgeIntent::Error => classes.push("rw-badge-error"),
        }

        if let Some(ref extra) = self.extra_class {
            let mut result = classes.join(" ");
            result.push(' ');
            result.push_str(extra);
            return result;
        }

        classes.join(" ")
    }

    pub fn build(self) -> ElementBuilder {
        let class = self.compute_class();
        let mut builder = el(El::Span).class(&class);

        if let Some(text) = self.text {
            builder = builder.text(&text);
        }

        builder
    }
}
```

**File: `rwire/src/components/badge_css.rs`**

```rust
pub fn badge_css() -> &'static str {
    r#"/* Badge */
.rw-badge{display:inline-flex;align-items:center;padding:0 var(--rw-space-2);height:1.25rem;font-size:var(--rw-text-xs);font-weight:var(--rw-font-medium);border-radius:var(--rw-radius-full);background:var(--rw-bg-emphasis);color:var(--rw-text-high)}
.rw-badge-primary{background:var(--rw-accent-4);color:var(--rw-accent-11)}
.rw-badge-success{background:var(--rw-green-4);color:var(--rw-green-11)}
.rw-badge-warning{background:var(--rw-amber-4);color:var(--rw-amber-11)}
.rw-badge-error{background:var(--rw-red-4);color:var(--rw-red-11)}
"#
}
```

## Deliverables

- [ ] `rwire/src/components/stack.rs` + CSS
- [ ] `rwire/src/components/input.rs` + CSS
- [ ] `rwire/src/components/card.rs` + CSS
- [ ] `rwire/src/components/badge.rs` + CSS
- [ ] Update `rwire/src/components/mod.rs` exports

## Size Budget

| Component | Max CSS Size |
|-----------|--------------|
| Stack | 400 bytes |
| Input | 700 bytes |
| Card | 300 bytes |
| Badge | 350 bytes |
| **Total Phase 4** | < 1.8KB |

## Usage Examples

```rust
use rwire::components::*;

// Layout
Stack::column()
    .gap(Gap::Lg)
    .children([
        Card::new()
            .padding(CardPadding::Lg)
            .child(
                Stack::row()
                    .justify(StackJustify::Between)
                    .children([
                        Input::text()
                            .placeholder("Search...")
                            .build(),
                        Button::primary("Search").build(),
                    ])
            )
            .build(),

        // Status badges
        Stack::row()
            .gap(Gap::Sm)
            .children([
                Badge::success("Active").build(),
                Badge::warning("Pending").build(),
            ])
            .build(),
    ])
    .build()
```

## Next Phase

[Phase 5: CSS Integration](./05-css-integration.md) — Capsule injection and tree-shaking.
