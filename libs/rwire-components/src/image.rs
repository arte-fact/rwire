//! Image component.
//!
//! Responsive image with aspect ratio, object-fit, and loading/error states.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{Image, ImageFit};
//!
//! Image::new("/photos/hero.jpg")
//!     .alt("Hero banner")
//!     .fit(ImageFit::Cover)
//!     .aspect_video()
//!     .build()
//! ```

use rwire::attr_tokens::At;
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Image object-fit mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImageFit {
    /// Cover the container (crop if needed).
    #[default]
    Cover,
    /// Fit within container (letterbox if needed).
    Contain,
    /// Stretch to fill.
    Fill,
    /// No resizing.
    None,
}

/// Image aspect ratio preset.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImageAspect {
    /// No fixed aspect ratio (default).
    #[default]
    Auto,
    /// 1:1 square.
    Square,
    /// 16:9 widescreen.
    Video,
}

/// Image builder.
#[derive(Clone, Default)]
pub struct Image {
    src: Cow<'static, str>,
    alt: Option<Cow<'static, str>>,
    fit: ImageFit,
    aspect: ImageAspect,
    rounded: bool,
    full_width: bool,
    extra_class: Option<Cow<'static, str>>,
}

impl Image {
    /// Create a new image with a source URL.
    pub fn new(src: impl Into<Cow<'static, str>>) -> Self {
        Self {
            src: src.into(),
            ..Self::default()
        }
    }

    /// Set alt text.
    pub fn alt(mut self, alt: impl Into<Cow<'static, str>>) -> Self {
        self.alt = Some(alt.into());
        self
    }

    /// Set object-fit mode.
    pub fn fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self
    }

    /// Set 1:1 square aspect ratio.
    pub fn aspect_square(mut self) -> Self {
        self.aspect = ImageAspect::Square;
        self
    }

    /// Set 16:9 video aspect ratio.
    pub fn aspect_video(mut self) -> Self {
        self.aspect = ImageAspect::Video;
        self
    }

    /// Apply rounded corners.
    pub fn rounded(mut self) -> Self {
        self.rounded = true;
        self
    }

    /// Make image full width.
    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens.
    pub fn compute_tokens(&self) -> Vec<St> {
        let mut tokens = vec![St::DisplayBlock, St::HAuto];

        match self.fit {
            ImageFit::Cover => tokens.push(St::ObjectCover),
            ImageFit::Contain => tokens.push(St::ObjectContain),
            ImageFit::Fill => tokens.push(St::ObjectFill),
            ImageFit::None => tokens.push(St::ObjectNone),
        }

        match self.aspect {
            ImageAspect::Auto => tokens.push(St::AspectAuto),
            ImageAspect::Square => tokens.push(St::AspectSquare),
            ImageAspect::Video => tokens.push(St::AspectVideo),
        }

        if self.rounded {
            tokens.push(St::RoundedMd);
        }

        if self.full_width {
            tokens.push(St::WFull);
        } else {
            tokens.push(St::MaxWFull);
        }

        tokens
    }

    /// Build the image into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut img = el(El::Img)
            .st(self.compute_tokens())
            .at_str(At::Src, &self.src);

        if let Some(ref alt) = self.alt {
            img = img.at_str(At::Alt, alt);
        }

        if let Some(ref extra) = self.extra_class {
            img = img.class(extra.as_ref());
        }

        img
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_defaults() {
        let img = Image::new("/test.jpg");
        assert_eq!(img.src.as_ref(), "/test.jpg");
        assert_eq!(img.fit, ImageFit::Cover);
        assert_eq!(img.aspect, ImageAspect::Auto);
    }

    #[test]
    fn test_image_tokens() {
        let img = Image::new("/test.jpg");
        let tokens = img.compute_tokens();
        assert!(tokens.contains(&St::ObjectCover));
        assert!(tokens.contains(&St::MaxWFull));
        assert!(tokens.contains(&St::AspectAuto));
    }

    #[test]
    fn test_image_cover_square() {
        let img = Image::new("/test.jpg").fit(ImageFit::Cover).aspect_square();
        let tokens = img.compute_tokens();
        assert!(tokens.contains(&St::ObjectCover));
        assert!(tokens.contains(&St::AspectSquare));
    }

    #[test]
    fn test_image_full_width_rounded() {
        let img = Image::new("/test.jpg").full_width().rounded();
        let tokens = img.compute_tokens();
        assert!(tokens.contains(&St::WFull));
        assert!(tokens.contains(&St::RoundedMd));
        assert!(!tokens.contains(&St::MaxWFull));
    }
}
