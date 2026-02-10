//! Markdown-to-ElementBuilder parser.
//!
//! Converts markdown into a tree of rwire ElementBuilder nodes using
//! the pulldown-cmark parser. Produces styled elements using the
//! Code, Blockquote, and Prose components.

use rwire::attr_tokens::At;
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

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

/// Parse markdown into an ElementBuilder tree.
///
/// Returns the content wrapped in a Prose container and extracted headings.
pub fn parse_markdown(markdown: &str) -> ParseResult {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS;

    let parser = Parser::new_ext(markdown, options);
    let mut builder = MarkdownBuilder::new();

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
}

impl MarkdownBuilder {
    fn new() -> Self {
        Self {
            stack: Vec::new(),
            children: Vec::new(),
            headings: Vec::new(),
            heading_text: None,
            code_lang: None,
            in_code_block: false,
            text_buf: String::new(),
            in_thead: false,
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
                    .st([St::FontMono, St::BgCode, St::TextSm, St::RoundedSm, St::PxXs, St::PyXs])
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
                self.stack.push(
                    el(El::Blockquote)
                        .st([St::BorderL3Accent, St::PlLg, St::Italic, St::TextMuted, St::MyMd]),
                );
            }
            Tag::CodeBlock(kind) => {
                let lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                        let l = lang.to_string();
                        if l.is_empty() { None } else { Some(l) }
                    }
                    _ => None,
                };
                self.code_lang = lang;
                self.in_code_block = true;
                self.text_buf.clear();
            }
            Tag::List(Some(_)) => {
                self.stack.push(
                    el(El::Ol).st([St::ListDecimal, St::PlLg, St::MyMd]),
                );
            }
            Tag::List(None) => {
                self.stack.push(
                    el(El::Ul).st([St::ListDisc, St::PlLg, St::MyMd]),
                );
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
                self.stack.push(
                    el(El::A)
                        .st([St::TextAccent, St::NoDecoration])
                        .at_str(At::Href, &dest_url),
                );
            }
            Tag::Image { dest_url, title, .. } => {
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
                self.stack.push(el(El::Div).st([St::OverflowXAuto, St::MyMd]));
                self.stack.push(
                    el(El::Table)
                        .st([St::WFull, St::BorderSubtle, St::TextSm]),
                );
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
                    self.stack.push(el(El::Th).st([St::PMd, St::TextLeft, St::FontSemibold, St::BgSubtle]));
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

                let code_el = el(El::Code)
                    .st([St::FontMono, St::TextSm, St::WhitespacePre, St::DisplayBlock, St::LeadingRelaxed])
                    .text(&code_content);

                let mut pre = el(El::Pre)
                    .st([St::BgCode, St::RoundedLg, St::OverflowXAuto, St::PMd, St::BorderSubtle, St::MyMd]);

                if let Some(lang) = lang {
                    let label = el(El::Span)
                        .st([St::TextXs, St::TextMuted, St::DisplayBlock, St::MbSm, St::TextUppercase, St::TrackingWider, St::FontMedium])
                        .text(&lang);
                    let wrapper = el(El::Div)
                        .st([St::PositionRelative])
                        .append([label, pre.append([code_el])]);
                    self.push_to_parent(wrapper);
                } else {
                    pre = pre.append([code_el]);
                    self.push_to_parent(pre);
                }
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
        // Wrap in prose container
        let mut prose = el(El::Div).st([
            St::LeadingRelaxedProse,
            St::TextDefault,
            St::SpaceYMd,
            St::TextBase,
            St::MaxWProse,
        ]);

        for child in self.children {
            prose = prose.append([child]);
        }

        ParseResult {
            content: prose,
            headings: self.headings,
        }
    }
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

/// All element types the markdown parser can produce.
///
/// Use with `CapsuleConfig::extra_elements()` to ensure tree-shaking
/// includes these types even if the startup page has no tables/lists/etc.
pub fn markdown_element_types() -> Vec<El> {
    vec![
        El::Div, El::Span, El::P, El::A, El::Img,
        El::H1, El::H2, El::H3,
        El::Pre, El::Code,
        El::Ul, El::Ol, El::Li,
        El::Em, El::Strong,
        El::Blockquote, El::Hr,
        El::Table, El::Thead, El::Tbody, El::Tr, El::Th, El::Td,
    ]
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

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("Getting Started!"), "getting-started");
        assert_eq!(slugify("API Reference (v2)"), "api-reference-v2");
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
}
