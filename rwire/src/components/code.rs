//! Code component.
//!
//! Inline code and code blocks for documentation and code display.
//!
//! # Example
//!
//! ```ignore
//! use rwire::components::Code;
//!
//! // Inline code
//! Code::inline("let x = 42").build()
//!
//! // Code block
//! Code::block("fn main() {\n    println!(\"Hello\");\n}")
//!     .language("rust")
//!     .build()
//! ```

use crate::style_tokens::St;
use crate::{el, El, ElementBuilder};
use std::borrow::Cow;

/// Code display mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CodeMode {
    /// Inline code span
    #[default]
    Inline,
    /// Multi-line code block
    Block,
}

/// Code component builder.
#[derive(Clone, Debug, Default)]
pub struct Code {
    mode: CodeMode,
    content: Cow<'static, str>,
    language: Option<Cow<'static, str>>,
    extra_class: Option<Cow<'static, str>>,
}

impl Code {
    /// Create an inline code element.
    pub fn inline(content: impl Into<Cow<'static, str>>) -> Self {
        Self {
            mode: CodeMode::Inline,
            content: content.into(),
            ..Self::default()
        }
    }

    /// Create a code block element.
    pub fn block(content: impl Into<Cow<'static, str>>) -> Self {
        Self {
            mode: CodeMode::Block,
            content: content.into(),
            ..Self::default()
        }
    }

    /// Set the language label (for code blocks).
    pub fn language(mut self, lang: impl Into<Cow<'static, str>>) -> Self {
        self.language = Some(lang.into());
        self
    }

    /// Add custom class.
    pub fn class(mut self, class: impl Into<Cow<'static, str>>) -> Self {
        self.extra_class = Some(class.into());
        self
    }

    /// Compute style tokens for inline code.
    pub fn compute_inline_tokens() -> Vec<St> {
        vec![
            St::FontMono,
            St::BgCode,
            St::TextSm,
            St::RoundedSm,
            St::PxXs,
            St::PyXs,
        ]
    }

    /// Compute style tokens for the code block container (pre).
    pub fn compute_block_tokens() -> Vec<St> {
        vec![
            St::BgCode,
            St::RoundedMd,
            St::OverflowXAuto,
            St::PSm,
            St::BorderSubtle,
        ]
    }

    /// Compute style tokens for the code element inside a block.
    pub fn compute_block_code_tokens() -> Vec<St> {
        vec![
            St::FontMono,
            St::TextSm,
            St::WhitespacePre,
            St::DisplayBlock,
        ]
    }

    /// Build the code component into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        match self.mode {
            CodeMode::Inline => {
                let mut code = el(El::Code)
                    .st(Self::compute_inline_tokens())
                    .text(&self.content);

                if let Some(ref extra) = self.extra_class {
                    code = code.class(extra.as_ref());
                }

                code
            }
            CodeMode::Block => {
                let code_el = el(El::Code)
                    .st(Self::compute_block_code_tokens())
                    .text(&self.content);

                let mut pre = el(El::Pre)
                    .st(Self::compute_block_tokens())
                    .append([code_el]);

                if let Some(ref extra) = self.extra_class {
                    pre = pre.class(extra.as_ref());
                }

                if let Some(ref lang) = self.language {
                    // Wrap in a container with language label
                    let label = el(El::Span)
                        .st([St::TextXs, St::TextMuted, St::DisplayBlock, St::MbXs])
                        .text(lang.as_ref());

                    el(El::Div)
                        .st([St::PositionRelative])
                        .append([label, pre])
                } else {
                    pre
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_inline() {
        let code = Code::inline("let x = 42");
        assert_eq!(code.mode, CodeMode::Inline);
        assert_eq!(code.content.as_ref(), "let x = 42");
    }

    #[test]
    fn test_code_block() {
        let code = Code::block("fn main() {}");
        assert_eq!(code.mode, CodeMode::Block);
    }

    #[test]
    fn test_code_inline_tokens() {
        let tokens = Code::compute_inline_tokens();
        assert!(tokens.contains(&St::FontMono));
        assert!(tokens.contains(&St::BgCode));
        assert!(tokens.contains(&St::TextSm));
        assert!(tokens.contains(&St::RoundedSm));
    }

    #[test]
    fn test_code_block_tokens() {
        let tokens = Code::compute_block_tokens();
        assert!(tokens.contains(&St::BgCode));
        assert!(tokens.contains(&St::RoundedMd));
        assert!(tokens.contains(&St::OverflowXAuto));
        assert!(tokens.contains(&St::BorderSubtle));
    }

    #[test]
    fn test_code_block_code_tokens() {
        let tokens = Code::compute_block_code_tokens();
        assert!(tokens.contains(&St::FontMono));
        assert!(tokens.contains(&St::WhitespacePre));
    }

    #[test]
    fn test_code_with_language() {
        let code = Code::block("fn main() {}").language("rust");
        assert_eq!(code.language.as_deref(), Some("rust"));
    }
}
