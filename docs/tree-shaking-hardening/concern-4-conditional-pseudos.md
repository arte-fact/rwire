# Concern 4: Conditional Pseudo/Breakpoint Styles

## Problem

A subcase of Concern 1. When pseudo-class styles (`.hover()`, `.focus()`) or breakpoint styles (`.sm()`, `.md()`) are applied conditionally based on state (e.g., only `ButtonIntent::Destructive` has `.hover([St::BgDestructiveHover])`), the tree-shaker misses those `(Pc, St)` or `(Bp, St)` pairs.

## Solution

Solved by compile-time token scanning (same as Concern 1):

1. **`#[renderer]`** scans for `.hover()`, `.focus()`, `.sm()`, `.md()` calls with `St::*` arguments — discovers all pseudo/breakpoint pairs across all branches.

2. **`#[component]`** scans entire component impl blocks, including methods like `apply_pseudo()` where conditional pseudo styles are typically defined.

See [Concern 1](concern-1-default-state-only.md) for implementation details.
