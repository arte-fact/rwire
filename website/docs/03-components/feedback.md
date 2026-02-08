---
title: Feedback Components
description: Alert, Toast, Spinner, Progress, Skeleton, Modal
order: 6
---
# Feedback Components

## Alert

Persistent messages with intent-based styling and a left border accent.

```rust
use rwire::components::{Alert, AlertIntent};

Alert::info()
    .title("Note")
    .message("Your changes have been saved")
    .build()

Alert::error()
    .title("Connection Failed")
    .message("Could not reach the server. Please try again.")
    .build()

Alert::warning()
    .title("Rate Limited")
    .message("You've exceeded the request limit.")
    .build()

Alert::success()
    .title("Deployed")
    .message("v2.1.0 is now live in production.")
    .build()
```

| Intent | Background | Border |
|--------|-----------|--------|
| `Info` | Blue | Blue |
| `Success` | Green | Green |
| `Warning` | Yellow | Yellow |
| `Error` | Red | Red |

Alerts render with `role="alert"` for screen reader announcements.

---

## Toast

Transient notifications positioned at the bottom-right corner of the screen. Server pushes toasts via state; the browser displays them with a slide-in animation.

```rust
use rwire::components::{Toast, ToastContainer, ToastIntent};

// In a renderer, show active toasts:
ToastContainer::new()
    .toast(Toast::success("Changes saved"))
    .toast(Toast::error("Failed to delete"))
    .build()

// With dismiss handler
Toast::info("New message received")
    .on_dismiss(dismiss_toast())
    .build()
```

`ToastContainer` fixes itself to the bottom-right with a high z-index. When the toast list is empty, it renders `display:none`. Each toast supports intent variants: `Info`, `Success`, `Warning`, `Error`. Providing an `on_dismiss` handler adds a close button.

---

## Spinner

Animated loading indicator.

```rust
use rwire::components::{Spinner, SpinnerSize};

// Default medium spinner
Spinner::new().build()

// Large spinner with custom label
Spinner::new()
    .size(SpinnerSize::Lg)
    .label("Loading data...")
    .build()
```

| Size | Dimensions |
|------|-----------|
| `Sm` | 1rem (16px) |
| `Md` | 1.5rem (24px, default) |
| `Lg` | 2rem (32px) |

Renders a `<span>` with `role="status"` and `aria-label="Loading"` (or your custom label). The spinning animation uses a transparent right border on a rounded element.

---

## Progress

Determinate progress bar showing completion state.

```rust
use rwire::components::Progress;

// Percentage-based (default max: 100)
Progress::new()
    .value(65)
    .build()

// Step-based
Progress::new()
    .value(3)
    .max(5)
    .label("Step 3 of 5")
    .build()
```

Renders with `role="progressbar"`, `aria-valuenow`, `aria-valuemin`, and `aria-valuemax`. The inner bar width is calculated as a percentage and set via inline style. The bar has a smooth transition when the value changes.

---

## Skeleton

Loading placeholder that shows the shape of content before data arrives.

```rust
use rwire::components::Skeleton;

// Single text line
Skeleton::text().build()

// Multi-line text (last line is shorter for a natural look)
Skeleton::text().lines(3).build()

// Circle (avatar placeholder)
Skeleton::circle().build()

// Rectangle (card/image placeholder)
Skeleton::rect().build()
```

| Shape | Dimensions |
|-------|-----------|
| `Text` | Full width, 1rem height |
| `Circle` | 3rem diameter |
| `Rect` | Full width, 6rem min-height |

All shapes use a shimmer animation background. Multi-line text wraps in a flex column with `Gap::Sm`.

---

## Modal

Dialog overlay with backdrop, focus management, and keyboard handling. Modal visibility is controlled by server state.

```rust
use rwire::components::{Modal, ModalSize};

Modal::new()
    .title("Confirm Action")
    .size(ModalSize::Lg)
    .open(state.modal_open)
    .on_close(close_modal())
    .content(el(El::P).text("Are you sure you want to proceed?"))
    .footer(
        Stack::row().gap(Gap::Sm).children([
            Button::secondary("Cancel").on_click(close_modal()),
            Button::primary("Confirm").on_click(confirm_action()),
        ]).build()
    )
    .build()
```

| Size | Width |
|------|-------|
| `Sm` | 400px |
| `Md` | 600px (default) |
| `Lg` | 800px |
| `Xl` | 1000px |
| `Full` | 100% viewport |

When `open` is `false`, the modal renders as `display:none`. When open, it shows a semi-transparent backdrop (clickable to close by default), a centered panel with the title bar, scrollable content area, and an optional footer. Set `.close_on_backdrop_click(false)` to require explicit dismissal. The modal uses `role="dialog"` and `aria-modal="true"`.

---

## EmptyState

Placeholder for empty lists, search results, or data views.

```rust
use rwire::components::EmptyState;

EmptyState::new()
    .title("No results found")
    .description("Try adjusting your search terms.")
    .action(Button::primary("Clear filters").on_click(clear_handler()))
    .build()
```

Centers content vertically and horizontally with generous padding. Supports an optional icon element, title, description, and action (any `ElementBuilder` -- typically a button or link).
