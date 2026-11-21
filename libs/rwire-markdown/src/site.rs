//! Documentation site structure.
//!
//! Provides `DocSite` for loading a directory of markdown pages
//! and `DocPage` for individual page data.

use crate::frontmatter;
use crate::search::SearchIndex;
pub use crate::search::SearchResult;

use std::collections::{BTreeMap, HashMap};

use crate::FrontmatterValue;
use std::path::Path;

/// A single documentation page.
#[derive(Clone, Debug)]
pub struct DocPage {
    /// URL path for this page (e.g., "/docs/install").
    pub path: String,
    /// Page title from frontmatter.
    pub title: String,
    /// Optional description from frontmatter.
    pub description: Option<String>,
    /// Section this page belongs to.
    pub section: String,
    /// Raw markdown content (after frontmatter).
    pub markdown: String,
    /// Sort order within section.
    pub order: u32,
}

/// Documentation site with pages, search, and sidebar data.
pub struct DocSite {
    pages: HashMap<String, DocPage>,
    /// The frontmatter's other keys, by page path (pages with any).
    extras: HashMap<String, BTreeMap<String, FrontmatterValue>>,
    sections: Vec<(String, Vec<String>)>,
    search_index: SearchIndex,
}

impl DocSite {
    /// Load a documentation site from a directory.
    ///
    /// Expected structure:
    /// ```text
    /// docs/
    ///   getting-started/
    ///     install.md
    ///     quickstart.md
    ///   components/
    ///     button.md
    ///     input.md
    /// ```
    ///
    /// Each `.md` file can have YAML frontmatter:
    /// ```text
    /// ---
    /// title: Installation
    /// description: How to install rwire
    /// order: 1
    /// tags: [setup, cli]
    /// ---
    /// # Installation
    /// ...
    /// ```
    ///
    /// Keys other than `title`, `description`, `order` and `component` are
    /// kept per page, as text or lists: see [`Self::extra`].
    pub fn load(docs_dir: &str) -> Self {
        let base = Path::new(docs_dir);
        let mut pages = HashMap::new();
        let mut extras = HashMap::new();
        let mut section_map: HashMap<String, Vec<(u32, String)>> = HashMap::new();

        if let Ok(sections) = std::fs::read_dir(base) {
            for section_entry in sections.flatten() {
                let section_path = section_entry.path();
                if !section_path.is_dir() {
                    continue;
                }

                let section_name = section_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                if let Ok(files) = std::fs::read_dir(&section_path) {
                    for file_entry in files.flatten() {
                        let file_path = file_entry.path();
                        if file_path.extension().and_then(|e| e.to_str()) != Some("md") {
                            continue;
                        }

                        let slug = file_path
                            .file_stem()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        if let Ok(content) = std::fs::read_to_string(&file_path) {
                            let (fm, body) = frontmatter::parse_frontmatter(&content);

                            let title = fm
                                .as_ref()
                                .map(|f| f.title.clone())
                                .unwrap_or_else(|| slug.replace('-', " "));
                            let description = fm.as_ref().and_then(|f| f.description.clone());
                            let order = fm.as_ref().and_then(|f| f.order).unwrap_or(100);
                            if let Some(fm) = fm.filter(|f| !f.extra.is_empty()) {
                                extras.insert(format!("/docs/{section_name}/{slug}"), fm.extra);
                            }

                            let page_path = format!("/docs/{}/{}", section_name, slug);

                            section_map
                                .entry(section_name.clone())
                                .or_default()
                                .push((order, page_path.clone()));

                            pages.insert(
                                page_path.clone(),
                                DocPage {
                                    path: page_path,
                                    title,
                                    description,
                                    section: section_name.clone(),
                                    markdown: body.to_string(),
                                    order,
                                },
                            );
                        }
                    }
                }
            }
        }

        // Sort sections and pages within sections
        let mut sections: Vec<(String, Vec<String>)> = section_map
            .into_iter()
            .map(|(name, mut entries)| {
                entries.sort_by_key(|(order, _)| *order);
                let paths: Vec<String> = entries.into_iter().map(|(_, path)| path).collect();
                (name, paths)
            })
            .collect();
        sections.sort_by(|(a, _), (b, _)| a.cmp(b));

        let search_index = SearchIndex::build(pages.values());

        Self {
            pages,
            extras,
            sections,
            search_index,
        }
    }

    /// The frontmatter keys of the page at `path` other than `title`,
    /// `description`, `order` and `component` (see
    /// [`Frontmatter::extra`](crate::Frontmatter::extra)), if it has any.
    pub fn extra(&self, path: &str) -> Option<&BTreeMap<String, FrontmatterValue>> {
        self.extras.get(path)
    }

    /// Get a page by path.
    pub fn page(&self, path: &str) -> Option<&DocPage> {
        self.pages.get(path)
    }

    /// Get all sections with their page paths.
    pub fn sections(&self) -> &[(String, Vec<String>)] {
        &self.sections
    }

    /// Search pages.
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        self.search_index.search(query, limit)
    }

    /// Get page count.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doc_page_struct() {
        let page = DocPage {
            path: "/docs/test".to_string(),
            title: "Test Page".to_string(),
            description: Some("A test page".to_string()),
            section: "testing".to_string(),
            markdown: "# Test\n\nContent".to_string(),
            order: 1,
        };
        assert_eq!(page.title, "Test Page");
        assert_eq!(page.section, "testing");
    }

    #[test]
    fn extra_frontmatter_keys_are_kept_per_page() {
        let dir = std::env::temp_dir().join(format!("rwire-docsite-extra-{}", std::process::id()));
        let section = dir.join("blog");
        std::fs::create_dir_all(&section).unwrap();
        std::fs::write(
            section.join("hello.md"),
            "---\ntitle: Hello\nauthor: Ada\ntags: [rust, web]\n---\n# Hello",
        )
        .unwrap();
        std::fs::write(section.join("plain.md"), "---\ntitle: Plain\n---\n# Plain").unwrap();
        let site = DocSite::load(dir.to_str().unwrap());
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(site.page("/docs/blog/hello").unwrap().title, "Hello");
        let extra = site.extra("/docs/blog/hello").expect("extra keys");
        assert_eq!(
            extra.get("author"),
            Some(&FrontmatterValue::Text("Ada".into()))
        );
        assert_eq!(
            extra.get("tags"),
            Some(&FrontmatterValue::List(vec!["rust".into(), "web".into()]))
        );
        assert!(site.extra("/docs/blog/plain").is_none());
    }
}
