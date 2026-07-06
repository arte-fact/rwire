//! The `ChatItem` trait — how an app's transcript items render through the
//! Chat family. Full design rationale: `docs/chat-component-design.md`.
//!
//! The component owns the shell (scroll pinning, windowed history, composer,
//! writing state) and the entry chrome (rail, header, spacing); the app owns
//! what an item IS. Three required methods (`key`/`author`/`body`); everything
//! else defaults. `row()` is the chrome-free escape hatch for system lines,
//! fold markers, and interactive gate rows.

use std::borrow::Cow;

use rwire::{ElementBuilder, ItemRef};

/// Tone for author rails, avatars, and tags.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChatIntent {
    #[default]
    Default,
    /// The counterpart voice (typically the agent/assistant).
    Accent,
    /// De-emphasized (tool chatter, system-ish rows that still carry chrome).
    Muted,
    /// Failures.
    Danger,
}

/// Who authored an entry — drives the rail tone, the avatar slot, and the
/// header name.
pub struct ChatAuthor {
    pub name: Cow<'static, str>,
    pub intent: ChatIntent,
    /// Custom avatar/icon element; `None` renders no avatar slot.
    pub avatar: Option<ElementBuilder>,
}

impl ChatAuthor {
    /// The local user's voice (default tone).
    pub fn user(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            intent: ChatIntent::Default,
            avatar: None,
        }
    }

    /// The counterpart voice (accent tone).
    pub fn agent(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            intent: ChatIntent::Accent,
            avatar: None,
        }
    }

    /// De-emphasize (tool chatter and similar).
    pub fn muted(mut self) -> Self {
        self.intent = ChatIntent::Muted;
        self
    }

    /// Attach an avatar/icon element.
    pub fn avatar(mut self, avatar: ElementBuilder) -> Self {
        self.avatar = Some(avatar);
        self
    }
}

/// A small status/phase chip beside the entry header.
#[derive(Clone, Debug)]
pub struct ChatTag {
    pub label: Cow<'static, str>,
    pub intent: ChatIntent,
}

impl ChatTag {
    pub fn muted(label: impl Into<Cow<'static, str>>) -> Self {
        Self {
            label: label.into(),
            intent: ChatIntent::Muted,
        }
    }
    pub fn accent(label: impl Into<Cow<'static, str>>) -> Self {
        Self {
            label: label.into(),
            intent: ChatIntent::Accent,
        }
    }
    pub fn danger(label: impl Into<Cow<'static, str>>) -> Self {
        Self {
            label: label.into(),
            intent: ChatIntent::Danger,
        }
    }
}

/// Collapsible detail under an entry body, rendered with native
/// `<details>/<summary>` — zero-latency, no server round-trip for the toggle,
/// keyboard/a11y semantics for free. The server still controls the initial
/// state via `open`.
pub struct ChatDetail {
    pub summary: Cow<'static, str>,
    pub body: ElementBuilder,
    pub open: bool,
}

impl ChatDetail {
    /// Collapsed by default (the usual case: full tool results, long logs).
    pub fn closed(summary: impl Into<Cow<'static, str>>, body: ElementBuilder) -> Self {
        Self {
            summary: summary.into(),
            body,
            open: false,
        }
    }

    /// Expanded on first render (e.g. while its content is still streaming).
    pub fn open(summary: impl Into<Cow<'static, str>>, body: ElementBuilder) -> Self {
        Self {
            summary: summary.into(),
            body,
            open: true,
        }
    }
}

/// Per-item render context threaded by [`ChatTranscript`](crate::ChatTranscript).
pub struct ChatItemCtx<T> {
    /// Present when the transcript was built from `iter_with_ref()` — lets a
    /// body bind per-item handlers (`on_ref`) for retry/copy/approve actions
    /// without data-attributes.
    pub item_ref: Option<ItemRef<T>>,
}

/// Implemented by an app's transcript item type so `Chat`/`ChatTranscript`
/// can render it. See the module docs and `docs/chat-component-design.md`.
pub trait ChatItem {
    /// Stable identity — becomes the entry's DOM id (prefix `ce-`), driving
    /// keyed morph reuse and history-window anchors. Must be unique within
    /// the transcript, stable across re-renders, and id-safe (no spaces).
    fn key(&self) -> Cow<'_, str>;

    /// Standard chrome: who authored this entry.
    fn author(&self) -> ChatAuthor;

    /// Entry body — any element tree: markdown, a tool card, a diff, a form.
    fn body(&self, ctx: &ChatItemCtx<Self>) -> ElementBuilder
    where
        Self: Sized;

    /// Preformatted timestamp (the server owns formatting).
    fn time(&self) -> Option<Cow<'_, str>> {
        None
    }

    /// Optional status/phase tag beside the header.
    fn tag(&self) -> Option<ChatTag> {
        None
    }

    /// Optional trailing header element (a status chip, a running dot) pushed to the right edge.
    fn trailing(&self, _ctx: &ChatItemCtx<Self>) -> Option<ElementBuilder>
    where
        Self: Sized,
    {
        None
    }

    /// Optional collapsible detail below the body.
    fn detail(&self, _ctx: &ChatItemCtx<Self>) -> Option<ChatDetail>
    where
        Self: Sized,
    {
        None
    }

    /// Full-row override: bypasses ALL chrome (author/time/tag/body/detail).
    /// For system lines, fold markers, and interactive gate rows.
    fn row(&self, _ctx: &ChatItemCtx<Self>) -> Option<ElementBuilder>
    where
        Self: Sized,
    {
        None
    }

    /// This item is the live/streaming one — the transcript appends a subtle
    /// writing cue to its entry.
    fn streaming(&self) -> bool {
        false
    }

    /// Group with the previous entry (suppress the repeated author header)?
    fn groups_with(&self, _prev: &Self) -> bool {
        false
    }
}
