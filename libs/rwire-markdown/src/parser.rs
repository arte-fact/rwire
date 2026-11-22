//! Markdown-to-ElementBuilder parser.
//!
//! Converts markdown into a tree of rwire ElementBuilder nodes using
//! the pulldown-cmark parser. Produces styled elements using the
//! Code, Blockquote, and Prose components.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use rwire::attr_tokens::At;
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder, ProseFace};

use crate::prose::ProseSize;

/// Result of parsing markdown.
pub struct ParseResult {
    /// The rendered ElementBuilder tree (wrapped in Prose container).
    pub content: ElementBuilder,
    /// Extracted headings for table of contents.
    pub headings: Vec<TocEntry>,
}

/// A heading entry for table of contents generation.
#[derive(Clone, Debug)]
pub struct TocEntry {
    /// Heading level (1-6).
    pub level: u8,
    /// Heading text content.
    pub text: String,
    /// Anchor ID (slug of heading text).
    pub anchor: String,
}

/// Options for [`parse_markdown_with_options`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseOptions {
    /// Prose text size of the wrapper.
    pub size: ProseSize,
    /// Whether the wrapper spans the full width (no reading-width cap).
    pub full_width: bool,
    /// Whether images from another origin (`https://…`, `//…`) are loaded.
    /// Off by default: loading one tells that server who reads the page and
    /// when (a tracking pixel), so a remote image is rendered as its alt
    /// text. Same-origin images and raster `data:image/*` are always kept.
    pub remote_images: bool,
    /// The face the wrapper is set in. `Ui` (the default): the interface face
    /// at `size`, with the `65ch` reading cap unless `full_width`. `Read`:
    /// the theme's reading face (`--Qfr`, 15.5 px / 15 px on a phone, line
    /// height 1.6) spanning its container — a chat answer inside the reading
    /// column, where the column itself is the cap.
    pub face: ProseFace,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            size: ProseSize::Base,
            full_width: false,
            remote_images: false,
            face: ProseFace::Ui,
        }
    }
}

/// Parse markdown into an ElementBuilder tree, wrapped in a default prose container
/// (`Base` size, max-width constrained). See [`parse_markdown_with`] to control those.
///
/// Returns the content wrapped in a Prose container and extracted headings.
pub fn parse_markdown(markdown: &str) -> ParseResult {
    parse_markdown_with(markdown, ProseSize::Base, false)
}

/// Parse markdown, controlling the prose wrapper's text `size` and whether it spans the
/// `full_width` (dropping the reading-width cap). [`Markdown`](crate::Markdown) routes its
/// `.size()` / `.full_width()` here.
pub fn parse_markdown_with(markdown: &str, size: ProseSize, full_width: bool) -> ParseResult {
    parse_markdown_with_options(
        markdown,
        ParseOptions {
            size,
            full_width,
            ..ParseOptions::default()
        },
    )
}

/// Parse markdown with every [`ParseOptions`] (e.g. `remote_images: true`).
pub fn parse_markdown_with_options(markdown: &str, opts: ParseOptions) -> ParseResult {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;

    let parser = Parser::new_ext(markdown, options);
    let mut builder = MarkdownBuilder::new(opts);

    for event in parser {
        builder.process_event(event);
    }

    builder.finish()
}

/// Internal builder that processes pulldown-cmark events.
struct MarkdownBuilder {
    /// Stack of open elements being built.
    stack: Vec<ElementBuilder>,
    /// Top-level children collected.
    children: Vec<ElementBuilder>,
    /// Extracted headings.
    headings: Vec<TocEntry>,
    /// Current heading text accumulator.
    heading_text: Option<(u8, String)>,
    /// Current code block language.
    code_lang: Option<String>,
    /// Whether we're inside a code block.
    in_code_block: bool,
    /// Text accumulator for code blocks.
    text_buf: String,
    /// Whether we're inside a <thead>.
    in_thead: bool,
    /// Prose text size for the wrapper.
    size: ProseSize,
    /// Whether the prose wrapper spans full width (no reading-width cap).
    full_width: bool,
    /// Whether images from another origin are loaded.
    remote_images: bool,
    /// The face the wrapper (and its code blocks) are set in.
    face: ProseFace,
}

impl MarkdownBuilder {
    fn new(opts: ParseOptions) -> Self {
        let ParseOptions {
            size,
            full_width,
            remote_images,
            face,
        } = opts;
        Self {
            stack: Vec::new(),
            children: Vec::new(),
            headings: Vec::new(),
            heading_text: None,
            code_lang: None,
            in_code_block: false,
            text_buf: String::new(),
            in_thead: false,
            size,
            full_width,
            remote_images,
            face,
        }
    }

    fn process_event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.open_tag(tag),
            Event::End(tag) => self.close_tag(tag),
            Event::Text(text) => {
                if let Some((_, ref mut buf)) = self.heading_text {
                    buf.push_str(&text);
                }
                if self.in_code_block {
                    self.text_buf.push_str(&text);
                } else {
                    self.push_text(&text);
                }
            }
            Event::Code(code) => {
                // Inline code
                let code_el = el(El::Code)
                    .st([
                        St::FontMono,
                        St::BgCode,
                        St::TextSm,
                        St::RoundedSm,
                        St::PxXs,
                        St::PyXs,
                    ])
                    .text(&code);
                self.push_child(code_el);
            }
            Event::SoftBreak | Event::HardBreak => {
                self.push_text(" ");
            }
            Event::Rule => {
                self.push_to_parent(el(El::Hr).st([St::MyMd, St::BorderSubtle]));
            }
            _ => {}
        }
    }

    fn open_tag(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => {
                self.stack.push(el(El::P).st([St::M0, St::LeadingRelaxed]));
            }
            Tag::Heading { level, .. } => {
                let heading_level = heading_level_to_u8(level);
                self.heading_text = Some((heading_level, String::new()));

                let el_type = match level {
                    HeadingLevel::H1 => El::H1,
                    HeadingLevel::H2 => El::H2,
                    HeadingLevel::H3 => El::H3,
                    _ => El::H3,
                };

                let tokens = match level {
                    HeadingLevel::H1 => vec![St::Text2xl, St::FontBold, St::MtLg, St::MbSm],
                    HeadingLevel::H2 => vec![St::TextXl, St::FontSemibold, St::Mt2xl, St::MbSm],
                    HeadingLevel::H3 => vec![St::TextLg, St::FontMedium, St::MtXl, St::MbXs],
                    _ => vec![St::TextBase, St::FontMedium, St::MtMd],
                };

                self.stack.push(el(el_type).st(tokens));
            }
            Tag::BlockQuote(_) => {
                self.stack.push(el(El::Blockquote).st([
                    St::BorderL3Accent,
                    St::PlLg,
                    St::Italic,
                    St::TextMuted,
                    St::MyMd,
                ]));
            }
            Tag::CodeBlock(kind) => {
                let lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                        let l = lang.to_string();
                        if l.is_empty() {
                            None
                        } else {
                            Some(l)
                        }
                    }
                    _ => None,
                };
                self.code_lang = lang;
                self.in_code_block = true;
                self.text_buf.clear();
            }
            Tag::List(Some(_)) => {
                self.stack
                    .push(el(El::Ol).st([St::ListDecimal, St::PlLg, St::MyMd]));
            }
            Tag::List(None) => {
                self.stack
                    .push(el(El::Ul).st([St::ListDisc, St::PlLg, St::MyMd]));
            }
            Tag::Item => {
                self.stack.push(el(El::Li).st([St::MySm]));
            }
            Tag::Emphasis => {
                self.stack.push(el(El::Em));
            }
            Tag::Strong => {
                self.stack.push(el(El::Strong).st([St::FontSemibold]));
            }
            Tag::Strikethrough => {
                self.stack.push(el(El::Span).st([St::LineThrough]));
            }
            Tag::Link { dest_url, .. } => {
                // Only relative URLs, anchors, `http`, `https` and `mailto` keep
                // their href; any other scheme (`javascript:`, `data:`, `tel:`,
                // an unknown one) loses it: the text stays, as an inert anchor.
                let mut link = el(El::A).st([St::TextAccent, St::NoDecoration]);
                if rwire::url_safety::is_allowed_link(&dest_url) {
                    link = link.at_str(At::Href, &dest_url);
                }
                self.stack.push(link);
            }
            Tag::Image {
                dest_url, title, ..
            } => {
                // An unsafe URL drops the image, and so does a remote one unless
                // `remote_images` is on; its alt text still reads, as text.
                if !rwire::url_safety::is_allowed_image(&dest_url)
                    || (!self.remote_images && rwire::url_safety::is_remote(&dest_url))
                {
                    return;
                }
                let mut img = el(El::Img)
                    .st([St::MaxWFull, St::HAuto, St::RoundedMd, St::MyMd])
                    .at_str(At::Src, &dest_url);
                if !title.is_empty() {
                    img = img.at_str(At::Alt, &title);
                }
                self.push_to_parent(img);
            }
            Tag::Table(_) => {
                // Wrapper div allows horizontal scroll on narrow viewports
                self.stack
                    .push(el(El::Div).st([St::OverflowXAuto, St::MyMd]));
                self.stack
                    .push(el(El::Table).st([St::WFull, St::BorderSubtle, St::TextSm]));
            }
            Tag::TableHead => {
                self.in_thead = true;
                self.stack.push(el(El::Thead));
            }
            Tag::TableRow => {
                self.stack.push(el(El::Tr).st([St::BorderBSubtle]));
            }
            Tag::TableCell => {
                if self.in_thead {
                    self.stack.push(el(El::Th).st([
                        St::PMd,
                        St::TextLeft,
                        St::FontSemibold,
                        St::BgSubtle,
                    ]));
                } else {
                    self.stack.push(el(El::Td).st([St::PMd]));
                }
            }
            _ => {}
        }
    }

    fn close_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::CodeBlock => {
                self.in_code_block = false;
                let code_content = std::mem::take(&mut self.text_buf);
                let lang = self.code_lang.take();
                // The shared renderer also serves standalone code files — one code path.
                self.push_to_parent(crate::code::highlight_code_face(
                    &code_content,
                    lang.as_deref(),
                    self.face,
                ));
            }
            TagEnd::Heading(_) => {
                if let Some((level, text)) = self.heading_text.take() {
                    let anchor = slugify(&text);
                    if let Some(mut heading_el) = self.stack.pop() {
                        heading_el = heading_el.attr("id", &anchor);
                        self.push_to_parent(heading_el);
                    }
                    self.headings.push(TocEntry {
                        level,
                        text,
                        anchor: format!("#{anchor}"),
                    });
                } else {
                    self.close_current();
                }
            }
            TagEnd::Table => {
                // Close table into wrapper div, then wrapper into parent
                self.close_current(); // table → wrapper div
                self.close_current(); // wrapper div → parent
            }
            TagEnd::TableHead => {
                self.in_thead = false;
                self.close_current();
            }
            TagEnd::Image => {
                // Image was already pushed in open_tag
            }
            _ => {
                self.close_current();
            }
        }
    }

    fn push_text(&mut self, text: &str) {
        // Wrap text in a span so it becomes a child node alongside inline elements
        // like <code>, <strong>, <em>. Using .text() on the parent would set
        // textContent which overwrites other children.
        let text_node = el(El::Span).text(text);
        self.push_child(text_node);
    }

    fn push_child(&mut self, child: ElementBuilder) {
        if let Some(current) = self.stack.pop() {
            self.stack.push(current.append([child]));
        } else {
            self.children.push(child);
        }
    }

    fn push_to_parent(&mut self, element: ElementBuilder) {
        if let Some(current) = self.stack.pop() {
            self.stack.push(current.append([element]));
        } else {
            self.children.push(element);
        }
    }

    fn close_current(&mut self) {
        if let Some(element) = self.stack.pop() {
            self.push_to_parent(element);
        }
    }

    fn finish(self) -> ParseResult {
        // Wrap in the prose container, honouring the requested face, size and
        // reading-width cap.
        let content = match self.face {
            ProseFace::Ui => {
                let mut prose = el(El::Div).st(prose_tokens(self.size, self.full_width));
                for child in self.children {
                    prose = prose.append([child]);
                }
                prose
            }
            ProseFace::Read => {
                // The reading face spans its container (the chat's reading
                // column is the cap); `SpaceYMd`/`BreakWords` keep the rhythm
                // and let long tokens wrap instead of overflowing.
                let mut prose = ProseFace::Read
                    .apply(el(El::Div).st([St::SpaceYMd, St::BreakWords]))
                    .data("testid", "markdown");
                for child in self.children {
                    prose = prose.append([child]);
                }
                prose
            }
        };

        ParseResult {
            content,
            headings: self.headings,
        }
    }
}

/// Style tokens for the prose wrapper. `BreakWords` (overflow-wrap) lets long unbroken
/// tokens — URLs, inline code — break instead of forcing horizontal overflow on a narrow
/// viewport (code blocks opt back into `white-space: pre` with their own scroll). The
/// size token and the `MaxWProse` reading-width cap follow the requested options.
fn prose_tokens(size: ProseSize, full_width: bool) -> Vec<St> {
    let mut tokens = vec![
        St::LeadingRelaxedProse,
        St::TextDefault,
        St::SpaceYMd,
        St::BreakWords,
        match size {
            ProseSize::Sm => St::TextSm,
            ProseSize::Base => St::TextBase,
            ProseSize::Lg => St::TextLg,
        },
    ];
    if !full_width {
        tokens.push(St::MaxWProse);
    }
    tokens
}

/// Convert heading text to a URL-safe slug.
fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The static HTML of `markdown`, for checking hrefs and srcs.
    fn html(markdown: &str) -> String {
        parse_markdown(markdown).content.to_static_html()
    }

    const TRAPS: [&str; 9] = [
        "[x](javascript:alert(1))",
        "[x](JaVaScRiPt:alert(1))",
        "[x](<java\tscript:alert(1)>)",
        "[x](&#106;avascript:alert(1))",
        "[x](javascript&#58;alert(1))",
        "[x](%6Aavascript:alert(1))",
        "[x](vbscript:msgbox(1))",
        "[x](data:text/html,<script>alert(1)</script>)",
        "<javascript:alert(1)>",
    ];

    #[test]
    fn unsafe_link_urls_lose_their_href() {
        for md in TRAPS {
            let out = html(md);
            assert!(out.contains("<a"), "{md:?} is not a link: {out}");
            assert!(!out.contains("href"), "{md:?} kept an href: {out}");
        }
        // The text stays.
        assert!(html("[read me](javascript:alert(1))").contains("read me"));
        // Reference-style links take the same path.
        let out = html("[x][evil]\n\n[evil]: javascript:alert(1)\n");
        assert!(!out.contains("href"), "{out}");
    }

    #[test]
    fn unsafe_image_urls_drop_the_image() {
        for md in [
            "![alt](javascript:alert(1))",
            "![alt](data:image/svg+xml,<svg onload=alert(1)>)",
            "![alt](data:text/html,x)",
            "![alt][pic]\n\n[pic]: vbscript:x\n",
        ] {
            let out = html(md);
            assert!(!out.contains("<img"), "{md:?} kept the image: {out}");
            assert!(out.contains("alt"), "{md:?} lost its alt text: {out}");
        }
        let out = html("![dot](data:image/png;base64,iVBORw0KGgo=)");
        assert!(
            out.contains("src=\"data:image/png;base64,iVBORw0KGgo=\""),
            "{out}"
        );
    }

    #[test]
    fn safe_urls_are_kept() {
        for (md, href) in [
            ("[a](/docs/intro)", "/docs/intro"),
            ("[a](guide.md)", "guide.md"),
            ("[a](#install)", "#install"),
            ("[a](?tab=2)", "?tab=2"),
            (
                "[a](https://example.com/x?y=1)",
                "https://example.com/x?y=1",
            ),
            ("[a](mailto:me@example.com)", "mailto:me@example.com"),
            ("<https://example.com>", "https://example.com"),
        ] {
            let out = html(md);
            assert!(out.contains(&format!("href=\"{href}\"")), "{md:?}: {out}");
        }
        assert!(html("![c](/img/cover.png)").contains("src=\"/img/cover.png\""));
    }

    /// Only relative links, anchors, `http`, `https` and `mailto` keep their
    /// `href`: `tel:` and any other scheme are rendered as text.
    #[test]
    fn other_link_schemes_are_text() {
        for md in [
            "[call](tel:+33123456789)",
            "[x](sms:+33123456789)",
            "[x](foo:bar)",
            "[x](magnet:?xt=urn:btih:x)",
            "[x](file:///etc/passwd)",
        ] {
            let out = html(md);
            assert!(!out.contains("href"), "{md:?} kept an href: {out}");
        }
        assert!(html("[call](tel:+33123456789)").contains("call"));
    }

    /// A remote image (absolute `http(s)://` or protocol-relative `//`) is not
    /// loaded by default: its alt text reads instead, as text.
    #[test]
    fn remote_images_are_not_loaded_by_default() {
        for md in [
            "![a cat](https://x.example/y.png)",
            "![a cat](http://x.example/y.png)",
            "![a cat](HTTPS://x.example/y.png)",
            "![a cat](//x.example/y.png)",
            "![a cat][pic]\n\n[pic]: https://x.example/y.png\n",
        ] {
            let out = html(md);
            assert!(!out.contains("<img"), "{md:?} loads a remote image: {out}");
            assert!(!out.contains("x.example"), "{md:?}: {out}");
            assert!(out.contains("a cat"), "{md:?} lost its alt text: {out}");
        }
        // Same-origin images and raster data stay.
        assert!(html("![c](/img/cover.png)").contains("src=\"/img/cover.png\""));
        assert!(html("![c](img/cover.png)").contains("src=\"img/cover.png\""));
        assert!(html("![d](data:image/png;base64,iVBORw0KGgo=)").contains("<img"));
    }

    #[test]
    fn remote_images_load_when_allowed() {
        let opts = ParseOptions {
            remote_images: true,
            ..ParseOptions::default()
        };
        for (md, src) in [
            ("![a](https://x.example/y.png)", "https://x.example/y.png"),
            ("![a](//x.example/y.png)", "//x.example/y.png"),
        ] {
            let out = parse_markdown_with_options(md, opts)
                .content
                .to_static_html();
            assert!(out.contains(&format!("src=\"{src}\"")), "{md:?}: {out}");
        }
        // Unsafe URLs stay dropped.
        let out = parse_markdown_with_options("![a](javascript:alert(1))", opts)
            .content
            .to_static_html();
        assert!(!out.contains("<img"), "{out}");
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("Getting Started!"), "getting-started");
        assert_eq!(slugify("API Reference (v2)"), "api-reference-v2");
    }

    #[test]
    fn prose_size_selects_the_matching_text_token() {
        // Regression: `Markdown::build()` used to ignore `.size()`, hardcoding `TextBase`.
        assert!(prose_tokens(ProseSize::Sm, false).contains(&St::TextSm));
        assert!(prose_tokens(ProseSize::Lg, false).contains(&St::TextLg));
        let base = prose_tokens(ProseSize::Base, false);
        assert!(base.contains(&St::TextBase));
        assert!(!base.contains(&St::TextSm) && !base.contains(&St::TextLg));
    }

    #[test]
    fn full_width_drops_the_reading_width_cap() {
        assert!(prose_tokens(ProseSize::Base, false).contains(&St::MaxWProse));
        assert!(!prose_tokens(ProseSize::Base, true).contains(&St::MaxWProse));
    }

    #[test]
    fn test_parse_simple_markdown() {
        let result = parse_markdown("# Hello\n\nWorld");
        assert_eq!(result.headings.len(), 1);
        assert_eq!(result.headings[0].text, "Hello");
        assert_eq!(result.headings[0].level, 1);
        assert_eq!(result.headings[0].anchor, "#hello");
    }

    #[test]
    fn test_parse_multiple_headings() {
        let md = "# Title\n\n## Section 1\n\nText\n\n## Section 2\n\nMore text\n\n### Sub";
        let result = parse_markdown(md);
        assert_eq!(result.headings.len(), 4);
        assert_eq!(result.headings[0].level, 1);
        assert_eq!(result.headings[1].level, 2);
        assert_eq!(result.headings[2].level, 2);
        assert_eq!(result.headings[3].level, 3);
    }

    #[test]
    fn test_parse_empty_markdown() {
        let result = parse_markdown("");
        assert!(result.headings.is_empty());
    }

    #[test]
    fn test_heading_level_conversion() {
        assert_eq!(heading_level_to_u8(HeadingLevel::H1), 1);
        assert_eq!(heading_level_to_u8(HeadingLevel::H6), 6);
    }

    #[test]
    fn fenced_code_blocks_render_for_known_and_unknown_languages() {
        // The highlighted path (known language → coloured spans) and the fallback path
        // (unknown/blank language → flat text) must both build without panicking.
        let highlighted = parse_markdown("```rust\nfn main() { let x = 1; }\n```");
        assert!(highlighted.headings.is_empty());
        let fallback = parse_markdown("```\nplain text\n```");
        assert!(fallback.headings.is_empty());
        let unknown_lang = parse_markdown("```brainfuck\n+[->+]\n```");
        assert!(unknown_lang.headings.is_empty());
    }

    #[test]
    fn the_read_face_sets_the_wrapper_and_its_code_blocks() {
        use rwire::ProseFace;
        let opts = ParseOptions {
            face: ProseFace::Read,
            ..ParseOptions::default()
        };
        let out = parse_markdown_with_options(
            "## Answer\n\nSome text.\n\n```rust\nlet x = 1;\n```",
            opts,
        );
        let wrapper = out.content;
        let utils = wrapper.get_style_utils();
        assert!(utils.contains(&St::FontRead.as_u16()));
        assert!(utils.contains(&St::TextProsePhone.as_u16()));
        assert!(!utils.contains(&St::MaxWProse.as_u16()));
        let html = wrapper.to_static_html();
        assert!(html.contains("data-testid=\"markdown\""), "{html}");
        assert!(html.contains("data-testid=\"markdown-code\""), "{html}");
        // The Ui face keeps the 65ch cap and the Base size.
        let ui = parse_markdown("```\nx\n```");
        assert!(ui
            .content
            .get_style_utils()
            .contains(&St::MaxWProse.as_u16()));
    }

    #[test]
    fn the_read_face_keeps_the_unsafe_link_and_remote_image_neutralisation() {
        use rwire::ProseFace;
        let opts = ParseOptions {
            face: ProseFace::Read,
            ..ParseOptions::default()
        };
        for md in TRAPS {
            let out = parse_markdown_with_options(md, opts)
                .content
                .to_static_html();
            assert!(!out.contains("href"), "{md:?} kept an href: {out}");
        }
        let out = parse_markdown_with_options("![a cat](https://x.example/y.png)", opts)
            .content
            .to_static_html();
        assert!(!out.contains("<img"), "{out}");
        assert!(out.contains("a cat"), "{out}");
    }
}
