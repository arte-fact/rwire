---
title: Stepper
description: Multi-step progress indicator with numbered circles and connecting lines
order: 606
component: stepper
---

## Import

```rust
use rwire_components::Stepper;
```

## Usage

```rust
Stepper::new()
    .step("Cart")
    .step("Shipping")
    .step("Payment")
    .step("Confirm")
    .current(1) // 0-indexed, "Shipping" is active
    .build()
```

## Step States

Steps are rendered in three visual states based on the `current` index:

- **Completed** (index < current): checkmark icon, accent color
- **Active** (index == current): numbered circle, highlighted
- **Upcoming** (index > current): numbered circle, muted

```rust
Stepper::new()
    .step("Account")
    .step("Profile")
    .step("Review")
    .current(2) // "Account" and "Profile" completed, "Review" active
    .build()
```

## Accessibility

- Steps are connected with visual lines
- Current step is prominently highlighted
- Completed steps show a checkmark
