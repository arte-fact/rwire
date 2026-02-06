# 01 — Current Inventory

Complete audit of rwire's component library: 28 components across 8 categories, totaling ~12.5KB CSS with full tree-shaking support.

## Summary Table

| # | Component | Category | CSS Budget | Actual CSS | Key Features |
|---|-----------|----------|------------|------------|--------------|
| 1 | Button | Buttons | 1,536B | ~1,262B | 4 intents, 3 sizes, loading/disabled states |
| 2 | Input | Form | 1,000B | ~860B | 7 types, 3 sizes, validation states |
| 3 | Textarea | Form | 1,000B | ~894B | 3 sizes, auto-rows |
| 4 | Select | Form | 850B | ~786B | Custom dropdown arrow, validation |
| 5 | Checkbox | Form | 650B | ~591B | Auto-ID for label association |
| 6 | Radio | Form | 650B | ~598B | Auto-ID, group name binding |
| 7 | Switch | Form | 800B | ~697B | Toggle slider, role="switch" |
| 8 | FormField | Form | 400B | ~362B | Label + input + help/error sections |
| 9 | Label | Form | 250B | ~195B | Required asterisk |
| 10 | Stack | Layout | 600B | ~547B | Row/column, gap, align, justify, wrap |
| 11 | Card | Layout | 450B | ~354B | Padding, shadow, border options |
| 12 | Container | Layout | 350B | ~243B | 5 max-width sizes, centered/flush |
| 13 | Divider | Layout | 500B | ~321B | Horizontal/vertical, margin sizes |
| 14 | Spacer | Layout | 500B | ~359B | 6 sizes, horizontal/vertical |
| 15 | Text | Typography | 800B | ~738B | 7 variants (h1-h3, body, caption), 7 colors |
| 16 | Link | Typography | 300B | ~204B | Internal/external, auto-target |
| 17 | List | Typography | 250B | ~194B | Ordered/unordered, custom items |
| 18 | Badge | Display | 600B | ~538B | 5 intents |
| 19 | Progress | Display | 300B | ~231B | Value/max, optional label |
| 20 | Spinner | Display | 400B | ~324B | 3 sizes, keyframe animation |
| 21 | Avatar | Display | 550B | ~458B | Image with fallback initials |
| 22 | Alert | Display | 650B | ~573B | 4 intents, title + message |
| 23 | Table | Display | 500B | ~458B | Headers, rows, striped option |
| 24 | Breadcrumb | Navigation | 500B | ~440B | Items with optional links |
| 25 | Tabs | Navigation | 650B | ~546B | Tab items with panels |
| 26 | Pagination | Navigation | 700B | ~617B | Prev/next, ellipsis, max visible |
| 27 | Modal | Overlay | 2,000B | ~1,580B | 5 sizes, title/content/footer sections |
| 28 | ThemeToggle | Buttons | 800B | ~673B | Light/dark mode, 3 sizes |
| — | Utils | Utility | — | ~350B | Z-index, sr-only, hidden, portal |

**Total CSS (all components): ~12,743B (~12.4KB)**

## Component Details

### Buttons (2 components)

#### Button
The most feature-rich component. Supports four intent variants, three sizes, and states for disabled and loading.

```rust
Button::primary("Save").build()
Button::secondary("Cancel").size(ButtonSize::Sm).build()
Button::new().intent(ButtonIntent::Ghost).disabled(true).build()
Button::new().intent(ButtonIntent::Destructive).on_click(handler).build()
```

**Intents**: Primary (filled), Secondary (outlined), Ghost (transparent), Destructive (red)
**Sizes**: Sm (28px), Md (36px default), Lg (44px)
**States**: `.rw-btn-disabled`, `.rw-btn-loading`, `.rw-btn-full`

#### ThemeToggle
Specialized toggle button for light/dark mode switching. Uses the icon system for sun/moon icons.

```rust
ThemeToggle::new().build()
ThemeToggle::new().size(ToggleSize::Lg).mode(ThemeToggleMode::Dark).on_toggle(handler).build()
```

### Form (8 components)

The form components share consistent patterns:
- **Sizes**: Sm/Md/Lg with matching height (28px/36px/44px)
- **Validation**: `.invalid(true)` adds visual error state
- **Events**: `.on_input(handler)`, `.on_change(handler)` convenience methods
- **Auto-ID**: Checkbox, Radio, Switch auto-generate IDs for `<label for="">` binding

#### Input
Seven HTML input types with type-safe constructors:

```rust
Input::text().placeholder("Name").name("username").build()
Input::password().required(true).build()
Input::email().invalid(true).build()
Input::number().build()
Input::search().build()
```

#### Textarea
Multi-line text input with configurable rows (default 4):

```rust
Textarea::new().placeholder("Description").rows(6).build()
Textarea::new().size(TextareaSize::Lg).required(true).build()
```

#### Select
Native `<select>` with custom dropdown arrow via CSS data URI:

```rust
Select::new()
    .option("us", "United States")
    .option("ca", "Canada")
    .value("us")
    .build()
```

#### Checkbox, Radio, Switch
All three follow the same pattern: auto-wrap in a Stack with label when `.label()` is provided.

```rust
Checkbox::new().label("Subscribe").checked(true).build()
Radio::new().name("plan").value("pro").label("Pro Plan").build()
Switch::new().label("Dark mode").on_change(handler).build()
```

#### FormField
Composition wrapper that assembles label + input + help/error text:

```rust
FormField::new()
    .label("Email")
    .input(Input::email().name("email").build())
    .help("We'll never share your email")
    .error("Invalid email address")
    .required(true)
    .build()
```

#### Label
Simple label with optional required indicator:

```rust
Label::new("Password").required(true).build()
```

### Layout (5 components)

#### Stack
The primary layout primitive. Flexbox-based with all common flex properties:

```rust
Stack::column().gap(Gap::Lg).children([...]).build()
Stack::row().gap(Gap::Sm).justify(StackJustify::Between).build()
Stack::centered().build()  // Centers both axes
```

**Gaps**: None, Xs (4px), Sm (8px), Md (16px), Lg (24px), Xl (32px)
**Align**: Stretch, Start, Center, End
**Justify**: Start, Center, End, Between, Around

#### Card
Content container with padding, shadow, and border options:

```rust
Card::new().padding(CardPadding::Lg).shadow(CardShadow::Md).build()
Card::new().child(content).children([child1, child2]).build()
```

#### Container
Max-width wrapper for page content:

```rust
Container::new().size(ContainerSize::Lg).child(content).build()
```

**Sizes**: Sm (640px), Md (768px), Lg (1024px), Xl (1280px), Full

#### Divider
Horizontal or vertical rule with configurable margin:

```rust
Divider::horizontal().margin(SpacingSize::Lg).build()
Divider::vertical().build()
```

#### Spacer
Fixed-size spacing element:

```rust
Spacer::md().build()
Spacer::lg().horizontal().build()
```

### Typography (3 components)

#### Text
Semantic text with automatic HTML element selection:

```rust
Text::heading1("Welcome").build()      // renders <h1>
Text::heading2("Section").build()       // renders <h2>
Text::body("Content").build()           // renders <p>
Text::caption("Help").muted().build()   // renders <span> with muted color
```

**Variants**: Heading1, Heading2, Heading3, Body, BodySmall, Label, Caption
**Colors**: Default, High, Muted, Accent, Success, Warning, Error

#### Link
Navigation with external link detection:

```rust
Link::new("/about").text("About").build()
Link::external("https://example.com").text("Docs").build()
```

External links automatically add `target="_blank" rel="noopener noreferrer"` and an arrow icon via CSS `::after`.

#### List
Ordered and unordered lists:

```rust
List::unordered().children([
    ListItem::new("First").build(),
    ListItem::new("Second").build(),
]).build()
```

### Display (5 components)

#### Badge
Small status labels with intent-based colors:

```rust
Badge::success("Active").build()
Badge::error("Failed").build()
```

#### Alert
Prominent notification blocks:

```rust
Alert::info().title("Note").message("Changes saved").build()
Alert::error().title("Error").message("Something went wrong").build()
```

#### Progress
Simple progress bar:

```rust
Progress::new().value(65).max(100).label("65%").build()
```

#### Spinner
Loading indicator with CSS animation:

```rust
Spinner::new().size(SpinnerSize::Lg).label("Loading...").build()
```

#### Avatar
User avatar with image or fallback initials:

```rust
Avatar::new().src("/avatar.jpg").alt("John").build()
Avatar::new().fallback("JD").size(AvatarSize::Lg).build()
```

#### Table
Data table with headers and rows:

```rust
Table::new()
    .headers(["Name", "Email"])
    .row(TableRow::new().cells(["Alice", "alice@co.com"]))
    .striped(true)
    .build()
```

### Navigation (3 components)

#### Breadcrumb
Path breadcrumbs with separator:

```rust
Breadcrumb::new()
    .item("Home", Some("/"))
    .item("Docs", Some("/docs"))
    .item("API", None)  // current page
    .build()
```

#### Tabs
Tab switcher with panels:

```rust
Tabs::new()
    .tab(Tab::new("Overview", content1))
    .tab(Tab::new("API", content2))
    .active(0)
    .build()
```

#### Pagination
Page navigation with ellipsis:

```rust
Pagination::new().current_page(3).total_pages(10).build()
```

### Overlay (1 component)

#### Modal
The most complex component. Uses named section builders:

```rust
Modal::new()
    .title("Confirm")
    .open(true)
    .on_close(handler)
    .content(el(El::P).text("Are you sure?"))
    .footer(Stack::row().children([cancel_btn, confirm_btn]).build())
    .size(ModalSize::Lg)
    .close_on_backdrop_click(true)
    .build()
```

**Sizes**: Sm (400px), Md (600px), Lg (800px), Xl (1000px), Full (100%)

## Cross-Cutting Patterns

### 1. Fluent Builder API
Every component uses the builder pattern: `Component::new().option(val).build()`.

### 2. CSS Tree-Shaking
All `.build()` methods call `mark_component_used(ComponentType)`. Only used component CSS is included in the capsule.

### 3. Type-Safe Variants
Enums for intents, sizes, and states — no stringly-typed APIs:
```rust
Button::new().intent(ButtonIntent::Primary).size(ButtonSize::Lg)
```

### 4. Convenience Constructors
Common configurations have shortcuts:
```rust
Button::primary("Save")    // intent=Primary, text="Save"
Text::heading1("Title")    // variant=Heading1, text="Title"
Input::email()             // type=Email
```

### 5. Event Handler Shortcuts
Components expose typed event methods:
```rust
Button::new().on_click(handler)    // vs .on(Ev::Click, handler)
Input::new().on_input(handler)     // vs .on(Ev::Input, handler)
```

### 6. Named Sections
Complex components use section builders instead of positional children:
- Modal: `.title()`, `.content()`, `.footer()`
- FormField: `.label()`, `.input()`, `.help()`, `.error()`
- Alert: `.title()`, `.message()`

### 7. Z-Index Scale
Centralized z-index constants in `utils.rs`:
```
Dropdown: 1000  →  Sticky: 1100  →  Fixed: 1200
Modal Backdrop: 1300  →  Modal: 1400  →  Popover: 1500
Tooltip: 1600  →  Toast: 1700
```

## Protocol Coverage

### Element Types (29 defined)
Container: `div`, `span`, `section`, `article`, `nav`, `header`, `footer`
Typography: `p`, `h1`, `h2`, `h3`
Form: `button`, `input`, `textarea`, `select`, `option`, `label`, `fieldset`, `legend`, `form`
Lists: `ul`, `li`, `ol`
Navigation: `a`, `hr`
Graphics: `svg`, `path`

### Event Types (12 defined)
Mouse: `click`, `dblclick`, `mousedown`, `mouseup`, `mousemove`
Form: `submit`, `input`, `change`
Keyboard: `keydown`, `keyup`
Focus: `focus`, `blur`

### Missing for Docs Site
Elements: `pre`, `code`, `blockquote`, `strong`, `em`, `img`, `table`, `thead`, `tbody`, `tr`, `th`, `td`, `aside`, `main`
Events: `scroll` (for table of contents scroll-spy)
