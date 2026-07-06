//! StreamedContent component.
//!
//! Progressive delivery for large content (files, long documents, list pages):
//! render only the chunks delivered so far, followed by a **sentinel row** that
//! fires `on_load_more` when it nears the viewport (the runtime's one-shot
//! `BIND_SENTINEL` primitive). The handler grows the delivered count in state;
//! the re-render appends the next chunk and a re-keyed sentinel — so exactly
//! one request is in flight, structurally, and loading continues until content
//! outruns the viewport.
//!
//! Each chunk is wrapped in a stable-id `<div>` (`{id}-{index}`), so the morph
//! reuses every already-rendered chunk and the update cost stays at "one new
//! chunk", not "re-render everything".
//!
//! # Example
//!
//! ```ignore
//! use rwire_components::StreamedContent;
//!
//! #[renderer]
//! fn render_doc(state: &DocState) -> ElementBuilder {
//!     StreamedContent::new("doc")
//!         .chunks(state.lines[..state.delivered].chunks(100).map(render_chunk))
//!         .load_more(load_more(), state.delivered as u32, state.delivered < state.total)
//!         .build()
//! }
//!
//! #[handler]
//! fn load_more(state: &mut DocState, ctx: &EventContext) {
//!     // The sentinel's param is the chunk index it was armed with — ignore
//!     // stale fires (reconnects, morph races) instead of double-appending.
//!     if ctx.item_index() == Some(state.delivered) {
//!         state.delivered = (state.delivered + CHUNK).min(state.total);
//!     }
//! }
//! ```

use std::borrow::Cow;

use rwire::style_tokens::St;
use rwire::{el, El, ElementBuilder, HandlerSpec};

use crate::{Spinner, SpinnerSize};

/// StreamedContent builder.
pub struct StreamedContent {
    id: Cow<'static, str>,
    chunks: Vec<ElementBuilder>,
    load_more: Option<(HandlerSpec, u32)>,
    gap: Option<St>,
}

impl StreamedContent {
    /// Create a streamed region. `id` prefixes each chunk's stable DOM id —
    /// keep it unique per page so morphs never cross-match chunks.
    pub fn new(id: impl Into<Cow<'static, str>>) -> Self {
        Self {
            id: id.into(),
            chunks: Vec::new(),
            load_more: None,
            gap: None,
        }
    }

    /// The chunks delivered so far, oldest first. Wrapped in stable-id divs.
    pub fn chunks<I>(mut self, chunks: I) -> Self
    where
        I: IntoIterator<Item = ElementBuilder>,
    {
        self.chunks.extend(chunks);
        self
    }

    /// Arm the sentinel: when `has_more`, a spinner row fires `handler` with
    /// `next` (the index to deliver next — read via `ctx.item_index()`) as it
    /// nears the viewport. Pass the CURRENT delivered count as `next` on every
    /// render; the changing value re-keys the sentinel so each response arms a
    /// fresh observer.
    pub fn load_more(mut self, handler: HandlerSpec, next: u32, has_more: bool) -> Self {
        self.load_more = if has_more {
            Some((handler, next))
        } else {
            None
        };
        self
    }

    /// Vertical gap token between chunks (e.g. `St::GapMd`).
    pub fn gap(mut self, gap: St) -> Self {
        self.gap = Some(gap);
        self
    }

    /// Build into an ElementBuilder.
    pub fn build(self) -> ElementBuilder {
        let id = self.id;
        let mut root = el(El::Div).st([St::DisplayFlex, St::FlexCol, St::MinW0]);
        if let Some(gap) = self.gap {
            root = root.st([gap]);
        }
        root = root.append(self.chunks.into_iter().enumerate().map(|(i, chunk)| {
            el(El::Div)
                .id(format!("{id}-{i}").as_str())
                .st([St::MinW0])
                .append([chunk])
        }));
        if let Some((handler, next)) = self.load_more {
            root = root.append([el(El::Div)
                .id(format!("{id}-sentinel").as_str())
                .st([St::DisplayFlex, St::JustifyCenter, St::PMd])
                .on_visible(handler, next)
                .append([Spinner::new().size(SpinnerSize::Sm).build()])]);
        }
        root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rwire::builder::BuildContext;
    use rwire::{ChangeSet, HandlerSpec};

    #[derive(rwire::State, Default)]
    #[storage(memory)]
    struct S;

    fn handler() -> HandlerSpec {
        fn noop(_s: &mut S) {}
        HandlerSpec::from_fn_with_changes::<S>(noop, ChangeSet::all())
            .with_handler_id(rwire::stable_handler_id("streamed_content_tests", "noop"))
    }

    fn emit(root: &ElementBuilder) -> Vec<u8> {
        let mut ctx = BuildContext::new();
        let state: S = S;
        ctx.collect_symbols(root, &state);
        ctx.emit(root, &state);
        ctx.finish().to_vec()
    }

    fn chunk(i: usize) -> ElementBuilder {
        el(El::P).text(format!("chunk {i}").as_str())
    }

    fn stream(has_more: bool) -> Vec<u8> {
        emit(
            &StreamedContent::new("doc")
                .chunks((0..3).map(chunk))
                .load_more(handler(), 3, has_more)
                .build(),
        )
    }

    /// Count BIND_SENTINEL opcodes. 0x4F can also appear inside symbol text,
    /// so compare armed vs unarmed streams rather than asserting absolutes.
    fn sentinel_bytes(bytes: &[u8]) -> usize {
        bytes.iter().filter(|&&b| b == 0x4F).count()
    }

    #[test]
    fn sentinel_armed_while_more_absent_when_complete() {
        let armed = stream(true);
        let done = stream(false);
        assert!(
            sentinel_bytes(&armed) > sentinel_bytes(&done),
            "armed stream must carry the BIND_SENTINEL opcode"
        );
        assert!(
            done.len() < armed.len(),
            "completed stream must not render the sentinel row"
        );
    }
}
