//! Footer component.
//!
//! Multi-column footer with logo area, link columns, and copyright line.
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::{Footer, FooterColumn};
//!
//! Footer::new()
//!     .logo(el(El::Span).text("rwire"))
//!     .tagline("Server-side UI framework")
//!     .column(FooterColumn::new("Docs")
//!         .link("Getting Started", "/docs/getting-started")
//!         .link("API Reference", "/docs/api"))
//!     .column(FooterColumn::new("Community")
//!         .external_link("GitHub", "https://github.com/example/rwire"))
//!     .copyright("2026 rwire contributors")
//!     .build()
//! ```

use rwire::router::Link;
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};

/// A column of links in the footer.
#[derive(Clone)]
pub struct FooterColumn {
    title: String,
    links: Vec<FooterLink>,
}

#[derive(Clone)]
struct FooterLink {
    label: String,
    href: String,
    external: bool,
}

impl FooterColumn {
    /// Create a new footer column with a title.
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            links: Vec::new(),
        }
    }

    /// Add an internal link (uses client-side routing).
    pub fn link(mut self, label: &str, href: &str) -> Self {
        self.links.push(FooterLink {
            label: label.to_string(),
            href: href.to_string(),
            external: false,
        });
        self
    }

    /// Add an external link (opens in new tab).
    pub fn external_link(mut self, label: &str, href: &str) -> Self {
        self.links.push(FooterLink {
            label: label.to_string(),
            href: href.to_string(),
            external: true,
        });
        self
    }
}

/// Footer builder.
#[derive(Clone, Default)]
pub struct Footer {
    logo: Option<ElementBuilder>,
    tagline: Option<String>,
    columns: Vec<FooterColumn>,
    copyright: Option<String>,
}

impl Footer {
    /// Create a new footer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the logo element.
    pub fn logo(mut self, logo: ElementBuilder) -> Self {
        self.logo = Some(logo);
        self
    }

    /// Set the tagline text.
    pub fn tagline(mut self, tagline: &str) -> Self {
        self.tagline = Some(tagline.to_string());
        self
    }

    /// Add a link column.
    pub fn column(mut self, column: FooterColumn) -> Self {
        self.columns.push(column);
        self
    }

    /// Set the copyright text.
    pub fn copyright(mut self, text: &str) -> Self {
        self.copyright = Some(text.to_string());
        self
    }

    /// Build the footer into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let mut footer = el(El::Footer).st([
            St::BgSubtle,
            St::BorderT,
            St::PyXl,
            St::PxLg,
        ]);

        // Top section: logo/tagline + link columns
        let mut top = el(El::Div).st([
            St::DisplayFlex,
            St::FlexWrap,
            St::GapXl,
            St::JustifyBetween,
            St::MbLg,
        ]);

        // Brand area (logo + tagline)
        if self.logo.is_some() || self.tagline.is_some() {
            let mut brand = el(El::Div).st([St::DisplayFlex, St::FlexCol, St::GapSm]);
            if let Some(logo) = self.logo {
                brand = brand.append([logo]);
            }
            if let Some(ref tagline) = self.tagline {
                brand = brand.append([
                    el(El::P).st([St::TextSm, St::TextMuted]).text(tagline),
                ]);
            }
            top = top.append([brand]);
        }

        // Link columns
        if !self.columns.is_empty() {
            let mut cols = el(El::Div).st([
                St::DisplayFlex,
                St::FlexWrap,
                St::GapXl,
            ]);

            for col in &self.columns {
                let mut column = el(El::Div).st([St::DisplayFlex, St::FlexCol, St::GapSm]);

                column = column.append([
                    el(El::Div).st([St::TextSm, St::FontSemibold, St::TextHigh]).text(&col.title),
                ]);

                for link in &col.links {
                    let link_el = if link.external {
                        el(El::A)
                            .attr("href", &link.href)
                            .attr("target", "_blank")
                            .attr("rel", "noopener noreferrer")
                            .st([St::TextSm, St::TextMuted, St::NoDecoration, St::CursorPointer])
                            .hover([St::TextDefault])
                            .text(&link.label)
                    } else {
                        Link::to(&link.href, &link.label)
                            .st([St::TextSm, St::TextMuted, St::NoDecoration, St::CursorPointer])
                            .hover([St::TextDefault])
                    };
                    column = column.append([link_el]);
                }

                cols = cols.append([column]);
            }

            top = top.append([cols]);
        }

        footer = footer.append([top]);

        // Copyright line
        if let Some(ref copyright) = self.copyright {
            footer = footer.append([
                el(El::Div)
                    .st([St::BorderT, St::PtMd, St::TextSm, St::TextMuted])
                    .append([
                        el(El::P).text(copyright),
                    ]),
            ]);
        }

        footer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_footer_defaults() {
        let footer = Footer::new();
        assert!(footer.logo.is_none());
        assert!(footer.tagline.is_none());
        assert!(footer.columns.is_empty());
        assert!(footer.copyright.is_none());
    }

    #[test]
    fn test_footer_column() {
        let col = FooterColumn::new("Docs")
            .link("Install", "/docs/install")
            .external_link("GitHub", "https://github.com");
        assert_eq!(col.title, "Docs");
        assert_eq!(col.links.len(), 2);
        assert!(!col.links[0].external);
        assert!(col.links[1].external);
    }

    #[test]
    fn test_footer_builds() {
        let footer = Footer::new()
            .tagline("A UI framework")
            .copyright("2026 rwire")
            .build();
        drop(footer);
    }
}
