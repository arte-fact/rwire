//! DocumentView component — the view/edit shell over a document: a header
//! (title + actions slot) above a scrolling body (a rendered view, or an
//! editor when the app switches modes).

use std::borrow::Cow;

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};

/// DocumentView builder.
pub struct DocumentView {
    title: Cow<'static, str>,
    actions: Vec<ElementBuilder>,
    body: ElementBuilder,
    /// Editors manage their own scrolling; rendered views scroll here.
    body_scrolls: bool,
}

impl DocumentView {
    pub fn new(title: impl Into<Cow<'static, str>>, body: ElementBuilder) -> Self {
        Self {
            title: title.into(),
            actions: Vec::new(),
            body,
            body_scrolls: true,
        }
    }

    /// Header actions (mode toggle chips, buttons…).
    pub fn action(mut self, action: ElementBuilder) -> Self {
        self.actions.push(action);
        self
    }

    /// The body hosts an editor that scrolls itself (don't double-scroll).
    pub fn editor_body(mut self) -> Self {
        self.body_scrolls = false;
        self
    }

    pub fn build(self) -> ElementBuilder {
        let mut header = vec![el(El::Span)
            .st([
                St::FontSemibold,
                St::TextSm,
                St::TextHigh,
                St::Flex1,
                St::MinW0,
            ])
            .text(self.title.as_ref())];
        header.extend(self.actions);
        let mut body_tokens = vec![
            St::Flex1,
            St::MinH0,
            St::MinW0,
            St::DisplayFlex,
            St::FlexCol,
        ];
        if self.body_scrolls {
            body_tokens.push(St::OverflowAuto);
        }
        el(El::Div)
            .st([
                St::DisplayFlex,
                St::FlexCol,
                St::Flex1,
                St::MinH0,
                St::MinW0,
            ])
            .append([
                el(El::Div)
                    .st([
                        St::DisplayFlex,
                        St::ItemsCenter,
                        St::GapSm,
                        St::PbSm,
                        St::BorderB,
                        St::FlexShrink0,
                    ])
                    .append(header),
                el(El::Div).st(body_tokens).append([self.body]),
            ])
    }
}
