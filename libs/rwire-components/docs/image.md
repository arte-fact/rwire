---
title: Image
description: Responsive image with object-fit and aspect ratio options
order: 309
component: image
---

## Import

```rust
use rwire_components::{Image, ImageFit};
```

## Usage

```rust
Image::new("/photos/hero.jpg")
    .alt("Hero banner")
    .fit(ImageFit::Cover)
    .full_width(true)
    .build()
```

## Object Fit

```rust
Image::new("/img.jpg").fit(ImageFit::Cover).build()   // crop to fill (default)
Image::new("/img.jpg").fit(ImageFit::Contain).build()  // letterbox
Image::new("/img.jpg").fit(ImageFit::Fill).build()     // stretch
Image::new("/img.jpg").fit(ImageFit::None).build()     // no resizing
```

## Options

```rust
Image::new("/photo.jpg")
    .alt("Description")   // alt text
    .rounded(true)         // border-radius
    .full_width(true)      // 100% width
    .build()
```

## Accessibility

- Always set `.alt()` for meaningful images
- Decorative images should use an empty alt: `.alt("")`
- Renders a native `<img>` element
