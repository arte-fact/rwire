# rwire Bug Summary Report
**Date:** 2024-02-04  
**Testing Method:** Playwright E2E Testing  
**Example:** design-system

---

## Executive Summary

Comprehensive testing of the rwire design-system example revealed **2 critical bugs** and **1 minor issue**:

1. ✅ **FIXED:** Empty CSS variables (tree-shaking bug) - **RESOLVED**
2. 🔴 **CRITICAL:** State changes don't trigger UI re-renders
3. 🔴 **CRITICAL:** InvalidCharacterError crashes rendering on navigation
4. 🟡 **MINOR:** Password field browser warnings

---

## Bug Details

### ✅ BUG-000: Empty CSS Variables (FIXED)

**Status:** ✅ Fixed  
**Severity:** Critical  
**Impact:** All component styling was broken

**Problem:**  
CSS variables like `--rw-bg-app`, `--rw-neutral-1`, `--rw-blue-9` were empty because tree-shaking excluded primitive tokens that semantic CSS depends on.

**Root Cause:**  
`generate_capsule_css()` filtered primitive tokens before extracting variables from semantic CSS, which references those primitives.

**Fix Applied:**
```rust
// rwire/rwire/src/capsule_gen.rs - generate_capsule_css()

// OLD (buggy):
// 1. Get component CSS
// 2. Extract variables from component CSS
// 3. Filter primitives ❌ (semantic CSS variables not included!)
// 4. Generate semantic CSS

// NEW (fixed):
// 1. Get base CSS → extract variables
// 2. Get component CSS → extract variables
// 3. Generate semantic CSS → extract variables ✅
// 4. Get theme overrides → extract variables
// 5. Filter primitives with COMPLETE variable set
// 6. Output all CSS
```

**Verification:**
- ✅ All 211 tests pass
- ✅ Zero clippy warnings
- ✅ New regression test added
- ✅ Playwright confirms all CSS vars have values
- ✅ Components render with proper styling

---

### 🔴 BUG-001: State Changes Don't Trigger Re-renders

**Status:** 🔴 Open - Needs Fix  
**Severity:** High  
**Component:** Reactivity System

**Problem:**  
Clicking interactive form elements (checkboxes, radios, switches) updates the DOM element state but doesn't re-render dependent UI text.

**Example:**
```
1. Click "Subscribe to newsletter" checkbox
2. ✅ Checkbox becomes checked (DOM updated)
3. ❌ Text stays "Newsletter: No" (should change to "Newsletter: Yes")
```

**Server Logs:**
```
[127.0.0.1:43786] Event: handler=0x03 type=change target_ref=45
```
Event received, handler executed, state changed, but NO update sent back.

**Root Cause:**
```rust
// examples/design-system/src/main.rs

// ❌ Missing #[renderer] macro!
fn render_forms_section(state: &DesignSystemState) -> ElementBuilder {
    Stack::column()
        .gap(Gap::Lg)
        .children([
            // Checkbox with state dependency
            el(El::P).text(&format!(
                "Newsletter: {}",
                if state.checkbox_checked { "Yes" } else { "No" }
            )),
        ])
        .build(),
}
```

Without `#[renderer]`, the function isn't tracked as a dependency of `checkbox_checked` field, so state changes don't trigger re-renders.

**Fix Required:**
```rust
// Add #[renderer] to ALL state-dependent render functions

#[renderer]  // ← Add this
fn render_forms_section(state: &DesignSystemState) -> ElementBuilder {
    // ... existing code
}

#[renderer]  // ← Add this
fn render_data_display_section(state: &DesignSystemState) -> ElementBuilder {
    // ... existing code
}

#[renderer]  // ← Add this
fn render_feedback_section(state: &DesignSystemState) -> ElementBuilder {
    // ... existing code
}
```

**Impact:**
- All interactive examples are broken
- Checkbox, radio, switch, progress demos don't work
- Poor user experience in design system showcase

**Test Plan:**
1. Add `#[renderer]` macros
2. Rebuild and run
3. Click checkbox → verify text updates
4. Click radio buttons → verify selection text updates
5. Toggle switch → verify status text updates
6. Click progress +/- → verify percentage updates

---

### 🔴 BUG-002: InvalidCharacterError on Navigation

**Status:** 🔴 Open - Needs Investigation  
**Severity:** Critical  
**Component:** Binary Protocol / JavaScript Runtime

**Problem:**  
Clicking "Data Display" button causes JavaScript error and halts rendering:

```javascript
InvalidCharacterError: Failed to execute 'setAttribute' on 'Element': 
'rw-btn rw-btn-secondary rw-btn-sm' is not a valid attribute name.
```

**Console Output:**
```javascript
[LOG] SET_ATTR: ak=255 vk=254 attr=rw-btn rw-btn-secondary rw-btn-sm val=Decrease
```

**Analysis:**
- `s[255]` contains `"rw-btn rw-btn-secondary rw-btn-sm"` (a CSS class value!)
- `s[254]` contains `"Decrease"` (button text!)
- Code tries: `element.setAttribute("rw-btn rw-btn-secondary rw-btn-sm", "Decrease")`
- This is clearly wrong - neither should be an attribute

**Possible Causes:**

1. **Symbol Table Corruption:**
   - Class values incorrectly stored in symbol table
   - Symbols retrieved with wrong indices

2. **Opcode Confusion:**
   - SET_CLASS (0x10) mistakenly encoded as SET_ATTR (0x12)
   - Wrong opcode chosen during element update

3. **Attribute/Value Swap:**
   - Protocol encoder swaps key/value during updates
   - Different from initial render behavior

4. **Progress Component:**
   - Uses `attr("style", "width:X%")` for inline styles
   - May be related to the error

**Investigation Steps:**
```rust
// Check these locations:

// 1. Protocol encoder
rwire/rwire/src/protocol/encoder.rs:209
pub fn set_attr(&mut self, ref_idx: u8, attr_symbol: u32, value_symbol: u32)

// 2. Element emission
rwire/rwire/src/builder.rs:838
for (key, value) in &el.attrs {
    let key_sym = self.get_or_intern_symbol(key);
    let val_sym = self.get_or_intern_symbol(value);
    self.buf.set_attr(ref_idx, key_sym, val_sym);  // ← Check order
}

// 3. JavaScript decoder
rwire/rwire/src/capsule_gen.rs:126
else if(o===O.A){
    let f=d[i++],[ak,al]=rv(d,i);i+=al;
    let[vk,vl]=rv(d,i);i+=vl;
    let an=A[ak]||s[ak]||'data';
    r[f].setAttribute(an,s[vk]||'')  // ← Error here
}

// 4. Progress component
rwire/rwire/src/components/progress.rs:115
.attr("style", &format!("width:{}%", percentage))  // ← Suspicious
```

**Impact:**
- Navigation between sections crashes
- Client-side rendering stops
- Entire app becomes unusable
- Likely affects other dynamic updates

**Fix Strategy:**
1. Add protocol encoder/decoder logging
2. Trace symbol table generation
3. Compare initial render vs. update render
4. Verify SET_CLASS vs SET_ATTR emission
5. Check if Progress inline styles trigger the bug

---

### 🟡 BUG-003: Password Field Warnings

**Status:** 🟡 Minor  
**Severity:** Low  
**Component:** HTML Semantics

**Problem:**
```
[VERBOSE] [DOM] Password field is not contained in a form. https://goo.gl/9p2vKq
```

**Cause:**  
Password inputs in design-system are not wrapped in `<form>` elements.

**Fix:**
```rust
// Wrap password demos in forms or use autocomplete="off"
Input::password()
    .placeholder("Password")
    .attr("autocomplete", "off")  // May suppress warning
    .build()
```

**Impact:** Cosmetic only - doesn't break functionality.

---

## Testing Results Summary

### ✅ What Works

1. **CSS Variables** - All properly defined and resolved
   ```javascript
   {
     "--rw-bg-app": "oklch(0.985 0 0)",
     "--rw-neutral-1": "oklch(0.985 0 0)",
     "--rw-blue-9": "oklch(0.55 0.18 250)",
     "--rw-space-4": "1rem",
     // ... all variables have values ✅
   }
   ```

2. **Component Styling** - Buttons, cards, all components render correctly
   ```javascript
   {
     "backgroundColor": "oklch(0.55 0.18 250)",  // ✅ Blue primary
     "color": "oklch(1 0 0)",                    // ✅ White text
     "padding": "0px 16px",                      // ✅ Proper spacing
     "borderRadius": "6px",                      // ✅ Rounded corners
     "height": "36px"                            // ✅ Correct size
   }
   ```

3. **Initial Page Render** - Everything displays correctly on first load

4. **Event Reception** - Server receives click/change events properly

### ❌ What's Broken

1. **State Reactivity** - UI doesn't update after state changes
2. **Section Navigation** - Crashes with InvalidCharacterError
3. **Interactive Demos** - All form component demos appear non-functional

---

## Fix Priority

### 🚨 Immediate (Critical)

1. **BUG-002 (InvalidCharacterError)**
   - Blocks all navigation
   - Breaks app functionality
   - Requires protocol investigation

2. **BUG-001 (Missing Re-renders)**
   - Easy fix (add 3 macros)
   - High user impact
   - Critical for showcase app

### 📋 Soon (Medium)

3. **BUG-003 (Password Warnings)**
   - Low impact
   - Easy fix
   - Polish for production

---

## Recommended Action Plan

### Phase 1: Quick Win (15 minutes)
```bash
# Fix BUG-001 - Add missing #[renderer] macros
1. Edit examples/design-system/src/main.rs
2. Add #[renderer] to three functions
3. Test with Playwright
4. Verify state updates work
```

### Phase 2: Deep Dive (2-4 hours)
```bash
# Fix BUG-002 - Protocol debugging
1. Add logging to encoder/decoder
2. Capture protocol bytes during navigation
3. Compare working vs broken message structure
4. Fix root cause (likely in update path)
5. Add regression test
```

### Phase 3: Polish (30 minutes)
```bash
# Fix BUG-003 - Form semantics
1. Wrap password inputs in forms
2. Test browser warnings
3. Update other examples if needed
```

---

## Testing Checklist

Before considering bugs fixed:

- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` shows zero warnings
- [ ] Initial page renders correctly
- [ ] Click checkbox → text updates immediately
- [ ] Click radio → selection text updates
- [ ] Toggle switch → status text updates
- [ ] Click "Data Display" → no JavaScript errors
- [ ] Navigate all sections successfully
- [ ] Progress +/- buttons work
- [ ] No console errors or warnings
- [ ] Take screenshot for visual verification

---

## Additional Notes

### Code Quality
- Zero clippy warnings currently ✅
- All 211 unit tests passing ✅
- Good test coverage overall

### Architecture Observations
- Binary protocol is elegant and compact
- CSS tree-shaking works well (after fix)
- Component system is clean
- Need better reactivity documentation

### Future Improvements
- Add integration tests for state updates
- Protocol fuzzing to catch edge cases
- Better error messages in JavaScript runtime
- Debug mode for protocol inspection

---

## Debug Commands Reference

```bash
# Start server with logging
cargo run -p design-system 2>&1 | tee /tmp/server.log

# Watch logs
tail -f /tmp/server.log

# Kill server
pkill -f design-system

# Run specific test
cargo test test_css_variables_not_empty -- --nocapture

# Playwright inspection
# (Use MCP tools to interact with browser)
browser_navigate("http://127.0.0.1:9000")
browser_click(ref="e48")  # Click checkbox
browser_console_messages(level="error")
```

---

**Report Generated:** 2024-02-04  
**Engineer:** Claude (AI Assistant)  
**Testing Tool:** Playwright Browser Automation  
**Framework Version:** rwire 0.1.0