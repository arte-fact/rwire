//! Chat component (macro).
//!
//! The full chat surface: a bottom-pinned scrolling transcript over a pinned
//! composer that reserves its own height — the composer can never cover the
//! last turn (the layout lesson encoded in claw-rwire's chat). Compose from
//! [`ChatTranscript`] + [`Composer`](crate::Composer); an error banner slot
//! sits between them.
//!
//! ```ignore
//! Chat::new(
//!     ChatTranscript::new()
//!         .items(state.messages.iter_with_ref())
//!         .on_load_older(load_older(), state.window_start as u32, state.window_start > 0)
//!         .writing(state.sending.then(|| "claw is typing…".into()))
//!         .build(),
//! )
//! .error(state.error.as_deref().map(error_banner))
//! .composer(
//!     Composer::new()
//!         .id(format!("composer-{}", state.messages.len()))
//!         .placeholder("Message…")
//!         .on_submit(send_message())
//!         .build(),
//! )
//! .build()
//! ```

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder};

use crate::ChatScroll;

/// Chat builder.
pub struct Chat {
    transcript: ElementBuilder,
    composer: Option<ElementBuilder>,
    error: Option<ElementBuilder>,
}

impl Chat {
    /// Start from a built transcript (usually [`ChatTranscript::build`]).
    pub fn new(transcript: ElementBuilder) -> Self {
        Self {
            transcript,
            composer: None,
            error: None,
        }
    }

    /// The message-entry bar (usually a built [`Composer`](crate::Composer)).
    pub fn composer(mut self, composer: ElementBuilder) -> Self {
        self.composer = Some(composer);
        self
    }

    /// Error banner between transcript and composer (pass `None` to clear).
    pub fn error(mut self, error: Option<ElementBuilder>) -> Self {
        self.error = Some(error.unwrap_or_else(|| el(El::Span).st([St::DisplayNone])));
        self
    }

    /// Build into an ElementBuilder: a column where the scroller flexes and
    /// the composer keeps its own height.
    pub fn build(self) -> ElementBuilder {
        let mut column = vec![ChatScroll::new(self.transcript).build()];
        if let Some(error) = self.error {
            column.push(error);
        }
        if let Some(composer) = self.composer {
            column.push(
                el(El::Div)
                    .st([St::FlexShrink0, St::PtSm])
                    .append([composer]),
            );
        }
        el(El::Div)
            .st([St::DisplayFlex, St::FlexCol, St::Flex1, St::MinH0])
            .append(column)
    }
}
