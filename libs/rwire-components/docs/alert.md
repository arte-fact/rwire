---
title: Alert
description: Alert messages with intent-based styling
order: 400
component: alert
---

## Import

```rust
use rwire_components::{Alert, AlertIntent};
```

## Usage

```rust
Alert::info()
    .title("Note")
    .message("Your changes have been saved")
    .build()

Alert::error()
    .title("Error")
    .message("Failed to connect to server")
    .build()
```

## Intent Constructors

```rust
Alert::info().message("Informational message").build()
Alert::success().message("Operation completed").build()
Alert::warning().message("Proceed with caution").build()
Alert::error().message("Something went wrong").build()
```

## With Title and Message

```rust
Alert::new()
    .intent(AlertIntent::Warning)
    .title("Deprecation Notice")
    .message("This API will be removed in v2.0")
    .build()
```

## Accessibility

- Uses `role="alert"` for screen reader announcements
- Intent colors provide visual distinction (blue, green, amber, red)
- Title is rendered with stronger emphasis
