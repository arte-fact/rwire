//! Frontmatter parser for markdown documentation pages.
//!
//! Parses YAML-style frontmatter delimited by `---`:
//!
//! ```text
//! ---
//! title: Installation
//! description: How to install rwire
//! order: 1
//! author: Ada
//! tags: [rust, web]
//! authors:
//!   - Ada
//!   - Grace
//! ---
//! # Installation
//! ...
//! ```
//!
//! `title`, `description`, `order` and `component` fill their fields; any
//! other key lands in [`Frontmatter::extra`], as text or as a list (inline
//! `[a, b]` or one `- item` per line). This is a small subset of YAML: one
//! level, no nested maps, no multi-line strings.

use std::collections::BTreeMap;

/// A value of [`Frontmatter::extra`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrontmatterValue {
    /// `key: value` (surrounding quotes removed).
    Text(String),
    /// `key: [a, b]`, or `key:` followed by `- a` / `- b` lines.
    List(Vec<String>),
}

/// Parsed frontmatter fields.
#[derive(Clone, Debug, Default)]
pub struct Frontmatter {
    /// Page title.
    pub title: String,
    /// Optional description.
    pub description: Option<String>,
    /// Sort order within section.
    pub order: Option<u32>,
    /// Associated component slug (links markdown to a catalog entry).
    pub component: Option<String>,
    /// Every other key, in key order.
    pub extra: BTreeMap<String, FrontmatterValue>,
}

impl Frontmatter {
    /// The text value of an extra key (`None` for a list or a missing key).
    pub fn get(&self, key: &str) -> Option<&str> {
        match self.extra.get(key)? {
            FrontmatterValue::Text(text) => Some(text),
            FrontmatterValue::List(_) => None,
        }
    }

    /// The items of an extra key: a list, or a text value as one item.
    pub fn list(&self, key: &str) -> Option<&[String]> {
        match self.extra.get(key)? {
            FrontmatterValue::List(items) => Some(items),
            FrontmatterValue::Text(text) => Some(std::slice::from_ref(text)),
        }
    }
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

/// The one-level YAML subset of the module docs.
fn parse_yaml_block(block: &str) -> Frontmatter {
    let mut fm = Frontmatter::default();
    // An extra key written `key:` with nothing after it: its `- item` lines
    // follow.
    let mut open_list: Option<String> = None;
    // Keys written `key:` (block lists, possibly with no item).
    let mut block_keys: Vec<String> = Vec::new();

    for raw in block.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(item) = line.strip_prefix("- ").or((line == "-").then_some("")) {
            if let Some(key) = &open_list {
                if let Some(FrontmatterValue::List(items)) = fm.extra.get_mut(key) {
                    items.push(unquote(item.trim()).to_string());
                }
            }
            continue;
        }
        open_list = None;

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();

            match key {
                // Unchanged behaviour for the known fields.
                "title" => fm.title = legacy(value),
                "description" => fm.description = Some(legacy(value)),
                "order" => fm.order = legacy(value).parse().ok(),
                "component" => fm.component = Some(legacy(value)),
                "" => {}
                _ if value.is_empty() => {
                    fm.extra
                        .insert(key.to_string(), FrontmatterValue::List(Vec::new()));
                    open_list = Some(key.to_string());
                    block_keys.push(key.to_string());
                }
                _ => {
                    let parsed = match value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
                        Some(inner) => FrontmatterValue::List(split_inline_list(inner)),
                        None => FrontmatterValue::Text(unquote(value).to_string()),
                    };
                    fm.extra.insert(key.to_string(), parsed);
                }
            }
        }
    }

    // `key:` with no items is an empty text, not an empty list (`key: []`
    // is the empty list).
    for key in block_keys {
        if let Some(value) = fm.extra.get_mut(&key) {
            if matches!(value, FrontmatterValue::List(items) if items.is_empty()) {
                *value = FrontmatterValue::Text(String::new());
            }
        }
    }
    fm
}

/// The known fields' historical quote handling.
fn legacy(value: &str) -> String {
    value.trim_matches('"').trim_matches('\'').to_string()
}

/// Remove one pair of matching surrounding quotes.
fn unquote(value: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(inner) = value
            .strip_prefix(quote)
            .and_then(|v| v.strip_suffix(quote))
        {
            return inner;
        }
    }
    value
}

/// The items of `a, "b, c", d` (commas inside quotes are kept).
fn split_inline_list(inner: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for c in inner.chars() {
        match (quote, c) {
            (None, '"' | '\'') => {
                quote = Some(c);
                current.push(c);
            }
            (Some(q), _) if c == q => {
                quote = None;
                current.push(c);
            }
            (None, ',') => {
                items.push(unquote(current.trim()).to_string());
                current.clear();
            }
            _ => current.push(c),
        }
    }
    if !current.trim().is_empty() || !items.is_empty() {
        items.push(unquote(current.trim()).to_string());
    }
    items.retain(|item| !item.is_empty());
    items
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

    #[test]
    fn extra_keys_keep_text_and_lists() {
        let content = "---\n\
            title: Post\n\
            author: \"Ada Lovelace\"\n\
            date: 2026-11-21\n\
            url: https://example.com/a?b=c\n\
            tags: [rust, \"web, ssr\", 'blog']\n\
            authors:\n\
            \x20 - Ada\n\
            \x20 - \"Grace Hopper\"\n\
            draft:\n\
            empty: []\n\
            ---\n# Post";
        let (fm, rest) = parse_frontmatter(content);
        let fm = fm.unwrap();
        assert_eq!(rest, "# Post");
        assert_eq!(fm.title, "Post");
        assert_eq!(fm.get("author"), Some("Ada Lovelace"));
        assert_eq!(fm.get("date"), Some("2026-11-21"));
        assert_eq!(fm.get("url"), Some("https://example.com/a?b=c"));
        assert_eq!(
            fm.list("tags"),
            Some(
                &[
                    "rust".to_string(),
                    "web, ssr".to_string(),
                    "blog".to_string()
                ][..]
            )
        );
        assert_eq!(
            fm.list("authors"),
            Some(&["Ada".to_string(), "Grace Hopper".to_string()][..])
        );
        assert_eq!(fm.get("draft"), Some(""));
        assert_eq!(fm.list("empty"), Some(&[][..]));
        // A text value reads as a one-item list; a list has no text value.
        assert_eq!(fm.list("author"), Some(&["Ada Lovelace".to_string()][..]));
        assert_eq!(fm.get("tags"), None);
        assert_eq!(fm.get("missing"), None);
        // Known fields never land in `extra`.
        assert!(!fm.extra.contains_key("title"));
    }

    /// The known fields parse exactly as before, extra keys or not.
    #[test]
    fn known_fields_are_unchanged() {
        let content = "---\ntitle: 'It''s'\ndescription: \"A: b\"\norder: 7\n\
            component: card\nnote: x\n---\nBody";
        let (fm, _) = parse_frontmatter(content);
        let fm = fm.unwrap();
        assert_eq!(fm.title, "It''s");
        assert_eq!(fm.description.as_deref(), Some("A: b"));
        assert_eq!(fm.order, Some(7));
        assert_eq!(fm.component.as_deref(), Some("card"));
        assert_eq!(fm.get("note"), Some("x"));
    }

    /// `- item` lines without an open list key are ignored, and a list ends
    /// at the next key.
    #[test]
    fn stray_items_and_list_ends() {
        let content = "---\n- orphan\ntags:\n  - a\nnext: b\n  - not a tag\n---\n";
        let (fm, _) = parse_frontmatter(content);
        let fm = fm.unwrap();
        assert_eq!(fm.list("tags"), Some(&["a".to_string()][..]));
        assert_eq!(fm.get("next"), Some("b"));
        assert_eq!(fm.extra.len(), 2);
    }

    #[test]
    fn test_frontmatter_component_field() {
        let content = "---\ntitle: Button\ncomponent: button\n---\n# Button";
        let (fm, rest) = parse_frontmatter(content);
        let fm = fm.unwrap();
        assert_eq!(fm.title, "Button");
        assert_eq!(fm.component.as_deref(), Some("button"));
        assert_eq!(rest, "# Button");
    }
}
