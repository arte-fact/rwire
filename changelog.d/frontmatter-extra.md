### Added: free frontmatter keys in rwire-markdown

- Keys other than `title`, `description`, `order` and `component` are kept
  in `Frontmatter::extra` (`BTreeMap<String, FrontmatterValue>`), as text or
  as a list: inline `[a, "b, c"]` or one `- item` per line. Read them with
  `Frontmatter::get` / `Frontmatter::list`, or per page with
  `DocSite::extra(path)`. `parse_frontmatter` and `FrontmatterValue` are now
  exported. The four known fields parse exactly as before.
