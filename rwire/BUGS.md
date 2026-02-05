# rwire Bug Tracker

This document tracks known bugs, their status, and reproduction steps.

## 🔴 Critical Bugs

### BUG-001: State Changes Don't Trigger Re-renders in design-system Example
**Status:** Open  
**Severity:** High  
**Component:** State Management / Reactivity  
**Discovered:** 2024-02-04

**Description:**  
Form component state changes (checkbox, radio, switch) don't trigger UI updates. The DOM reflects the state change (e.g., checkbox becomes checked), but rendered text depending on that state (e.g., "Newsletter: No" → "Newsletter: Yes") doesn't update.

**Root Cause:**  
The `render_forms_section()`, `render_data_display_section()`, and `render_feedback_section()` functions in `examples/design-system/src/main.rs` are NOT marked with `#[renderer]`. Without this macro, state changes don't trigger re-renders of dependent UI.

**Reproduction:**
1. Run `cargo run -p design-system`
2. Navigate to http://127.0.0.1:9000
3. Click the "Subscribe to newsletter" checkbox
4. Observe: Checkbox becomes checked (DOM updated)
5. Bug: Text "Newsletter: No" doesn't change to "Newsletter: Yes"

**Server Logs:**
```
[127.0.0.1:43786] Event: handler=0x03 type=change target_ref=45
```
Server receives the event but doesn't send update back because no renderer is subscribed to `checkbox_checked` field.

**Expected Behavior:**  
When `state.checkbox_checked` changes, all UI elements that reference this field should automatically re-render.

**Fix Required:**
- Add `#[renderer]` macro to `render_forms_section(state: &DesignSystemState)`
- Add `#[renderer]` macro to `render_data_display_section(state: &DesignSystemState)`
- Add `#[renderer]` macro to `render_feedback_section(state: &DesignSystemState)`

**Impact:**  
All interactive examples in design-system that depend on state changes are broken.

---

### BUG-002: InvalidCharacterError on SET_ATTR During Section Navigation
**Status:** Open  
**Severity:** Critical  
**Component:** Protocol / JavaScript Runtime  
**Discovered:** 2024-02-04

**Description:**  
When clicking "Data Display" button to switch sections, a JavaScript error occurs:
```
InvalidCharacterError: Failed to execute 'setAttribute' on 'Element': 
'rw-btn rw-btn-secondary rw-btn-sm' is not a valid attribute name.
```

**Console Output:**
```javascript
[LOG] SET_ATTR: ak=255 vk=254 attr=rw-btn rw-btn-secondary rw-btn-sm val=Decrease
```

**Analysis:**  
The attribute name (resolved as `s[255]`) contains `"rw-btn rw-btn-secondary rw-btn-sm"`, which is clearly a CSS class value, not an attribute name. The attribute value (resolved as `s[254]`) contains `"Decrease"`, which appears to be button text.

This suggests either:
1. Symbol table indices are being swapped during encoding
2. Wrong type of operation is being emitted (SET_CLASS encoded as SET_ATTR)
3. Attribute key/value pairs are getting confused with other element properties

**Reproduction:**
1. Run `cargo run -p design-system`
2. Navigate to http://127.0.0.1:9000
3. Click "Data Display" button
4. Observe JavaScript error in console
5. Page rendering halts after error

**Location:**
- JavaScript runtime: `rwire/rwire/src/capsule_gen.rs` line ~126
- Relevant code: `r[f].setAttribute(an,s[vk]||'')`

**Server Logs:**
```
[127.0.0.1:43786] Event: handler=0x01 type=click target_ref=9
```

**Impact:**  
- Prevents navigation between sections in design-system
- Crashes client-side rendering
- May affect other examples with dynamic updates

**Investigation Needed:**
- Check if Progress component's `attr("style", "width:...")` is related
- Verify symbol table generation order
- Check if SET_CLASS is being incorrectly encoded as SET_ATTR
- Review builder.rs emit_element() attribute emission logic

---

## 🟡 Medium Priority Bugs

### BUG-003: Password Field Console Warnings
**Status:** Open  
**Severity:** Low  
**Component:** Browser Compatibility  
**Discovered:** 2024-02-04

**Description:**  
Browser logs verbose DOM warnings about password fields not being in forms:
```
[VERBOSE] [DOM] Password field is not contained in... https://goo.gl/9p2vKq)
```

**Impact:**  
Cosmetic only - doesn't affect functionality but pollutes console.

**Fix Required:**
- Wrap password inputs in `<form>` elements
- Or suppress browser warning if intentional design

---

## ✅ Fixed Bugs

### BUG-000: Empty CSS Variables Breaking Component Styling
**Status:** Fixed  
**Severity:** Critical  
**Component:** CSS Generation / Tree-shaking  
**Fixed:** 2024-02-04  
**Fixed By:** CSS variable extraction refactor

**Description:**  
Component styling was completely broken because CSS variables (like `--rw-bg-app`, `--rw-neutral-1`) were empty. This was caused by incorrect tree-shaking logic that excluded primitive tokens referenced by semantic CSS.

**Root Cause:**  
In `generate_capsule_css()`, semantic CSS (which references primitives like `--rw-neutral-1`) was generated AFTER primitive tokens were filtered. The filter only looked at component CSS variables, missing the primitives that semantic tokens depend on.

**Fix:**  
Modified `rwire/rwire/src/capsule_gen.rs` to:
1. Extract variables from base CSS
2. Extract variables from component CSS  
3. **Extract variables from semantic CSS** (this was missing)
4. Extract variables from theme overrides
5. THEN filter and generate primitive tokens with complete variable set

**Verification:**  
- All 211 unit tests pass
- New regression test: `test_css_variables_not_empty()`
- Playwright verification shows all CSS variables have proper values
- Component styling renders correctly

---

## Testing Checklist

When fixing bugs, verify:
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` shows zero warnings
- [ ] Visual verification with Playwright
- [ ] Server logs show expected event flow
- [ ] No JavaScript console errors
- [ ] State updates propagate correctly

---

## Debug Commands

```bash
# Run design-system example
cargo run -p design-system

# Run with logging
RUST_LOG=debug cargo run -p design-system

# Kill existing server
pkill -f design-system

# Check server logs
tail -f /tmp/server.log

# Run tests
cargo test --workspace
cargo test --package rwire test_name -- --nocapture

# Clippy
cargo clippy --workspace
```

---

## Playwright Debug Session

```bash
# Start server
cargo run -p design-system > /tmp/server.log 2>&1 &

# Use Playwright MCP tools to:
# - Navigate to http://127.0.0.1:9000
# - Click elements by ref
# - Evaluate JavaScript to inspect state
# - Check console for errors
# - Take screenshots

# Check CSS variables
await page.evaluate(() => {
  const root = document.documentElement;
  const styles = getComputedStyle(root);
  return styles.getPropertyValue('--rw-bg-app');
});
```
