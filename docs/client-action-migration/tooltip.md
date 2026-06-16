# Tooltip — Client Action Migration

**File**: `libs/rwire-components/src/tooltip.rs`
**Primitive**: None
**Tier**: N/A (No migration needed)

## Current Behavior

Tooltip is already fully CSS-driven with zero JavaScript. It uses `St::HoverShowChild` on the container, which generates a CSS nesting rule:

```css
&:hover>[data-tip],&:focus-within>[data-tip]{opacity:1}
```

The popup starts at `St::Opacity0` and transitions to `opacity:1` on hover/focus via CSS alone.

```rust
Tooltip::new("Delete this item")
    .position(TooltipPosition::Top)
    .child(Button::primary("Delete").build())
    .build()
```

## Why No Migration

- No server state involved
- No JavaScript involved
- CSS `:hover` / `:focus-within` pseudo-classes handle the toggle
- Zero latency already
- Client actions would add unnecessary binary opcodes for something CSS handles perfectly

## Conclusion

Tooltip is the gold standard for client-side interactivity in rwire — pure CSS, no state, no opcodes. No migration needed.
