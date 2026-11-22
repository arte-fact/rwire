//! Standalone code rendering — a styled `<pre><code>` block with optional syntax highlighting.
//!
//! This is the single code-rendering path: the markdown parser uses it for fenced blocks, and it
//! is public so an app can render a **plain code file** (e.g. a file viewer) the same way, without
//! routing through markdown. Highlighting is best-effort over a handful of languages (see
//! [`crate::highlight`]); an unknown or absent language renders as flat text.

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder, ProseFace};

use crate::highlight::{highlight, Token, TokenKind};

/// Render `code` as a highlighted block. `lang` is a language name or alias (e.g. `rs`, `sql`,
/// `yml`); `None` or an unrecognised language renders flat. A non-empty `lang` adds a small
/// uppercase language label above the block (matching fenced-code rendering).
pub fn highlight_code(code: &str, lang: Option<&str>) -> ElementBuilder {
    highlight_code_face(code, lang, ProseFace::Ui)
}

/// [`highlight_code`] in a given face. In the reading face (`ProseFace::Read`, a chat
/// answer) the block is mono 12.5 px on the theme's code ground, radius 10, scrolling
/// horizontally inside its frame (`overflow-x:auto`) so a long line never scrolls the
/// page; the interface face keeps the larger `TextSm`.
pub fn highlight_code_face(code: &str, lang: Option<&str>, face: ProseFace) -> ElementBuilder {
    let size = match face {
        ProseFace::Read => St::Text12_5,
        ProseFace::Ui => St::TextSm,
    };
    let base = el(El::Code).st([
        St::FontMono,
        size,
        St::WhitespacePre,
        St::DisplayBlock,
        St::LeadingRelaxed,
    ]);
    // Colour a recognised language into spans; otherwise keep it as one flat text node.
    let code_el = match lang.and_then(|l| highlight(code, l)) {
        Some(tokens) => base.append(tokens.iter().map(highlight_span).collect::<Vec<_>>()),
        None => base.text(code),
    };
    // In the reading face the block itself runs at 12.5 px (the /app value);
    // the interface face inherits the wrapper's size.
    let pre_size = match face {
        ProseFace::Read => Some(St::Text12_5),
        ProseFace::Ui => None,
    };
    let mut pre = el(El::Pre)
        .st([
            St::BgCode,
            St::RoundedLg,
            St::OverflowXAuto,
            St::PMd,
            St::BorderSubtle,
            St::MyMd,
        ])
        .data("testid", "markdown-code");
    if let Some(s) = pre_size {
        pre = pre.st([s]);
    }
    match lang {
        Some(lang) if !lang.is_empty() => {
            let label = el(El::Span)
                .st([
                    St::TextXs,
                    St::TextMuted,
                    St::DisplayBlock,
                    St::MbSm,
                    St::TextUppercase,
                    St::TrackingWider,
                    St::FontMedium,
                ])
                .text(lang);
            el(El::Div)
                .st([St::PositionRelative])
                .append([label, pre.append([code_el])])
        }
        _ => pre.append([code_el]),
    }
}

/// Render one highlighted token as a coloured `<span>`. Keyword→accent, string→success (green),
/// number→warning (amber), comment→muted; plain text inherits the block's colour.
fn highlight_span(token: &Token) -> ElementBuilder {
    let span = el(El::Span).text(&token.text);
    match token.kind {
        TokenKind::Keyword => span.st([St::TextAccent]),
        TokenKind::Str => span.st([St::TextSuccess]),
        TokenKind::Number => span.st([St::TextWarning]),
        TokenKind::Comment => span.st([St::TextMuted]),
        TokenKind::Plain => span,
    }
}

/// Highlight `code` into one `ElementBuilder` per line (bare spans, no block
/// chrome) — for gutter-aligned code views like `rwire-editor`'s read-only
/// mode, where each line must be its own row. Unknown languages yield plain
/// text lines.
pub fn highlight_lines(code: &str, lang: Option<&str>) -> Vec<ElementBuilder> {
    let tokens = lang.and_then(|l| highlight(code, l));
    let line_count = code.lines().count().max(1);
    let mut lines: Vec<Vec<ElementBuilder>> = Vec::with_capacity(line_count);
    lines.push(Vec::new());
    match tokens {
        Some(tokens) => {
            for token in &tokens {
                let mut first = true;
                for part in token.text.split('\n') {
                    if !first {
                        lines.push(Vec::new());
                    }
                    first = false;
                    if !part.is_empty() {
                        let piece = Token {
                            kind: token.kind,
                            text: part.to_string(),
                        };
                        lines.last_mut().unwrap().push(highlight_span(&piece));
                    }
                }
            }
        }
        None => {
            lines = code
                .split('\n')
                .map(|l| vec![el(El::Span).text(l)])
                .collect();
        }
    }
    // Trailing newline produces a phantom empty last line; keep parity with
    // `str::lines()` so gutters align.
    if lines.len() > line_count {
        lines.truncate(line_count);
    }
    lines
        .into_iter()
        .map(|spans| el(El::Div).st([St::WhitespacePre, St::MinW0]).append(spans))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_for_known_unknown_and_no_language() {
        // All three paths must build without panicking.
        let _ = highlight_code("fn main() {}", Some("rust"));
        let _ = highlight_code("+[->+]", Some("brainfuck"));
        let _ = highlight_code("plain", None);
    }

    #[test]
    fn the_reading_face_sets_12_5px_code_on_a_radius_10_scrolling_block() {
        // No language: the root is the pre itself.
        let read = highlight_code_face("let x = 1;", None, ProseFace::Read);
        let html = read.to_static_html();
        assert!(html.contains("data-testid=\"markdown-code\""), "{html}");
        let pre_utils = read.get_style_utils();
        // The pre scrolls on its own, keeps the theme's 10 px radius and runs at 12.5 px.
        assert!(pre_utils.contains(&St::OverflowXAuto.as_u16()));
        assert!(pre_utils.contains(&St::RoundedLg.as_u16()));
        assert!(pre_utils.contains(&St::Text12_5.as_u16()));
        // The code runs at 12.5 px in the mono face.
        let code_utils = read.children().last().unwrap().get_style_utils();
        assert!(code_utils.contains(&St::FontMono.as_u16()));
        assert!(code_utils.contains(&St::Text12_5.as_u16()));
        // With a language, a label sits above the same pre.
        let labelled = highlight_code_face("let x = 1;", Some("rust"), ProseFace::Read);
        let pre = labelled.children().last().unwrap();
        assert!(pre.get_style_utils().contains(&St::OverflowXAuto.as_u16()));
        assert!(pre
            .children()
            .last()
            .unwrap()
            .get_style_utils()
            .contains(&St::Text12_5.as_u16()));
        // The interface face keeps TextSm.
        let ui = highlight_code("let x = 1;", None);
        assert!(ui
            .children()
            .last()
            .unwrap()
            .get_style_utils()
            .contains(&St::TextSm.as_u16()));
    }

    #[test]
    fn highlight_span_maps_each_kind() {
        for kind in [
            TokenKind::Keyword,
            TokenKind::Str,
            TokenKind::Number,
            TokenKind::Comment,
            TokenKind::Plain,
        ] {
            let _ = highlight_span(&Token {
                kind,
                text: "x".to_string(),
            });
        }
    }
}
