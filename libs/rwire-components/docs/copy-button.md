---
title: CopyButton
description: Button that copies text to clipboard with visual feedback
order: 604
component: copy-button
---

## Import

```rust
use rwire_components::CopyButton;
```

## Usage

```rust
CopyButton::new("npm install rwire").build()
```

## How It Works

The button uses a `data-copy` attribute. The client JS handles the clipboard
copy and shows a brief "copied" feedback state by swapping the clipboard icon
to a checkmark.

## Accessibility

- Renders a `<button>` element with clipboard icon
- Shows visual feedback (checkmark) after copying
- Hover and active states for interaction feedback
