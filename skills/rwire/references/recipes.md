# rwire — recipes

Seven shapes that come up in every app. Each one names the real API and the trap it avoids.

## 1. Filterable list

Filter **on the server**: the filter is state, the list is a renderer. There is no client-side
filtering model and no place to put one.

```rust
#[derive(State, Default)]
#[storage(memory)]
struct ListState { items: Vec<Item>, query: String, only_open: bool }

#[handler]
fn set_query(state: &mut ListState, ctx: &EventContext) {
    state.query = ctx.text().unwrap_or_default().to_string();
}

#[renderer]
fn rows(state: &ListState) -> ElementBuilder {
    el(El::Ul).append(
        state.items.iter_with_ref()
            .filter(|(_, item)| item.matches(&state.query) && (!state.only_open || item.open))
            .map(|(r, item)| el(El::Li).key(item.id).text(&item.label).on_ref(Ev::Click, open(), r))
    )
}
```

- `.on_debounced(Ev::Input, set_query(), 200)` on the search box — one round-trip per 200 ms of
  typing, not per keystroke.
- `.key(item.id)` on each row: on a reorder, keyed children **move** instead of being recreated,
  so input and focus survive.
- Row identity for the handler is `ItemRef` (`iter_with_ref` + `on_ref`, read back with
  `ctx.item_ref::<T>()`) — not a `data-*` index string.
- A filter that needs its own generated CSS is the single named exception to the no-free-CSS
  rule (`fn filter_css(`, budget 1, see `components-theme.md` §5). Anything beyond that is a
  bypass.

## 2. Form

Two forms, pick one.

**Server-handled** (`rwire::form`) when the payload is a plain set of fields:

```rust
Form::new()
    .action(submit())
    .fields([
        Field::text("name").label("Name").required(),
        Field::email("email").label("Email").required(),
        Field::password("password").min_length(12),
        Field::textarea("notes"),
        Field::submit("Save"),
    ])
```

`Field::{text, password, email, number, hidden, checkbox, textarea, submit}`, with
`.label()`, `.placeholder()`, `.required()`, `.min_length()`, `.max_length()`, `.pattern()` —
`Field::constraints()` (`libs/rwire/src/form.rs`) turns these into native HTML attributes, plus
`Min` / `Max` on number and range fields. That is all the client-side validation there is:
`ValidationRule::{Email, Url, Custom}` have no effect in the markup, and the `FORM_*` opcodes
(`0x60`–`0x63`) exist in the protocol but nothing emits or executes them. Validate in the submit
handler and render the error yourself.

`Field::textarea` carries a stable id (`field-<name>`), so typed text survives re-renders — that
is exactly why textareas in a server-handled form don't lose what you typed.

**Component-built** (`FormField` + `Input` / `Textarea` / `Select` / `Checkbox` / `Switch`)
when you need the fuller control set and your own layout.

Either way: a submit handler that does network work calls `ctx.defer()` and resolves later, so
the submit button spins and reports its own failure (`recipes.md` §4). Never block a handler.

## 3. Panel / overlay

| What you want | API |
|---|---|
| modal dialog | `Modal::new().title(…).open(state.open).on_close(close()).content(…).footer(…)` |
| side panel | `Drawer::new().title(…).position(…).open(…).on_close(…).content(…)` |
| menu on a trigger | `DropdownMenu` |
| collapsible section | `Accordion` |
| two resizable panes | `SplitPane` |

For pure show/hide, do **not** round-trip: `Target` (a named boolean) and `Selector` (a named
enum with one active variant) give zero-latency client-side toggling using `St` classes
(`libs/rwire/src/action.rs`):

```rust
#[derive(Target)]
pub struct PanelOpen;

el(El::Button).toggle::<PanelOpen>(Ev::Click)
el(El::Div).when::<PanelOpen>(St::DisplayBlock).st([St::DisplayNone])
```

`Selector` adds `when_eq(Variant, st)` and `select(Variant, ev)`; `toggle_timed(ev, ms)` gives a
self-closing toast.

Use server state when the panel's **content** depends on state or on who is asking; use
Target/Selector when only its visibility changes. A panel whose content is per-user must be
server-rendered — a Target can only show what the server already sent.

## 4. Refresh after an action

The rule: a handler mutates state, renderers re-run, the diff goes on the wire. Nothing else.

When the work is asynchronous (a POST, a poll, a subprocess):

```rust
#[handler]
fn save(state: &mut App, ctx: &EventContext) {
    let resolver = ctx.defer();
    let payload = state.draft.clone();
    tokio::spawn(async move {
        match client.post(url).json(&payload).timeout(Duration::from_secs(10)).send().await {
            Ok(r) if r.status().is_success() => resolver.ok(),
            Ok(r) => resolver.err(format!("refused ({})", r.status())),
            Err(e) => resolver.err(format!("unreachable: {e}")),
        }
    });
}
```

- `ctx.defer()` → `Resolver`; `ok()` clears the spinner, `err(msg)` turns the control
  destructive with the message. Identity is keyless `(session, handler_id, param)`.
- A handler that returns without deferring never spins — that is how you opt out.
- A worker that never resolves is failed after 30 s (`LOADING_TIMEOUT`), so a control cannot
  spin forever.
- Background code that is not a handler reaches the UI through
  `SharedServerState::{update_shared, update_shared_if}` taken with `server.shared_state()`
  before `run()`; `update_shared_if` broadcasts only when your closure returns
  `Some(ChangeSet)` — deciding what changed is your job — and is sync, callable from any thread.
- Page-level error surfaces: `ActionErrors` (inline) / `ActionToasts` (toast).

## 5. Navigate after creating something

The URL is often only known once the handler has run — a new item's id, the session you just
picked. Do not build the link client-side and do not redirect from the renderer:

```rust
#[handler]
fn create(state: &mut App, ctx: &EventContext) {
    let id = state.create_item();      // your own method: the id exists only now
    ctx.navigate(format!("/items/{id}"));   // pushed; Back returns here
}
```

- `ctx.navigate(url)` pushes a history entry; `ctx.replace_route(url)` swaps the current one (use
  it after a rename that should not add a step to Back).
- Both are queued and flushed **after** the handler's DOM update reaches that connection, so the
  page you land on is already rendered from the state you just changed — no flash of the old page.
- In markup, link with `rwire::Link::to(href, text)` (or `Link::to_with_content(href, el)`, or the bare
`Link::route(href)` you fill yourself) — these carry `data-route`, which the runtime intercepts.
Not `rwire_components::Link`: it renders a plain `<a href>`, a full reload: it drops the
  socket and the session's in-memory state.
- Never navigate from a `#[renderer]`: renderers re-run whenever a field they read changes, so
  a navigation there would fire on unrelated updates.

## 6. Network errors and reconnection

The client half is already written; know what it does so you don't duplicate it
(`runtime/src/connect.ts`):

- **Reconnect** with exponential backoff, capped at 30 s.
- On reopen, the client **wipes everything the server owns** and resets its client state; the
  server replays the page onto a clean slate. Name maps survive on purpose (they are per
  connection, and `MAP_DEF` is re-delivered).
- After **2 failed retries**, the client reloads the page once `/ready` answers — an escape
  hatch for a deploy that shipped a new capsule. `/ready` and `/health` are served by the
  framework (`server.rs`).
- An **offline overlay** with a Retry button. The `online` event and tab visibility reconnect
  immediately instead of waiting out the backoff; `offline` only shows the overlay
  (`runtime/src/connect.ts`).

What that means for your handler code: a dropped socket is not an error you report, it is a
reconnect you survive. But a **server-side outbound call** is yours, and it must have a timeout
— see `pitfalls.md` §1 for the failure that costs. Report it through `Resolver::err`, not by
blocking the handler.

## 7. Mobile and touch targets

- Breakpoints: `.sm()` 640px, `.md()` 768px, `.lg()` 1024px, `.xl()` 1280px (min-width).
  `St` tokens inside; nothing else.
- Layout adapts by tokens, not by a second page: `St::DisplayNone` + `.md([St::DisplayFlex])`.
- **Touch targets**: the theme already gives buttons, inputs, selects, tabs and `[data-touch]`
  elements a 44 px minimum under a coarse pointer or below `Theme::touch_below(px)` (default 640,
  the `sm` breakpoint; the mcp-agents front sets 760). `Button::icon_only` squares itself; use
  `Link::touch` for a link that acts as a control.
- Dynamic viewport, never raw `100vh`: `St::HDvh` / `St::MinHDvh`.
- `ChatScroll` for anything bottom-pinned — `column-reverse` plus an auto bottom margin. Never
  `justify-content` on the scroll container (see `pitfalls.md`).
