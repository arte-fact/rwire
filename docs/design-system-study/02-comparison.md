# 02 — Comparison with Popular Design Systems

Side-by-side comparison of rwire's component library with shadcn/ui, Chakra UI, Mantine, and Ant Design.

## Library Profiles

| | rwire | shadcn/ui | Chakra UI | Mantine | Ant Design |
|---|---|---|---|---|---|
| **Ecosystem** | Rust/WebSocket | React/Tailwind | React/Emotion | React/CSS-in-JS | React/Less |
| **Components** | 28 | ~50 | ~60 | ~100+ | ~70 |
| **Rendering** | Server binary protocol | Client-side React | Client-side React | Client-side React | Client-side React |
| **Styling** | CSS classes (tree-shaken) | Tailwind utilities | Emotion runtime | CSS modules / inline | Less/CSS variables |
| **Bundle cost** | ~12.5KB CSS (no JS per component) | ~0KB (copy-paste) | ~50-80KB JS | ~200KB+ JS+CSS | ~300KB+ JS+CSS |
| **Theming** | CSS custom properties | Tailwind theme | Theme provider | MantineProvider | ConfigProvider |
| **Tree-shaking** | CSS-level (per component) | N/A (copy-paste) | JS-level | JS-level | JS-level |
| **Philosophy** | Server owns state | Own your code | Accessible-first | Everything included | Enterprise-complete |

## Component Coverage by Category

### Layout

| Component | rwire | shadcn | Chakra | Mantine | Notes |
|-----------|:-----:|:------:|:------:|:-------:|-------|
| Stack/VStack/HStack | ✅ | — | ✅ | ✅ | rwire Stack unifies row+column |
| Card | ✅ | ✅ | ✅ | ✅ | |
| Container | ✅ | — | ✅ | ✅ | |
| Divider/Separator | ✅ | ✅ | ✅ | ✅ | |
| Spacer | ✅ | — | ✅ | ✅ | |
| Grid | — | — | ✅ | ✅ | **Gap**: CSS Grid layout |
| Box | — | — | ✅ | ✅ | Generic styled div |
| AspectRatio | — | — | ✅ | ✅ | CSS `aspect-ratio` wrapper |
| Center | — | — | ✅ | ✅ | rwire has `Stack::centered()` |
| Flex | — | — | ✅ | ✅ | rwire's Stack serves this role |
| AppShell | — | — | — | ✅ | **Gap**: Page layout scaffolding |
| **Subtotal** | **5** | **2** | **8** | **10** | |

**Assessment**: rwire's Stack is versatile enough to cover Flex and Center use cases. The significant gaps are Grid (for multi-column layouts), AppShell (page scaffolding), and Box (generic styled element). AspectRatio can be handled with a St token.

### Typography

| Component | rwire | shadcn | Chakra | Mantine | Notes |
|-----------|:-----:|:------:|:------:|:-------:|-------|
| Text/Heading | ✅ | ✅ | ✅ | ✅ | rwire unifies both |
| Link | ✅ | — | ✅ | ✅ | |
| Label | ✅ | ✅ | — | — | |
| List | ✅ | — | ✅ | ✅ | |
| Code/CodeBlock | — | — | ✅ | ✅ | **Gap**: Inline + block code |
| Highlight | — | — | ✅ | ✅ | Highlighted text span |
| Kbd | — | — | ✅ | ✅ | Keyboard shortcut display |
| Blockquote | — | — | — | ✅ | **Gap**: Quote styling |
| Title | — | — | — | ✅ | rwire has Text::heading* |
| TypographyStylesProvider | — | — | — | ✅ | **Gap**: Prose/markdown styles |
| **Subtotal** | **4** | **2** | **5** | **8** | |

**Assessment**: Critical gaps for a docs site are Code/CodeBlock, Blockquote, and a Prose/TypographyStylesProvider for rendering markdown content. Kbd and Highlight are nice-to-have.

### Form Controls

| Component | rwire | shadcn | Chakra | Mantine | Notes |
|-----------|:-----:|:------:|:------:|:-------:|-------|
| Input | ✅ | ✅ | ✅ | ✅ | |
| Textarea | ✅ | ✅ | ✅ | ✅ | |
| Select | ✅ | ✅ | ✅ | ✅ | |
| Checkbox | ✅ | ✅ | ✅ | ✅ | |
| Radio | ✅ | ✅ | ✅ | ✅ | |
| Switch | ✅ | ✅ | ✅ | ✅ | |
| FormField | ✅ | ✅ | ✅ | — | Mantine uses Input.Wrapper |
| Label | ✅ | ✅ | ✅ | — | |
| Slider | — | ✅ | ✅ | ✅ | Range input |
| Autocomplete | — | ✅ | — | ✅ | Search + suggestions |
| ColorInput | — | ✅ | — | ✅ | Color picker |
| DatePicker | — | ✅ | — | ✅ | Complex widget |
| FileInput | — | — | — | ✅ | File upload |
| NumberInput | — | — | ✅ | ✅ | rwire has Input::number() |
| PinInput | — | ✅ | ✅ | ✅ | OTP/verification codes |
| SegmentedControl | — | — | — | ✅ | Button-group toggle |
| **Subtotal** | **8** | **10** | **9** | **13** | |

**Assessment**: rwire has strong form coverage for basic use cases. Slider and Autocomplete are the most broadly useful gaps. DatePicker and ColorInput are complex widgets unlikely to fit rwire's CSS budget philosophy.

### Buttons & Actions

| Component | rwire | shadcn | Chakra | Mantine | Notes |
|-----------|:-----:|:------:|:------:|:-------:|-------|
| Button | ✅ | ✅ | ✅ | ✅ | |
| ThemeToggle | ✅ | ✅ | — | — | Mode switcher |
| IconButton | — | — | ✅ | ✅ | Square button with icon only |
| CopyButton | — | — | — | ✅ | Click-to-copy |
| ButtonGroup | — | — | ✅ | ✅ | Grouped buttons |
| CloseButton | — | — | ✅ | ✅ | ✕ dismiss button |
| **Subtotal** | **2** | **2** | **3** | **4** | |

**Assessment**: IconButton could be a Button variant rather than a separate component. CopyButton is interesting for a docs site (code examples).

### Data Display

| Component | rwire | shadcn | Chakra | Mantine | Notes |
|-----------|:-----:|:------:|:------:|:-------:|-------|
| Badge | ✅ | ✅ | ✅ | ✅ | |
| Progress | ✅ | ✅ | ✅ | ✅ | |
| Spinner | ✅ | — | ✅ | ✅ | shadcn uses Skeleton instead |
| Avatar | ✅ | ✅ | ✅ | ✅ | |
| Alert | ✅ | ✅ | ✅ | ✅ | |
| Table | ✅ | ✅ | ✅ | ✅ | |
| Skeleton | — | ✅ | ✅ | ✅ | **Gap**: Loading placeholder |
| Tag/Chip | — | — | ✅ | ✅ | Removable label |
| Stat | — | — | ✅ | — | Metric display |
| Timeline | — | — | — | ✅ | Vertical timeline |
| Accordion | — | ✅ | ✅ | ✅ | **Gap**: Collapsible sections |
| Image | — | — | ✅ | ✅ | Image with fallback |
| **Subtotal** | **6** | **6** | **9** | **10** | |

**Assessment**: Skeleton and Accordion are high-priority gaps. Skeleton is essential for perceived performance; Accordion is needed for docs site sidebar navigation.

### Overlay & Popover

| Component | rwire | shadcn | Chakra | Mantine | Notes |
|-----------|:-----:|:------:|:------:|:-------:|-------|
| Modal/Dialog | ✅ | ✅ | ✅ | ✅ | |
| Tooltip | — | ✅ | ✅ | ✅ | **Gap**: Hover information |
| Drawer | — | ✅ | ✅ | ✅ | **Gap**: Slide-in panel |
| Popover | — | ✅ | ✅ | ✅ | Floating content |
| DropdownMenu | — | ✅ | ✅ | ✅ | **Gap**: Action menu |
| HoverCard | — | ✅ | — | ✅ | Rich hover preview |
| AlertDialog | — | ✅ | ✅ | — | Confirmation dialog |
| ContextMenu | — | ✅ | — | — | Right-click menu |
| Sheet | — | ✅ | — | — | Bottom/side sheet |
| **Subtotal** | **1** | **7** | **5** | **5** | |

**Assessment**: This is rwire's biggest category gap. Tooltip, Drawer, and DropdownMenu are the highest-priority additions. All three require positioning logic — in rwire's model, the server can set CSS classes for position, and the client's existing portal system (`.rw-portal`) can handle DOM placement.

### Navigation

| Component | rwire | shadcn | Chakra | Mantine | Notes |
|-----------|:-----:|:------:|:------:|:-------:|-------|
| Breadcrumb | ✅ | ✅ | ✅ | ✅ | |
| Tabs | ✅ | ✅ | ✅ | ✅ | |
| Pagination | ✅ | ✅ | — | ✅ | |
| NavLink/Sidebar | — | ✅ | — | ✅ | **Gap**: Sidebar navigation |
| Accordion | — | ✅ | ✅ | ✅ | **Gap**: Collapsible sections |
| NavigationMenu | — | ✅ | — | — | Top-level nav bar |
| Stepper | — | — | ✅ | ✅ | Multi-step progress |
| Anchor | — | — | — | ✅ | Heading anchor links |
| **Subtotal** | **3** | **5** | **3** | **6** | |

**Assessment**: NavLink/Sidebar and Accordion are needed for the docs site. Stepper is useful but not urgent.

### Feedback

| Component | rwire | shadcn | Chakra | Mantine | Notes |
|-----------|:-----:|:------:|:------:|:-------:|-------|
| Toast/Notification | — | ✅ | ✅ | ✅ | **Gap**: Transient messages |
| LoadingOverlay | — | — | — | ✅ | Full-screen loader |
| Sonner | — | ✅ | — | — | Stacked toast system |
| **Subtotal** | **0** | **2** | **1** | **2** | |

**Assessment**: Toast is the key gap. In rwire's server model, a toast is a server-pushed temporary element with a CSS animation — a natural fit.

## Aggregate Comparison

| Category | rwire | shadcn/ui | Chakra UI | Mantine | rwire Gap Count |
|----------|:-----:|:---------:|:---------:|:-------:|:----:|
| Layout | 5 | 2 | 8 | 10 | 2 critical (Grid, AppShell) |
| Typography | 4 | 2 | 5 | 8 | 3 critical (Code, Blockquote, Prose) |
| Form | 8 | 10 | 9 | 13 | 1 moderate (Slider) |
| Buttons | 2 | 2 | 3 | 4 | 0 (IconButton = variant) |
| Display | 6 | 6 | 9 | 10 | 2 high (Skeleton, Accordion) |
| Overlay | 1 | 7 | 5 | 5 | 3 high (Tooltip, Drawer, Dropdown) |
| Navigation | 3 | 5 | 3 | 6 | 2 high (Sidebar, Accordion) |
| Feedback | 0 | 2 | 1 | 2 | 1 high (Toast) |
| **Total** | **29** | **36** | **43** | **58** | |

## Key Takeaways

### rwire's Strengths
1. **Zero JS cost per component** — no component adds client-side JavaScript, only CSS
2. **Tight CSS budgets** — 12.5KB total vs hundreds of KB for JS-based libraries
3. **Excellent tree-shaking** — CSS is per-component, only used components are shipped
4. **Type-safe API** — Rust enums for variants, no stringly-typed props
5. **Consistent builder pattern** — every component follows the same `.new().option().build()` shape
6. **Server-side state** — simpler mental model, no client/server state sync issues

### rwire's Gaps
1. **Overlay components** — only Modal exists; Tooltip, Drawer, and DropdownMenu are all missing
2. **Typography for content** — no Code, Blockquote, or Prose for rendering markdown/docs
3. **Layout scaffolding** — no AppShell for full-page structure with header/sidebar/content
4. **Feedback** — no Toast or notification system
5. **Loading states** — Spinner exists but Skeleton is missing

### Comparison Libraries' Tradeoffs
- **shadcn/ui**: Copy-paste model means zero dependency but no automatic updates. Good API inspiration.
- **Chakra UI**: Excellent accessibility defaults. Style props on every component add API surface but hurt bundle size.
- **Mantine**: Most complete library. Many components rwire doesn't need (DatePicker, RichTextEditor, Carousel). Good reference for which components are truly useful.
- **Ant Design**: Enterprise-focused. Heavy. Most components are overkill for rwire's lightweight philosophy.

### What rwire Should Learn From Each

| Library | Adopt | Skip |
|---------|-------|------|
| shadcn/ui | Accordion collapsible pattern, Sheet/Drawer positioning | CVA (rwire has enums), copy-paste model |
| Chakra UI | Semantic element polymorphism (`as` prop → `.element()`) | Style props everywhere, theme tokens on every component |
| Mantine | AppShell layout pattern, TypographyStylesProvider for markdown | 100+ component scope, compound components |
| Ant Design | Table column definitions, form validation patterns | Enterprise complexity, ant-motion animations |
