# Concern 5: No Validation When Uncollected Tokens Emitted

## Problem

When `emit_update_element()` emits style tokens not collected during tree-shaking, the CSS class doesn't exist. This fails silently.

## Solution

The proc macro token scanning approach largely eliminates this class of bug by discovering all tokens at compile time. The remaining gap is tokens generated inside called functions that are not visible to the macro scanner. In practice, this is rare since:

1. `#[renderer]` catches all direct `St::*` references across all branches in renderer functions
2. `#[component]` catches all tokens across all methods in component impl blocks
3. `collect_symbols` still renders with default state to catch function-generated tokens
4. `collect_tokens_from` now handles synced elements in router views
5. `extra_styles()` on `CapsuleConfig` provides explicit declaration for known edge cases
