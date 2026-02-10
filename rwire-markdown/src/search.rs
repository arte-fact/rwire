//! Simple full-text search index for documentation pages.
//!
//! Provides basic keyword matching across page titles and content.
//! No external dependencies required.

use crate::site::DocPage;

/// A search result entry.
#[derive(Clone, Debug)]
pub struct SearchResult {
    /// Page path.
    pub path: String,
    /// Page title.
    pub title: String,
    /// Content snippet around the match.
    pub snippet: String,
    /// Section name.
    pub section: String,
}

/// Search index entry for a single page.
struct IndexEntry {
    path: String,
    title: String,
    section: String,
    /// Lowercased content for searching.
    content_lower: String,
    /// Original content for snippet extraction.
    content: String,
}

/// Full-text search index.
pub struct SearchIndex {
    entries: Vec<IndexEntry>,
}

impl SearchIndex {
    /// Build a search index from doc pages.
    pub fn build<'a>(pages: impl Iterator<Item = &'a DocPage>) -> Self {
        let entries = pages
            .map(|page| IndexEntry {
                path: page.path.clone(),
                title: page.title.clone(),
                section: page.section.clone(),
                content_lower: page.markdown.to_lowercase(),
                content: page.markdown.clone(),
            })
            .collect();

        Self { entries }
    }

    /// Search for pages matching the query.
    ///
    /// Performs case-insensitive substring matching on title and content.
    /// Results are ordered by relevance (title matches first).
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        if query.is_empty() {
            return Vec::new();
        }

        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for entry in &self.entries {
            let title_match = entry.title.to_lowercase().contains(&query_lower);
            let content_pos = entry.content_lower.find(&query_lower);

            if title_match || content_pos.is_some() {
                let snippet = if let Some(pos) = content_pos {
                    extract_snippet(&entry.content, pos, 100)
                } else {
                    entry.content.chars().take(100).collect::<String>()
                };

                results.push((
                    title_match,
                    SearchResult {
                        path: entry.path.clone(),
                        title: entry.title.clone(),
                        snippet,
                        section: entry.section.clone(),
                    },
                ));
            }
        }

        // Title matches first
        results.sort_by(|(a_title, _), (b_title, _)| b_title.cmp(a_title));

        results
            .into_iter()
            .take(limit)
            .map(|(_, result)| result)
            .collect()
    }
}

/// Extract a text snippet around a match position.
fn extract_snippet(content: &str, match_pos: usize, context_chars: usize) -> String {
    let start = match_pos.saturating_sub(context_chars / 2);
    let end = (match_pos + context_chars).min(content.len());

    // Find word boundaries
    let start = content[..start]
        .rfind(char::is_whitespace)
        .map(|p| p + 1)
        .unwrap_or(start);
    let end = content[end..]
        .find(char::is_whitespace)
        .map(|p| end + p)
        .unwrap_or(end);

    let mut snippet = content[start..end].to_string();

    if start > 0 {
        snippet = format!("...{snippet}");
    }
    if end < content.len() {
        snippet.push_str("...");
    }

    snippet
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_snippet() {
        let text = "The quick brown fox jumps over the lazy dog";
        let snippet = extract_snippet(text, 10, 20);
        assert!(snippet.contains("brown"));
    }

    #[test]
    fn test_extract_snippet_start() {
        let text = "Hello world, this is a test";
        let snippet = extract_snippet(text, 0, 20);
        assert!(snippet.starts_with("Hello"));
    }

    #[test]
    fn test_search_empty_query() {
        let index = SearchIndex { entries: Vec::new() };
        let results = index.search("", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_no_results() {
        let index = SearchIndex {
            entries: vec![IndexEntry {
                path: "/test".to_string(),
                title: "Test".to_string(),
                section: "docs".to_string(),
                content_lower: "hello world".to_string(),
                content: "Hello World".to_string(),
            }],
        };
        let results = index.search("xyz", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_title_match() {
        let index = SearchIndex {
            entries: vec![IndexEntry {
                path: "/test".to_string(),
                title: "Installation Guide".to_string(),
                section: "docs".to_string(),
                content_lower: "some content here".to_string(),
                content: "Some content here".to_string(),
            }],
        };
        let results = index.search("install", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Installation Guide");
    }

    #[test]
    fn test_search_content_match() {
        let index = SearchIndex {
            entries: vec![IndexEntry {
                path: "/test".to_string(),
                title: "Page".to_string(),
                section: "docs".to_string(),
                content_lower: "the websocket protocol is fast".to_string(),
                content: "The WebSocket protocol is fast".to_string(),
            }],
        };
        let results = index.search("websocket", 10);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_limit() {
        let index = SearchIndex {
            entries: vec![
                IndexEntry {
                    path: "/a".to_string(),
                    title: "A".to_string(),
                    section: "docs".to_string(),
                    content_lower: "test content".to_string(),
                    content: "Test content".to_string(),
                },
                IndexEntry {
                    path: "/b".to_string(),
                    title: "B".to_string(),
                    section: "docs".to_string(),
                    content_lower: "test content".to_string(),
                    content: "Test content".to_string(),
                },
            ],
        };
        let results = index.search("test", 1);
        assert_eq!(results.len(), 1);
    }
}
