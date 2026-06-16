# AppShell — Client Action Migration

**File**: `libs/rwire-components/src/app_shell.rs`
**Primitive**: Target
**Tier**: 3 (Medium Impact)
**Complexity**: Low

## Current Behavior

AppShell is a static layout component. It renders a CSS grid with header, optional sidebar, and main content area. The sidebar is either present or absent at build time — there is no runtime toggle.

```rust
AppShell::new()
    .header(header_content)
    .sidebar(sidebar_content)
    .main(main_content)
    .build()
```

The sidebar width is set via inline style (`grid-template-columns:{width}px 1fr`). There is no collapse/expand mechanism.

## Opportunity

Mobile-responsive apps commonly need a collapsible sidebar — visible on desktop, hidden on mobile with a hamburger toggle. AppShell currently has no support for this. Client actions would enable:

```rust
#[derive(Target)]
struct SidebarOpen;

AppShell::new()
    .header(
        el(El::Div).append([
            el(El::Button)
                .text("☰")
                .toggle::<SidebarOpen>(Ev::Click)
                .md([St::DisplayNone]),  // hide hamburger on desktop
            el(El::Span).text("My App"),
        ])
    )
    .sidebar(sidebar_content)
    .sidebar_toggle::<SidebarOpen>()
    .main(main_content)
    .build()
```

On mobile: sidebar hidden by default, hamburger toggles it as an overlay. On desktop: sidebar always visible via breakpoint, hamburger hidden.

## Implementation

### 1. Add sidebar_toggle method

```rust
impl AppShell {
    pub fn sidebar_toggle<T: Target>(mut self) -> Self {
        self.sidebar_target_type_id = Some(TypeId::of::<T>());
        self
    }
}
```

### 2. Sidebar visibility bound to Target + breakpoint

```rust
// In build():
if let Some(target_type_id) = self.sidebar_target_type_id {
    sidebar_el = sidebar_el
        .st([St::DisplayNone])                          // hidden by default (mobile)
        .when_by_id(target_type_id, St::DisplayBlock)   // shown when toggled
        .md([St::DisplayBlock]);                         // always shown on desktop
}
```

### 3. Grid template adjustment

When sidebar is togglable, the grid template needs to accommodate both states:
- Sidebar hidden: `grid-template-columns: 1fr`
- Sidebar visible: `grid-template-columns: {width}px 1fr`

This is tricky with pure CSS class toggling. Options:
- Use `position: fixed` overlay for mobile sidebar (like Drawer) instead of grid column
- Use CSS `display:none` on sidebar which collapses the grid column automatically with `grid-template-columns: auto 1fr`

Recommendation: On mobile, render sidebar as a fixed overlay (like Drawer) controlled by Target. On desktop (md+ breakpoint), render inline in the grid. This matches common responsive patterns.

## Tokens Needed

May need new St tokens:
- `St::GridColCollapsed` — `grid-template-columns: 1fr` (or handle via sidebar display:none)

## Testing

- `test_app_shell_sidebar_toggle` — target_type_id set
- `test_app_shell_sidebar_hidden_mobile` — sidebar has DisplayNone by default when toggle set
- `test_app_shell_no_toggle_unchanged` — without sidebar_toggle, works as before
