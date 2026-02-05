# rwire Component Implementation Plan

**Goal**: Implement 40 new components, redesign the design system showcase, and add full dark/light mode theming.

---

## 📋 Executive Summary

This plan implements a complete component library for rwire with:
- **40 new components** across 4 priority tiers
- **Dark/light mode theming** (already exists, needs integration)
- **Professional design system showcase** with live examples
- **Compact CSS** (~250 bytes per component target)
- **Progressive enhancement** (works without JS where possible)
- **Server-side rendering** (leveraging rwire's architecture)

**Estimated Total Effort**: 5-7 days (160-200 components @ 20-30 min each + infrastructure + showcase redesign)

---

## 🎯 Phase 1: Foundation & Infrastructure (Day 1, ~4 hours)

### 1.1 Theme System Enhancement
**Goal**: Make dark/light mode work seamlessly in the design system.

**Tasks**:
- [x] Theme system already exists in `rwire/src/theme.rs`
- [ ] Add theme state to design system (storage: local)
- [ ] Create `ThemeToggle` component with sun/moon icons
- [ ] Add theme switcher to design system header
- [ ] Test theme persistence across page reloads

**Files to modify**:
- `examples/design-system/src/main.rs` - Add theme state
- `rwire/src/components/` - New `theme_toggle.rs`

**Implementation notes**:
```rust
#[derive(State, Default)]
#[storage(local)]
struct ThemeState {
    is_dark: bool,
}

// ThemeToggle component updates local state
// On toggle, update data-theme attribute on :root
```

### 1.2 Icon System
**Goal**: Minimal inline SVG icons for components (no external dependencies).

**Tasks**:
- [ ] Create `rwire/src/icons.rs` module
- [ ] Define core icons as `&'static str` SVG constants:
  - `ICON_CLOSE` (×)
  - `ICON_CHECK` (✓)
  - `ICON_CHEVRON_DOWN` (▼)
  - `ICON_CHEVRON_UP` (▲)
  - `ICON_CHEVRON_LEFT` (◀)
  - `ICON_CHEVRON_RIGHT` (▶)
  - `ICON_SUN` (☀)
  - `ICON_MOON` (🌙)
  - `ICON_INFO` (i)
  - `ICON_SUCCESS` (✓ in circle)
  - `ICON_WARNING` (!)
  - `ICON_ERROR` (✗)
  - `ICON_SEARCH` (🔍)
  - `ICON_UPLOAD` (⬆)
  - `ICON_MENU` (☰)
- [ ] Helper function: `icon(svg: &str, size: IconSize) -> ElementBuilder`

**Size budget**: ~2KB total for all icons (minified inline SVG)

### 1.3 Component Infrastructure
**Goal**: Shared utilities for complex components.

**Tasks**:
- [ ] Create `rwire/src/components/utils.rs`:
  - `generate_id()` - Unique component IDs
  - `z_index()` - Z-index constants for layers
  - `aria_attrs()` - ARIA attribute helpers
- [ ] Add overlay/backdrop shared styles
- [ ] Add animation utilities (fade, slide)

**Z-index layers**:
```rust
pub const Z_DROPDOWN: u32 = 1000;
pub const Z_STICKY: u32 = 1100;
pub const Z_FIXED: u32 = 1200;
pub const Z_MODAL_BACKDROP: u32 = 1300;
pub const Z_MODAL: u32 = 1400;
pub const Z_POPOVER: u32 = 1500;
pub const Z_TOOLTIP: u32 = 1600;
pub const Z_TOAST: u32 = 1700;
```

---

## 🔴 Phase 2: Critical Components (Day 2-3, ~16 hours)

### Priority 1 - Modal/Dialog
**Size target**: 350 bytes CSS
**Complexity**: High (overlay, focus trap, keyboard handling)

**Features**:
- Backdrop with blur
- Close button (X)
- Keyboard shortcuts (Esc to close)
- Sizes: sm (400px), md (600px), lg (800px), fullscreen
- Optional header, footer slots

**API**:
```rust
Modal::new()
    .size(ModalSize::Md)
    .title("Confirm Action")
    .body(el(El::P).text("Are you sure?"))
    .footer(Stack::row()
        .gap(Gap::Sm)
        .children([
            Button::ghost("Cancel").on_click(close_modal()),
            Button::destructive("Delete").on_click(confirm_delete()),
        ])
    )
    .build()
```

**Server-side state**:
```rust
struct ModalState {
    is_open: bool,
    modal_id: Option<String>,
}
```

### Priority 2 - Toast/Notification
**Size target**: 300 bytes CSS
**Complexity**: Medium (positioning, auto-dismiss, stacking)

**Features**:
- Position: top-right, bottom-right, top-center, bottom-center
- Intent: success, error, warning, info
- Auto-dismiss after timeout (configurable)
- Stack multiple toasts
- Close button

**API**:
```rust
Toast::success("Changes saved!")
    .duration(3000)
    .position(ToastPosition::TopRight)
    .build()

// Server manages toast queue
struct ToastState {
    toasts: Vec<ToastItem>,
}
```

### Priority 3 - Dropdown Menu
**Size target**: 280 bytes CSS
**Complexity**: Medium (positioning, keyboard nav)

**Features**:
- Triggered by button/element
- Menu items, dividers, submenus
- Keyboard navigation (arrow keys, enter, esc)
- Icons per item
- Disabled items

**API**:
```rust
DropdownMenu::new()
    .trigger(Button::ghost("Actions"))
    .items([
        MenuItem::new("Edit").icon(ICON_EDIT).on_click(edit()),
        MenuItem::new("Duplicate").on_click(duplicate()),
        MenuDivider::new(),
        MenuItem::new("Delete").intent(MenuIntent::Destructive).on_click(delete()),
    ])
    .build()
```

### Priority 4 - Tooltip
**Size target**: 200 bytes CSS
**Complexity**: Low-Medium (positioning, delay)

**Features**:
- Position: top, bottom, left, right, auto
- Arrow pointer
- Delay before showing (default 400ms)
- Works on hover and focus
- Max width

**API**:
```rust
Tooltip::new("Click to save")
    .position(TooltipPosition::Top)
    .child(Button::primary("Save"))
    .build()
```

### Priority 5 - Drawer/Sheet
**Size target**: 300 bytes CSS
**Complexity**: High (slide animation, overlay)

**Features**:
- Position: left, right, top, bottom
- Sizes: sm, md, lg, full
- Overlay backdrop
- Close button
- Slide animation

**API**:
```rust
Drawer::right()
    .size(DrawerSize::Md)
    .title("Filters")
    .body(/* filter form */)
    .build()
```

### Priority 6 - Combobox
**Size target**: 320 bytes CSS
**Complexity**: High (search, filtering, keyboard)

**Features**:
- Filter as you type
- Keyboard navigation
- Multi-select variant
- Empty state
- Loading state

**API**:
```rust
Combobox::new()
    .placeholder("Select country...")
    .options(countries)
    .searchable(true)
    .on_select(handle_select())
    .build()
```

### Priority 7 - Accordion
**Size target**: 250 bytes CSS
**Complexity**: Medium (animation, state)

**Features**:
- Single or multiple expansion
- Animated expand/collapse
- Icon indicator (chevron)
- Disabled items

**API**:
```rust
Accordion::new()
    .single() // or .multiple()
    .items([
        AccordionItem::new("Section 1")
            .content(el(El::P).text("Content 1")),
        AccordionItem::new("Section 2")
            .content(el(El::P).text("Content 2")),
    ])
    .build()
```

### Priority 8 - Skeleton
**Size target**: 150 bytes CSS
**Complexity**: Low (animation only)

**Features**:
- Animated pulse/shimmer
- Variants: text, avatar, card
- Custom dimensions

**API**:
```rust
Skeleton::text().lines(3).build()
Skeleton::avatar().size(AvatarSize::Lg).build()
Skeleton::card().height("200px").build()
```

### Priority 9 - Popover
**Size target**: 280 bytes CSS
**Complexity**: Medium (positioning)

**Features**:
- Position: top, bottom, left, right, auto
- Can contain interactive content
- Click or hover trigger
- Arrow pointer

**API**:
```rust
Popover::new()
    .trigger(Button::ghost("Help"))
    .content(
        Stack::column()
            .gap(Gap::Sm)
            .children([
                el(El::H4).text("Help"),
                el(El::P).text("Need assistance?"),
            ])
    )
    .build()
```

### Priority 10 - Slider
**Size target**: 300 bytes CSS
**Complexity**: High (input handling, positioning)

**Features**:
- Single or dual handles (range)
- Step increments
- Labels and ticks
- Disabled state

**API**:
```rust
Slider::new()
    .min(0)
    .max(100)
    .step(5)
    .value(50)
    .on_change(handle_change())
    .build()

Slider::range()
    .min(0)
    .max(1000)
    .value_range(200, 800)
    .on_change(handle_range_change())
    .build()
```

**Testing checklist for Phase 2**:
- [ ] All 10 components render correctly
- [ ] Dark/light mode works for all
- [ ] Keyboard navigation works
- [ ] ARIA attributes present
- [ ] Mobile responsive
- [ ] CSS under budget

---

## 🟡 Phase 3: Important Components (Day 4, ~12 hours)

### Components 11-20:

**11. File Upload** (400 bytes CSS)
- Drag & drop zone
- File list with preview
- Progress indicator
- Validation

**12. Chip/Tag** (180 bytes CSS)
- Removable labels
- Color variants
- Close button
- Compact size

**13. Empty State** (200 bytes CSS)
- Icon, title, description
- Optional action button
- Illustration slot

**14. Search** (250 bytes CSS)
- Search icon
- Clear button
- Autocomplete dropdown
- Keyboard shortcut (/)

**15. Date Picker** (500 bytes CSS)
- Calendar grid
- Month/year navigation
- Range selection
- Disabled dates

**16. Time Picker** (300 bytes CSS)
- Hour/minute selection
- 12/24 hour format
- Keyboard input

**17. Context Menu** (280 bytes CSS)
- Right-click trigger
- Menu items with icons
- Submenus

**18. Command Palette** (400 bytes CSS)
- Cmd+K trigger
- Fuzzy search
- Categories
- Keyboard navigation

**19. Stepper** (220 bytes CSS)
- Step indicator
- Active/completed states
- Optional labels

**20. Tree View** (320 bytes CSS)
- Expand/collapse nodes
- Selection
- Indentation
- Icons

---

## 🟢 Phase 4: Nice-to-Have Components (Day 5, ~8 hours)

### Components 21-30:

**21. Collapsible** (180 bytes CSS)
**22. Separator** (100 bytes CSS)
**23. Rating** (200 bytes CSS)
**24. Toggle Group** (220 bytes CSS)
**25. Color Picker** (450 bytes CSS)
**26. Navigation Menu** (350 bytes CSS)
**27. Carousel** (400 bytes CSS)
**28. Timeline** (280 bytes CSS)
**29. Stats Card** (200 bytes CSS)
**30. Code Block** (300 bytes CSS)

**Implementation strategy**: Batch similar components together (e.g., all simple toggles, all navigation components).

---

## 🔵 Phase 5: Advanced Components (Day 6, ~6 hours)

### Components 31-40:

**31. Split Pane** (350 bytes CSS)
**32. Virtual Scroller** (400 bytes CSS + JS optimization)
**33. Menu Bar** (280 bytes CSS)
**34. Toolbar** (200 bytes CSS)
**35. Aspect Ratio** (120 bytes CSS)
**36. Scroll Area** (250 bytes CSS)
**37. Image** (200 bytes CSS)
**38. Link** (150 bytes CSS)
**39. Icon** (100 bytes CSS)
**40. Kbd** (150 bytes CSS)

---

## 🎨 Phase 6: Design System Redesign (Day 7, ~8 hours)

### 6.1 New Landing/Hero Section
**Goal**: Professional first impression.

**Features**:
- Hero section with gradient background
- Feature highlights (3-4 key benefits)
- Quick start code example
- "View Components" CTA button

**Layout**:
```
┌─────────────────────────────────────┐
│  Header [Logo] [Theme Toggle]      │
├─────────────────────────────────────┤
│         HERO SECTION                │
│   "Build UIs with rwire"            │
│   [Key Features]                    │
│   [Code Example]                    │
│   [View Components Button]          │
├─────────────────────────────────────┤
```

### 6.2 Navigation Overhaul
**Goal**: Easy component discovery.

**Features**:
- Sticky sidebar navigation
- Component categories:
  - Form Inputs
  - Data Display
  - Feedback & Status
  - Overlays
  - Navigation
  - Layout
  - Advanced
- Search component showcase
- Active section highlighting

### 6.3 Component Showcase Pages
**Goal**: Each component has a dedicated page with examples.

**Structure per component**:
1. **Overview** - What it does
2. **Basic Example** - Minimal usage
3. **Variants** - All available styles/states
4. **Props/API** - Builder methods
5. **Code Example** - Copy-pasteable Rust code
6. **Accessibility** - ARIA attributes, keyboard support
7. **Dark Mode** - Screenshot comparison

**Example page structure**:
```rust
fn render_button_page(state: &DesignSystemState) -> ElementBuilder {
    Stack::column()
        .gap(Gap::Xl)
        .children([
            // Header
            Stack::column()
                .gap(Gap::Sm)
                .children([
                    el(El::H1).text("Button"),
                    el(El::P)
                        .class("text-muted")
                        .text("Trigger actions and events with buttons."),
                ]),
            
            // Basic Example
            ComponentDemo::new("Basic Usage")
                .example(Button::primary("Click me").build())
                .code(r#"Button::primary("Click me").build()"#),
            
            // Variants
            ComponentDemo::new("Variants")
                .example(Stack::row()
                    .gap(Gap::Sm)
                    .children([
                        Button::primary("Primary").build(),
                        Button::secondary("Secondary").build(),
                        Button::ghost("Ghost").build(),
                        Button::destructive("Destructive").build(),
                    ])
                )
                .code(r#"Button::primary("Primary").build()
Button::secondary("Secondary").build()
Button::ghost("Ghost").build()
Button::destructive("Destructive").build()"#),
            
            // Sizes
            ComponentDemo::new("Sizes")
                .example(/* ... */),
            
            // States
            ComponentDemo::new("States")
                .example(/* disabled, loading */),
        ])
}
```

### 6.4 Theme Showcase
**Goal**: Show off theming capabilities.

**Features**:
- Theme preview cards (light/dark/custom accent)
- Live theme switcher with instant preview
- Accent color selector (blue, red, green, amber)
- Radius selector (none, small, medium, large, full)
- "Copy theme code" button

### 6.5 Search Functionality
**Goal**: Quick component discovery.

**Features**:
- Keyboard shortcut (/)
- Fuzzy search across component names
- Search by category
- Recent searches

### 6.6 Responsive Design
**Goal**: Mobile-friendly showcase.

**Breakpoints**:
- Mobile: < 768px (hamburger menu)
- Tablet: 768px - 1024px (collapsible sidebar)
- Desktop: > 1024px (fixed sidebar)

### 6.7 Performance Optimizations
**Goal**: Fast, efficient showcase.

**Optimizations**:
- Tree-shaken CSS (only used components)
- Lazy-loaded component examples (synced regions)
- Minimal JavaScript runtime
- Optimized symbols/opcodes

---

## 📐 Implementation Guidelines

### CSS Budget Rules
- **Simple components** (Badge, Separator): < 150 bytes
- **Medium components** (Button, Input): 150-250 bytes
- **Complex components** (Modal, Dropdown): 250-400 bytes
- **Very complex** (Date Picker, Virtual Scroller): 400-600 bytes
- **Total library target**: < 12KB (80 components × 150 bytes avg)

### CSS Optimization Techniques
```css
/* Bad - verbose */
.rw-button {
    padding-top: 8px;
    padding-bottom: 8px;
    padding-left: 16px;
    padding-right: 16px;
}

/* Good - compact */
.rw-btn{padding:8px 16px}

/* Best - design tokens */
.rw-btn{padding:var(--rw-space-2) var(--rw-space-4)}
```

### Component Naming Convention
- CSS class: `rw-{component}` (e.g., `rw-modal`)
- Modifiers: `rw-{component}-{modifier}` (e.g., `rw-modal-lg`)
- States: `rw-{component}-{state}` (e.g., `rw-btn-disabled`)

### ARIA Compliance
Every component must have:
- Proper semantic HTML where possible
- ARIA roles for custom components
- Keyboard navigation support
- Focus management
- Screen reader announcements

### Testing Checklist (per component)
- [ ] Renders in light mode
- [ ] Renders in dark mode
- [ ] All variants work
- [ ] Keyboard navigation works
- [ ] Screen reader accessible
- [ ] Mobile responsive
- [ ] CSS under budget
- [ ] No clippy warnings
- [ ] Unit tests pass
- [ ] Example in showcase works

---

## 🗂️ File Structure

```
rwire/
├── rwire/src/
│   ├── components/
│   │   ├── mod.rs (updated exports)
│   │   ├── utils.rs (NEW - shared utilities)
│   │   ├── theme_toggle.rs (NEW)
│   │   ├── modal.rs (NEW)
│   │   ├── toast.rs (NEW)
│   │   ├── dropdown.rs (NEW)
│   │   ├── tooltip.rs (NEW)
│   │   ├── drawer.rs (NEW)
│   │   ├── combobox.rs (NEW)
│   │   ├── accordion.rs (NEW)
│   │   ├── skeleton.rs (NEW)
│   │   ├── popover.rs (NEW)
│   │   ├── slider.rs (NEW)
│   │   ├── ... (30 more)
│   │   └── registry.rs (updated)
│   ├── icons.rs (NEW)
│   └── theme.rs (already exists)
├── examples/design-system/
│   ├── src/
│   │   ├── main.rs (redesigned)
│   │   ├── pages/ (NEW)
│   │   │   ├── mod.rs
│   │   │   ├── home.rs
│   │   │   ├── button.rs
│   │   │   ├── input.rs
│   │   │   └── ... (one per component)
│   │   ├── components/ (NEW)
│   │   │   ├── mod.rs
│   │   │   ├── component_demo.rs
│   │   │   ├── code_block.rs
│   │   │   └── nav_sidebar.rs
│   │   └── state.rs (NEW - extracted state)
│   └── README.md (updated)
└── COMPONENT_IMPLEMENTATION_PLAN.md (this file)
```

---

## 🚀 Execution Strategy

### Parallel Work Streams
1. **Stream A**: Components 1-20 (critical + important)
2. **Stream B**: Infrastructure (icons, utils, theme)
3. **Stream C**: Design system redesign

### Daily Goals
- **Day 1**: Infrastructure complete, theme toggle working
- **Day 2**: Components 1-5 complete
- **Day 3**: Components 6-10 complete
- **Day 4**: Components 11-20 complete
- **Day 5**: Components 21-30 complete
- **Day 6**: Components 31-40 complete
- **Day 7**: Design system redesign complete

### Success Metrics
- [ ] All 40 components implemented
- [ ] All components have dark mode support
- [ ] Total component CSS < 12KB
- [ ] Zero clippy warnings
- [ ] All tests pass
- [ ] Design system looks professional
- [ ] Mobile responsive
- [ ] Keyboard accessible

---

## 🎯 Immediate Next Steps

**Start here** (in order):

1. **Create icon system** (`rwire/src/icons.rs`)
   - Define 15 core SVG icons as constants
   - Add `icon()` helper function
   
2. **Create component utils** (`rwire/src/components/utils.rs`)
   - Z-index constants
   - ID generation
   - ARIA helpers

3. **Add theme state to design system**
   - Add `ThemeState` to main.rs
   - Create theme toggle handler

4. **Implement ThemeToggle component**
   - First new component
   - Tests the infrastructure
   - Enables dark mode showcase

5. **Implement Modal component**
   - Most complex component
   - Sets patterns for others
   - Critical for many UIs

6. **Continue with remaining critical components** (2-10)

7. **Start design system redesign**
   - New hero section
   - Navigation sidebar
   - Component showcase pages

---

## 📊 Progress Tracking

### Phase Completion
- [ ] Phase 1: Foundation (0/3 tasks)
- [ ] Phase 2: Critical Components (0/10 components)
- [ ] Phase 3: Important Components (0/10 components)
- [ ] Phase 4: Nice-to-Have (0/10 components)
- [ ] Phase 5: Advanced (0/10 components)
- [ ] Phase 6: Design System Redesign (0/7 tasks)

### Component Checklist
Use this to track individual component completion:

```markdown
- [ ] Modal/Dialog
- [ ] Toast/Notification
- [ ] Dropdown Menu
- [ ] Tooltip
- [ ] Drawer/Sheet
- [ ] Combobox
- [ ] Accordion
- [ ] Skeleton
- [ ] Popover
- [ ] Slider
- [ ] File Upload
- [ ] Chip/Tag
- [ ] Empty State
- [ ] Search
- [ ] Date Picker
- [ ] Time Picker
- [ ] Context Menu
- [ ] Command Palette
- [ ] Stepper
- [ ] Tree View
- [ ] Collapsible
- [ ] Separator
- [ ] Rating
- [ ] Toggle Group
- [ ] Color Picker
- [ ] Navigation Menu
- [ ] Carousel
- [ ] Timeline
- [ ] Stats Card
- [ ] Code Block
- [ ] Split Pane
- [ ] Virtual Scroller
- [ ] Menu Bar
- [ ] Toolbar
- [ ] Aspect Ratio
- [ ] Scroll Area
- [ ] Image
- [ ] Link
- [ ] Icon
- [ ] Kbd
```

---

## 🤔 Design Decisions & Trade-offs

### Server-Side vs Client-Side
**Decision**: Keep all state on server, minimal client JS.
**Rationale**: Aligns with rwire philosophy, simpler state management.
**Trade-off**: Some animations/interactions need server round-trip.

### CSS Size vs Features
**Decision**: Strict CSS budget per component.
**Rationale**: Keep capsule small, fast loading.
**Trade-off**: May need to compromise on fancy animations.

### Accessibility vs Complexity
**Decision**: Full ARIA support, keyboard nav for all interactive components.
**Rationale**: Accessibility is non-negotiable.
**Trade-off**: More complex implementation.

### Component API Design
**Decision**: Builder pattern with sensible defaults.
**Rationale**: Ergonomic, type-safe, self-documenting.
**Example**:
```rust
// Good - clear, chainable
Modal::new()
    .title("Delete Item")
    .size(ModalSize::Md)
    .build()

// Bad - unclear, verbose
Modal {
    title: Some("Delete Item".to_string()),
    size: ModalSize::Md,
    is_open: true,
    ..Default::default()
}
```

---

## 📚 References

### Inspiration (Component Libraries)
- **Radix UI** - Unstyled primitives, excellent accessibility
- **shadcn/ui** - Beautiful defaults, dark mode
- **Chakra UI** - Comprehensive component set
- **Material UI** - Mature, battle-tested
- **Ant Design** - Enterprise-grade

### CSS Resources
- **Open Props** - Design token system
- **Pico CSS** - Minimal framework
- **Tailwind** - Utility patterns

### Accessibility
- **WAI-ARIA Authoring Practices** - Component patterns
- **A11y Project** - Accessibility checklist
- **WebAIM** - Testing resources

---

## ✅ Definition of Done

A component is complete when:

1. **Implementation**
   - [ ] Component file created with full implementation
   - [ ] Builder API implemented with all variants
   - [ ] CSS written and under budget
   - [ ] Exported from `components/mod.rs`
   - [ ] Added to `generate_components_css()`

2. **Theming**
   - [ ] Works in light mode
   - [ ] Works in dark mode
   - [ ] Uses design tokens (no hardcoded colors)
   - [ ] Respects accent color
   - [ ] Respects radius scale

3. **Accessibility**
   - [ ] Proper semantic HTML
   - [ ] ARIA attributes present
   - [ ] Keyboard navigation works
   - [ ] Focus management correct
   - [ ] Screen reader tested

4. **Testing**
   - [ ] Unit tests written
   - [ ] All tests pass
   - [ ] Zero clippy warnings
   - [ ] Manual testing done

5. **Documentation**
   - [ ] Doc comments on struct and methods
   - [ ] Usage example in doc comments
   - [ ] Added to design system showcase
   - [ ] Screenshot taken (light + dark)

6. **Integration**
   - [ ] Works with existing components
   - [ ] No CSS conflicts
   - [ ] Symbols tree-shaken correctly
   - [ ] Performance acceptable

The design system redesign is complete when:
- [ ] All 40 components have showcase pages
- [ ] Theme toggle works perfectly
- [ ] Navigation is intuitive
- [ ] Mobile responsive
- [ ] Hero section looks professional
- [ ] Search functionality works
- [ ] Zero visual bugs
- [ ] Load time < 2 seconds
- [ ] Looks better than popular UI libraries

---

**Last Updated**: 2024-02-04
**Status**: Planning Complete - Ready for Implementation
**Next Action**: Create icon system (`rwire/src/icons.rs`)