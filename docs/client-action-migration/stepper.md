# Stepper — Client Action Migration

**File**: `libs/rwire-components/src/stepper.rs`
**Primitive**: Selector
**Tier**: 2 (High Impact)
**Complexity**: Medium

## Current Behavior

Stepper uses a `current: usize` to render step indicators. Each step gets one of three visual states based on its index relative to `current`:
- `i < current` → done (checkmark, `St::StepCircleDone`, `St::StepLineActive`)
- `i == current` → active (`St::StepCircleActive`, `St::FontMedium`)
- `i > current` → pending (`St::StepCircle`, `St::StepLine`, `St::TextMuted`)

```rust
Stepper::new()
    .step("Cart")
    .step("Shipping")
    .step("Payment")
    .current(1)
    .build()
```

No click handlers — the stepper is display-only, driven by server state.

## Target State

```rust
#[derive(Selector)]
enum CheckoutStep {
    #[default]
    Cart,
    Shipping,
    Payment,
}

fn checkout_progress() -> ElementBuilder {
    Stepper::new()
        .step("Cart").variant(CheckoutStep::Cart)
        .step("Shipping").variant(CheckoutStep::Shipping)
        .step("Payment").variant(CheckoutStep::Payment)
        .client_select::<CheckoutStep>()
        .build()
}
```

## Implementation Challenges

### Three-state problem

Unlike Modal (2 states) or Tabs (active/inactive per item), Stepper has 3 visual states per step: done, active, pending. Client actions can only toggle one token per binding — there's no "when less than" or "when greater than" concept.

### Approach: Decompose into per-variant bindings

For each step, emit all three visual states as overlapping elements/tokens, then use Selector bindings to control visibility:

```rust
// For step index 1 ("Shipping"):
// - Done state: visible when selector > 1 (i.e., when Payment is selected)
// - Active state: visible when selector == 1
// - Pending state: visible when selector < 1 (i.e., when Cart is selected)
```

This is complex because Selectors match exact variants, not ranges. Each step would need N bindings (one per variant that makes it "done", one for "active", rest for "pending").

For 3 steps:
- Step 0: active when Cart, done when Shipping or Payment
- Step 1: pending when Cart, active when Shipping, done when Payment
- Step 2: pending when Cart or Shipping, active when Payment

This is O(N²) bindings. For small N (typical: 3-6 steps) it's acceptable.

### Alternative: Server-controlled only

Stepper is often tied to server-side form validation (can't advance to Payment without valid Shipping info). Client actions may not be the right fit. The visual update is secondary to the business logic.

**Recommendation**: Low priority. Implement only if there's a concrete use case where stepper steps are purely navigational (no validation gating).

## Minimal Migration

If implemented, focus on the simple case — clickable steps that advance/retreat:

```rust
Stepper::new()
    .step("Cart").select(CheckoutStep::Cart, Ev::Click)
    .step("Shipping").select(CheckoutStep::Shipping, Ev::Click)
    .build()
```

The three-state visual decomposition (done/active/pending) can be simplified by using two tokens per step:
- Default: pending style
- `.when_eq(variant, St::StepCircleActive)` — the exact match
- For "done" states: `.when_eq(later_variant, St::StepCircleDone)` for each later variant

## Testing

- `test_stepper_client_select` — verify selector_type_id set
- `test_stepper_server_mode_unchanged` — `.current(1)` still works
- `test_stepper_step_bindings` — correct selector bindings per step
