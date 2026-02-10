---
title: AvatarGroup
description: Stacked display of multiple avatars with an overflow count
order: 308
component: avatar-group
---

## Import

```rust
use rwire_components::{AvatarGroup, Avatar, AvatarSize};
```

## Usage

```rust
AvatarGroup::new()
    .avatar(Avatar::new().fallback("AB"))
    .avatar(Avatar::new().fallback("CD"))
    .avatar(Avatar::new().fallback("EF"))
    .avatar(Avatar::new().fallback("GH"))
    .max_visible(3) // shows 3 avatars + "+1" badge
    .build()
```

## Size

```rust
// Set size for all avatars in the group
AvatarGroup::new()
    .avatar(Avatar::new().fallback("AB"))
    .avatar(Avatar::new().fallback("CD"))
    .size(AvatarSize::Sm)
    .build()
```

## Accessibility

- Overflow count is displayed as a "+N" badge
- Avatars overlap with a ring border for visual separation
