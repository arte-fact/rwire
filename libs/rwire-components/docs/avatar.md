---
title: Avatar
description: User avatar with image or fallback text
order: 307
component: avatar
---

## Import

```rust
use rwire_components::{Avatar, AvatarSize};
```

## Usage

```rust
// With image
Avatar::new()
    .src("/users/avatar.jpg")
    .alt("John Doe")
    .build()

// With fallback initials
Avatar::new()
    .fallback("JD")
    .size(AvatarSize::Lg)
    .build()
```

## Sizes

```rust
Avatar::new().fallback("AB").size(AvatarSize::Sm).build() // 32px
Avatar::new().fallback("AB").size(AvatarSize::Md).build() // 40px (default)
Avatar::new().fallback("AB").size(AvatarSize::Lg).build() // 48px
```

## Accessibility

- Image avatars use the `alt` attribute for screen readers
- Fallback text is displayed when no image is provided
- Renders as a circular element
