---
title: Toast
description: Transient notification messages shown at screen edge
order: 401
component: toast
---

## Import

```rust
use rwire_components::{Toast, ToastIntent, ToastContainer};
```

## Usage

```rust
// Host toasts in a container
ToastContainer::new()
    .toast(Toast::success("Changes saved"))
    .toast(Toast::error("Failed to delete"))
    .build()
```

## Intent Constructors

```rust
Toast::info("Information message").build()
Toast::success("Operation completed").build()
Toast::warning("Check your input").build()
Toast::error("Something went wrong").build()
```

## Dismissable Toasts

```rust
Toast::new("File uploaded")
    .intent(ToastIntent::Success)
    .on_dismiss(dismiss_toast())
    .build()
```

## Toast Container

```rust
// The container positions toasts at the screen edge
ToastContainer::new()
    .toast(Toast::info("Toast 1"))
    .toast(Toast::success("Toast 2"))
    .build()
```

## Accessibility

- Toasts use `role="status"` for polite screen reader announcements
- Dismiss buttons are keyboard accessible
- Intent colors match Alert conventions (blue, green, amber, red)
