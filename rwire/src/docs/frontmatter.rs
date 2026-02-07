//! Frontmatter parser for markdown documentation pages.
//!
//! Parses YAML-style frontmatter delimited by `---`:
//!
//! ```text
//! ---
//! title: Installation
//! description: How to install rwire
//! order: 1
//! ---
//! # Installation
//! ...
//! ```

/// Parsed frontmatter fields.
#[derive(Clone, Debug, Default)]
pub struct Frontmatter {
    /// Page title.
    pub title: String,
    /// Optional description.
    pub description: Option<String>,
    /// Sort order within section.
    pub order: Option<u32>,
}

/// Parse frontmatter from a markdown string.
///
/// Returns the parsed frontmatter (if present) and the remaining content.
pub fn parse_frontmatter(content: &str) -> (Option<Frontmatter>, &str) {
    let trimmed = content.trim_start();

    if !trimmed.starts_with("---") {
        return (None, content);
    }

    // Find the closing ---
    let after_opening = &trimmed[3..];
    let closing = after_opening.find("\n---");

    match closing {
        Some(end) => {
            let yaml_block = &after_opening[..end];
            let rest_start = end + 4; // skip \n---
            let rest = after_opening[rest_start..].trim_start_matches('\n');

            let fm = parse_yaml_block(yaml_block);
            (Some(fm), rest)
        }
        None => (None, content),
    }
}

/// Simple YAML key-value parser (no nested objects).
fn parse_yaml_block(block: &str) -> Frontmatter {
    let mut fm = Frontmatter::default();

    for line in block.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');

            match key {
                "title" => fm.title = value.to_string(),
                "description" => fm.description = Some(value.to_string()),
                "order" => fm.order = value.parse().ok(),
                _ => {}
            }
        }
    }

    fm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_frontmatter() {
        let (fm, rest) = parse_frontmatter("# Hello\n\nWorld");
        assert!(fm.is_none());
        assert_eq!(rest, "# Hello\n\nWorld");
    }

    #[test]
    fn test_basic_frontmatter() {
        let content = "---\ntitle: Test Page\ndescription: A test\norder: 3\n---\n# Content";
        let (fm, rest) = parse_frontmatter(content);

        let fm = fm.unwrap();
        assert_eq!(fm.title, "Test Page");
        assert_eq!(fm.description.as_deref(), Some("A test"));
        assert_eq!(fm.order, Some(3));
        assert_eq!(rest, "# Content");
    }

    #[test]
    fn test_frontmatter_with_quotes() {
        let content = "---\ntitle: \"Quoted Title\"\n---\n# Content";
        let (fm, _) = parse_frontmatter(content);
        let fm = fm.unwrap();
        assert_eq!(fm.title, "Quoted Title");
    }

    #[test]
    fn test_frontmatter_missing_closing() {
        let content = "---\ntitle: No Close\n# Content";
        let (fm, rest) = parse_frontmatter(content);
        assert!(fm.is_none());
        assert_eq!(rest, content);
    }

    #[test]
    fn test_frontmatter_partial_fields() {
        let content = "---\ntitle: Just Title\n---\nBody";
        let (fm, _) = parse_frontmatter(content);
        let fm = fm.unwrap();
        assert_eq!(fm.title, "Just Title");
        assert!(fm.description.is_none());
        assert!(fm.order.is_none());
    }
}
