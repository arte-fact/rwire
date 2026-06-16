# CopyButton — Client Action Migration

**File**: `libs/rwire-components/src/copy_button.rs`
**Primitive**: Target
**Tier**: 3 (Medium Impact)
**Complexity**: Low

## Current Behavior

CopyButton uses a `data-copy` attribute and custom inline JS. The client runtime detects `data-copy` clicks, copies text to clipboard, adds a `.copied` CSS class to the button, and removes it after a timeout. The component includes two icon spans — one visible by default, one shown via custom CSS when `.copied` is present.

```rust
// Current — custom CSS + inline JS
el(El::Button)
    .data("copy", &self.text)
    .append([
        el(El::Span).class("copy-icon").append([icon(Icon::Clipboard)]),
        el(El::Span).class("copied-icon").st([St::DisplayNone]).append([icon(Icon::ClipboardCheck)]),
    ])

// Custom CSS required:
pub const COPY_BUTTON_CSS: &str =
    ".copied .copy-icon{display:none}.copied .copied-icon{display:inline-flex}";
```

**Issue**: Uses custom CSS class names (`.copy-icon`, `.copied-icon`, `.copied`) outside the St token system. The `COPY_BUTTON_CSS` const must be manually included in the capsule.

## Target State

```rust
// After migration — uses St tokens + Target instead of custom CSS classes
#[derive(Target)]
struct CopyFeedback;

el(El::Button)
    .data("copy", &self.text)
    .append([
        el(El::Span)
            .when::<CopyFeedback>(St::DisplayNone)       // hide when copied
            .append([icon(Icon::Clipboard)]),
        el(El::Span)
            .st([St::DisplayNone])
            .when::<CopyFeedback>(St::DisplayInlineFlex)  // show when copied
            .append([icon(Icon::ClipboardCheck)]),
    ])
```

## Implementation Challenges

### Timer-based reset

The copy feedback state needs to auto-reset after ~2 seconds (icon swaps back). Client actions don't support timed toggles.

Options:
1. **Keep the inline JS for the timer** — Target handles the icon swap, JS handles the timing
2. **Add TIMED_TOGGLE opcode** — new opcode that toggles a target for N ms, then resets
3. **Accept no auto-reset** — user clicks copy, icon stays as checkmark until next interaction

Recommendation: option 1 for now. The `data-copy` handler already exists in the JS runtime. Modify it to toggle the Target instead of adding a CSS class.

### Dependency on data-copy JS handler

The copy functionality itself (clipboard API) still needs JS. The migration only replaces the visual feedback mechanism, not the copy action.

## Migration Path

1. Replace `.class("copy-icon")` and `.class("copied-icon")` with Target bindings
2. Delete `COPY_BUTTON_CSS` const
3. Modify the `data-copy` JS handler to toggle a Target instead of adding `.copied` class
4. Keep the `setTimeout` in JS for auto-reset

## Testing

- `test_copy_button_no_custom_css` — no `.class()` calls on icon spans
- `test_copy_button_uses_target` — icon visibility bound to Target
- `test_copy_button_css_const_removed` — `COPY_BUTTON_CSS` deleted
