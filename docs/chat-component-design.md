# Chat component design: the `ChatItem` trait

Draft (2026-07-06) for RELEASE_ROADMAP.md **F7**. How a generic `Chat` renders
app-specific content — designed against claw-rwire's agent threads, which are the
hardest known consumer, without hardcoding any of their semantics.

## The problem

A chat transcript is not a list of text bubbles. claw-rwire's surfaces interleave
(from `state/views.rs`):

| Item | Needs |
|---|---|
| `ChatMessage { role, text, time }` | standard authored turn, markdown body |
| `ToolCardView { name, arguments, result, ok, time }` | status-tinted card, full result behind a disclosure |
| `AgentThreadItem::Memory { time, note }` | one muted line, **no chrome** (no author/rail/avatar) |
| `AgentThreadItem::ReviewGate { task_id }` | **interactive** row — approve/reject handlers |
| fold marker ("older messages condensed into memory") | muted divider line |
| live streaming turn (`live_turns`, token flush) | body grows as tokens land; cursor/pulse |

The component owns the *shell* (scroll pinning, windowing + seamless history,
composer, writing state) and the *entry chrome* (rail, header, spacing, alignment).
The app owns *what an item is and how its content renders*. The boundary is a trait.

## The trait

```rust
/// Implemented by an app's transcript item type so `Chat`/`ChatTranscript` can
/// render it. Three required methods; everything else defaults.
pub trait ChatItem {
    /// Stable identity — drives keyed morphing and history-window anchors.
    /// Must be unique within the transcript and stable across re-renders.
    fn key(&self) -> Cow<'_, str>;

    /// Standard chrome: who authored this entry.
    fn author(&self) -> ChatAuthor;

    /// Entry body — any element tree: markdown, a tool card, a diff, a form.
    fn body(&self, ctx: &ChatItemCtx<Self>) -> ElementBuilder
    where Self: Sized;

    /// Preformatted timestamp (the server owns formatting; no client locale logic).
    fn time(&self) -> Option<Cow<'_, str>> { None }

    /// Optional status/phase tag beside the header (renders as a `Chip`/`Badge`).
    fn tag(&self) -> Option<ChatTag> { None }

    /// Optional collapsible detail below the body — rendered with native
    /// `<details>`/`<summary>` (see "Collapse" below).
    fn detail(&self, ctx: &ChatItemCtx<Self>) -> Option<ChatDetail>
    where Self: Sized { None }

    /// Full-row override: bypasses ALL chrome. For system lines, fold markers,
    /// and interactive gate rows. `Some(_)` suppresses author/time/tag/body/detail.
    fn row(&self, ctx: &ChatItemCtx<Self>) -> Option<ElementBuilder>
    where Self: Sized { None }

    /// Group with the previous entry (suppress the repeated author header)?
    fn groups_with(&self, prev: &Self) -> bool { false }
}
```

Supporting types (component-provided):

```rust
pub struct ChatAuthor {
    pub name: Cow<'static, str>,
    pub intent: ChatIntent,              // Default | Accent | Muted | Danger — rail/avatar tone
    pub avatar: Option<ElementBuilder>,  // Avatar, icon(), anything — None = initial letter
}

pub struct ChatTag {
    pub label: Cow<'static, str>,
    pub intent: ChatIntent,              // Tool ok/err → Accent/Danger, phases → Muted…
}

pub struct ChatDetail {
    pub summary: Cow<'static, str>,      // the <summary> line ("result · 2.1 KB")
    pub body: ElementBuilder,
    pub open: bool,                      // server-controlled initial state
}

/// Per-item render context, threaded by the transcript.
pub struct ChatItemCtx<T> {
    /// Present when the transcript was built from `iter_with_ref()` — lets a body
    /// bind per-item handlers (`on_ref`) for retry/copy/approve without data-attrs.
    pub item_ref: Option<ItemRef<T>>,
    /// True for the live entry — body may render a cursor/pulse; the component
    /// shows `TypingIndicator` when the body is still empty.
    pub streaming: bool,
}
```

Consumption:

```rust
ChatTranscript::from_items(state.items.iter_with_ref())   // ctx.item_ref = Some(_)
ChatTranscript::from_iter(items.iter())                    // ctx.item_ref = None
    .has_older(state.window_start > 0)
    .on_load_older(load_older())                           // fired by the F1 sentinel
```

## Why a trait (not closure slots)

1. **Multiple surfaces, one impl.** claw P4a renders the *same* item types in three
   places (concierge chat, agent thread, task Thread tab). `impl ChatItem for
   TranscriptItem` once; every surface gets it. Closure slots would be copied or
   plumbed three times.
2. **Progressive disclosure.** A plain chatroom implements 3 methods. claw implements
   6. The defaults are the documentation of what's optional.
3. **`ChatEntry` stays à la carte.** The trait is an adapter *onto* the molecule, not
   a cage — apps can always hand-build rows and skip the trait entirely.

## Collapse: native `<details>`, not server toggles

claw's collapsibles round-trip a `toggled: &[String]` list through server state.
First-class bar says do better: render `ChatDetail` as `<details><summary>` —
zero-latency, zero server state, correct keyboard/a11y semantics for free, and the
server can still set the initial `open` attribute (or force it during streaming).

**Sub-task:** `El::Details` / `El::Summary` don't exist yet — add per the standard
new-element recipe (opcodes.rs variant + capsule mapping). Trivial; do it with F7.

## The claw adoption sketch (validation, not spec)

```rust
impl ChatItem for TranscriptItem {
    fn key(&self) -> Cow<'_, str> { /* message id / tool call id */ }

    fn author(&self) -> ChatAuthor {
        match self {
            Self::Message(m) if m.role == MessageRole::User => ChatAuthor::user("you"),
            Self::Message(_) => ChatAuthor::agent("claw"),
            Self::Tool(_)    => ChatAuthor::agent("claw").muted(),
        }
    }

    fn tag(&self) -> Option<ChatTag> {
        match self {
            Self::Tool(t) if t.ok  => Some(ChatTag::muted(&t.name)),
            Self::Tool(t)          => Some(ChatTag::danger(&t.name)),
            _ => None,
        }
    }

    fn body(&self, _cx: &ChatItemCtx<Self>) -> ElementBuilder {
        match self {
            Self::Message(m) => markdown(&m.text),
            Self::Tool(t)    => code_line(&t.arguments),
        }
    }

    fn detail(&self, _cx: &ChatItemCtx<Self>) -> Option<ChatDetail> {
        match self {
            Self::Tool(t) => Some(ChatDetail::closed("result", pre(&t.result))),
            _ => None,
        }
    }
}
```

`AgentThreadItem::Memory`/`ReviewGate` and the fold marker go through `row()` —
chrome-free muted lines, and a gate row whose approve/reject buttons bind via
`ctx.item_ref`.

## Open questions (settle during F7 implementation)

- **Alignment mode**: chat-style (user entries end-aligned) vs thread-style (all
  start-aligned with rails) — a transcript-level layout switch
  (`ChatTranscript::aligned()` / `::railed()`), not a per-item concern. Default TBD.
- **Day dividers**: component inserts them from a `fn day(&self) -> Option<...>`
  default method, or apps insert divider items via `row()`? Leaning `row()` — less
  trait surface.
- **`groups_with` default**: opt-in (current draft) or author-equality by default?
  Opt-in is predictable; revisit after the chatroom example exercises it.
