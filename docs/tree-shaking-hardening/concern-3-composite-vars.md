# Concern 3: Composite CSS Vars Not Scanned

## Problem

`generate_capsule_css()` extracts `var(--..)` references from utility, pseudo, and breakpoint CSS to determine which primitive/color variables to include. But it does not scan `config.composite_css`, so any CSS variable used only in composite styles is silently omitted from the `:root` block.

## Solution

Add one line after the existing `extract_used_variables` calls:

```rust
used_vars.extend(extract_used_variables(&config.composite_css));
```

**Files modified:**
- `libs/rwire/src/capsule_gen.rs` — `generate_capsule_css()`
