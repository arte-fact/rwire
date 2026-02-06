# 03 — API Analysis

Composition patterns, API ergonomics, and what to adopt or skip from popular design systems.

## Current rwire API Patterns

### Pattern 1: Fluent Builder
Every component follows the same shape:

```rust
Component::new()
    .variant(Variant::X)
    .size(Size::Md)
    .child(content)
    .build()
```

**Strengths**: Consistent, discoverable, IDE autocomplete-friendly.
**Weakness**: Verbose for simple cases — `Badge::success("OK").build()` vs JSX `<Badge color="green">OK</Badge>`.

### Pattern 2: Convenience Constructors
Shortcuts for the most common configurations:

```rust
Button::primary("Save")       // = Button::new().intent(Primary).text("Save")
Text::heading1("Welcome")     // = Text::new().variant(Heading1).text("Welcome")
Input::email()                 // = Input::new().input_type(Email)
Divider::horizontal()          // = Divider::new().direction(Horizontal)
```

**Assessment**: Good pattern. Should be extended to new components.

### Pattern 3: Named Sections
Complex components expose section builders:

```rust
Modal::new()
    .title("Confirm")         // header section
    .content(body_el)         // scrollable body
    .footer(footer_el)        // sticky bottom
    .build()
```

**Assessment**: Works well for Modal and FormField. The pattern should be extended to AppShell (`.header()`, `.sidebar()`, `.main()`).

### Pattern 4: Children via `.child()` / `.children()`
Layout components accept nested content:

```rust
Card::new().child(single_element).build()
Stack::column().children([el1, el2, el3]).build()
```

**Assessment**: Consistent. The `children` method accepts `IntoIterator`, making it composable with `.map()`.

### Pattern 5: Event Handler Shortcuts
Type-specific event methods:

```rust
Button::new().on_click(handler)    // Ev::Click
Input::new().on_input(handler)     // Ev::Input
Select::new().on_change(handler)   // Ev::Change
```

**Assessment**: Reduces boilerplate and prevents event type mismatches.

## Patterns from Other Libraries

### 1. Polymorphic Element Rendering

**Chakra UI / Mantine:**
```jsx
<Box as="section">content</Box>
<Stack as="nav">links</Stack>
```

**Proposed for rwire:**
```rust
Stack::column().element(El::Nav).children([...]).build()
Container::new().element(El::Main).child(content).build()
Card::new().element(El::Article).child(content).build()
```

**Analysis**: This is a high-value, low-cost addition. Currently rwire's Stack always renders as `<div>`. Adding `.element(El::Nav)` lets the same layout component produce semantic HTML. This matters for:
- Accessibility (screen readers use semantic elements)
- SEO (search engines weight semantic HTML)
- Docs site structure (needs `<nav>`, `<main>`, `<aside>`, `<article>`)

**Implementation cost**: Add an `element_type: Option<El>` field to Stack, Card, Container builders. In `.build()`, use the override or fall back to `El::Div`.

**Verdict: Adopt.** Simple change, significant benefit.

### 2. Section Builders (Named Slots)

**Mantine AppShell:**
```jsx
<AppShell>
  <AppShell.Header>...</AppShell.Header>
  <AppShell.Navbar>...</AppShell.Navbar>
  <AppShell.Main>...</AppShell.Main>
</AppShell>
```

**Proposed for rwire:**
```rust
AppShell::new()
    .header(header_content)
    .sidebar(sidebar_content)
    .main(main_content)
    .build()
```

**Analysis**: rwire already uses this pattern for Modal (`.title()`, `.content()`, `.footer()`). AppShell is the natural next candidate. Unlike JSX compound components (`AppShell.Header`), rwire's builder methods are simpler and don't require a separate type per section.

**Verdict: Adopt.** Consistent with existing Modal pattern.

### 3. Class Variance Authority (CVA)

**shadcn/ui:**
```typescript
const buttonVariants = cva("base-class", {
  variants: { intent: { primary: "bg-blue", secondary: "bg-gray" } },
  defaultVariants: { intent: "primary" }
})
```

**rwire equivalent (already exists):**
```rust
pub enum ButtonIntent { Primary, Secondary, Ghost, Destructive }
impl ButtonIntent {
    pub fn class(&self) -> &'static str {
        match self {
            Self::Primary => "",
            Self::Secondary => "rw-btn-secondary",
            Self::Ghost => "rw-btn-ghost",
            Self::Destructive => "rw-btn-destructive",
        }
    }
}
```

**Analysis**: rwire's enum-based variant system already accomplishes what CVA does in the JS world, with compile-time exhaustiveness checking. No need for an additional abstraction.

**Verdict: Skip.** Already covered by Rust enums.

### 4. Compound Components

**Radix UI / shadcn:**
```jsx
<Tabs.Root>
  <Tabs.List>
    <Tabs.Trigger>Tab 1</Tabs.Trigger>
  </Tabs.List>
  <Tabs.Content>Panel 1</Tabs.Content>
</Tabs.Root>
```

**Why it doesn't fit rwire**: Compound components rely on React context to share state between parent and children. In rwire's model:
- There's no JSX tree — ElementBuilder is the universal node type
- State flows through the builder pattern, not React context
- Named sections (`.header()`, `.content()`) achieve the same structural clarity

**Verdict: Skip.** Named section builders are rwire's equivalent.

### 5. Style Props

**Chakra UI:**
```jsx
<Box padding={4} marginTop={2} bg="gray.100" borderRadius="md" />
```

**Analysis**: Chakra puts CSS properties as component props. This creates a huge API surface and blurs the line between components and CSS utilities. In rwire:
- `St` tokens handle utility-style CSS (`St::Flex`, `St::Gap4`, etc.)
- `.class("custom")` is the escape hatch for one-off styling
- Design tokens are in CSS custom properties, not component props

**Verdict: Skip.** rwire's `St` tokens + `.class()` escape hatch is sufficient and more explicit.

### 6. `asChild` / Renderless Components

**Radix UI:**
```jsx
<Button asChild>
  <Link href="/home">Go Home</Link>
</Button>
```

**Analysis**: `asChild` merges props from parent onto child, allowing composition without wrapper elements. In rwire's binary protocol model:
- Elements are created and appended in a flat opcode stream
- There's no prop merging — each element gets its own attributes
- The server controls the entire DOM tree, so it can simply create the right element

**Verdict: Skip.** Not applicable to rwire's server-rendered architecture.

### 7. Unstyled/Headless Components

**Radix Primitives / Headless UI:**
Provide behavior (keyboard navigation, ARIA, focus management) without styling.

**Analysis**: In rwire, behavior is either:
- Server-side (event handlers) — keyboard events go through the binary protocol
- Client-side local handlers (BIND_LOCAL opcode) — small JS snippets for immediate feedback

Headless components would mean providing behavior patterns without CSS. This has some value (e.g., keyboard navigation for menus), but rwire's components are already lightweight enough that separating behavior from styling adds complexity without clear benefit.

**Verdict: Skip for now.** Revisit if component library grows beyond 50 components.

### 8. `.id()` Convenience Method

**Common across all libraries**: Every component supports an ID.

**Current rwire:**
```rust
el(El::Div).attr("id", "my-element")
```

**Proposed:**
```rust
el(El::Div).id("my-element")
```

**Analysis**: Trivial convenience that reduces noise. IDs are used for:
- ARIA relationships (`aria-labelledby`, `aria-describedby`)
- GET_BY_ID opcode targeting (server updating specific elements)
- CSS targeting (escape hatch)

**Verdict: Adopt.** One-line addition to ElementBuilder.

## Summary: Adopt vs Skip

### Adopt (3 patterns)

| Pattern | Source | Effort | Impact |
|---------|--------|--------|--------|
| Polymorphic `.element()` | Chakra `as` prop | Low — add field + match | High — semantic HTML everywhere |
| Section builders for AppShell | Mantine AppShell | Medium — new component | High — page layout scaffolding |
| `.id()` on ElementBuilder | Universal | Trivial — one method | Medium — reduces boilerplate |

### Skip (5 patterns)

| Pattern | Source | Reason |
|---------|--------|--------|
| CVA | shadcn/ui | Rust enums already provide this |
| Compound components | Radix | Named sections are rwire's equivalent |
| Style props | Chakra | St tokens + .class() covers this |
| `asChild` / renderless | Radix | Not applicable to binary protocol |
| Headless components | Radix Primitives | Premature for current library size |

## API Ergonomics Improvements

Beyond adopting patterns, some ergonomic improvements for new components:

### 1. Consistent Size Scale
All sizable components should use the same enum:
```rust
pub enum ComponentSize { Sm, Md, Lg }
```
Currently Button, Input, Textarea, Spinner, Avatar, ThemeToggle each define their own size enum. A shared enum would reduce cognitive overhead.

### 2. Consistent Intent Scale
Components with semantic intents should share:
```rust
pub enum Intent { Default, Primary, Success, Warning, Error }
```
Currently Badge has 5 intents (Default, Primary, Success, Warning, Error) while Button has 4 (Primary, Secondary, Ghost, Destructive). These serve different purposes but the naming could be more consistent.

### 3. Builder Return Type
All `.build()` methods return `ElementBuilder`, which makes components composable as children of other components:
```rust
Stack::column().children([
    Text::heading1("Title").build(),     // ElementBuilder
    Card::new().child(content).build(),  // ElementBuilder
    Button::primary("Save").build(),     // ElementBuilder
]).build()
```
This is a significant advantage over JSX-based systems where component types must match.
