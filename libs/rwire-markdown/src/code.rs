//! Standalone code rendering — a styled `<pre><code>` block with optional syntax highlighting.
//!
//! This is the single code-rendering path: the markdown parser uses it for fenced blocks, and it
//! is public so an app can render a **plain code file** (e.g. a file viewer) the same way, without
//! routing through markdown. Highlighting is best-effort over a handful of languages (see
//! [`crate::highlight`]); an unknown or absent language renders as flat text.
//!
//! The block is one card: a header strip (language, if known, and a copy button) above the code,
//! on the theme's code ground (`--Qc`, darker than the page). The copy button rides the runtime's
//! `data-copy` affordance and its own click state for the checkmark, so it costs no server
//! round-trip.

use rwire::icons::{icon_sized, Icon};
use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder, Ev, Target};

use crate::highlight::{highlight, Token, TokenKind};

/// Client-side state of one block's copy button (swaps the glyph for 2 s).
#[derive(Target)]
struct CopiedCode;

/// The copy button of a code block: clipboard glyph, then a check for two
/// seconds. `data-copy` does the clipboard write; the toggle is local state.
fn copy_button(code: &str) -> ElementBuilder {
    el(El::Button)
        .attr("type", "button")
        .data("copy", code)
        .attr("aria-label", "Copier le code")
        .attr("title", "Copier le code")
        .st([
            St::DisplayInlineFlex,
            St::ItemsCenter,
            St::JustifyCenter,
            St::BgTransparent,
            St::BorderNone,
            St::RoundedSm,
            St::TextMuted,
            St::CursorPointer,
            St::TransitionColors,
        ])
        .hover([St::TextHigh, St::BgSubtle])
        .toggle_timed::<CopiedCode>(Ev::Click, 2000)
        .append([
            el(El::Span)
                .when::<CopiedCode>(St::DisplayNone)
                .append([icon_sized(Icon::Clipboard, 16)]),
            el(El::Span)
                .unless::<CopiedCode>(St::DisplayNone)
                .append([icon_sized(Icon::ClipboardCheck, 16)]),
        ])
}

/// Render `code` as a highlighted block. `lang` is a language name or alias (e.g. `rs`, `sql`,
/// `yml`); `None` or an unrecognised language renders flat. A non-empty `lang` adds a small
/// uppercase language label above the block (matching fenced-code rendering).
pub fn highlight_code(code: &str, lang: Option<&str>) -> ElementBuilder {
    let base = el(El::Code).st([
        St::FontMono,
        St::TextSm,
        St::WhitespacePre,
        St::DisplayBlock,
        St::LeadingRelaxed,
    ]);
    // Colour a recognised language into spans; otherwise keep it as one flat text node.
    let code_el = match lang.and_then(|l| highlight(code, l)) {
        Some(tokens) => base.append(tokens.iter().map(highlight_span).collect::<Vec<_>>()),
        None => base.text(code),
    };
    // The code area itself: no chrome of its own, the card around it carries
    // the ground, the rounding and the border.
    let pre = el(El::Pre)
        .st([St::OverflowXAuto, St::PMd, St::M0, St::BgCode])
        .append([code_el]);
    // Header strip: the language (uppercase, muted) and the copy button.
    let mut head = el(El::Div).st([
        St::DisplayFlex,
        St::ItemsCenter,
        St::JustifyBetween,
        St::GapSm,
        St::PxSm,
        St::PyXs,
        St::BgSubtle,
        St::BorderB,
        St::MinW0,
    ]);
    head = head.append([match lang.filter(|l| !l.is_empty()) {
        Some(lang) => el(El::Span)
            .st([
                St::TextXs,
                St::TextMuted,
                St::TextUppercase,
                St::TrackingWider,
                St::FontMedium,
                St::Truncate,
                St::MinW0,
            ])
            .text(lang),
        None => el(El::Span),
    }]);
    head = head.append([copy_button(code)]);
    el(El::Div)
        .st([
            St::DisplayFlex,
            St::FlexCol,
            St::MinW0,
            St::OverflowHidden,
            St::RoundedLg,
            St::BorderSubtle,
            St::BgCode,
            St::MyMd,
        ])
        .append([head, pre])
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

    /// A fenced block is one card: a header strip with the language and a copy
    /// button (`data-copy`, runtime clipboard) above the `<pre><code>`.
    #[test]
    fn a_block_carries_its_language_and_a_copy_button() {
        let html = highlight_code("let x = 1;", Some("rust")).to_static_html();
        assert!(html.contains("data-copy=\"let x = 1;\""), "{html}");
        assert!(html.contains("aria-label=\"Copier le code\""), "{html}");
        assert!(html.contains("rust"), "language shown: {html}");
        // The clipboard glyph swaps to a check on click (client state, 2 s).
        assert!(html.contains("<svg"), "{html}");
        // The code area is a `<pre><code>` under the header, not a bare label.
        let head = html.find("data-copy").expect("header");
        let pre = html.find("<pre").expect("bloc");
        assert!(head < pre, "header above the code: {html}");
        // Without a language the strip still holds the copy button.
        let bare = highlight_code("ok", None).to_static_html();
        assert!(bare.contains("data-copy=\"ok\""), "{bare}");
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
