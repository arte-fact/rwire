# FileEditor — consolidation design (decisions locked 2026-07-07)

Consolidates the editor example's state machine into a **stateful component
kit**. Visual spec (annotated mockup, states, legend):
https://claude.ai/code/artifact/613e032a-dc77-45a6-b88a-8f77ed63cb92

## Shape

- **New crate `libs/rwire-editor`** — depends on rwire + rwire-components +
  rwire-markdown (view mode). Primitives (FileTree, SplitPane, CodeEditor,
  DocumentView, FsSnapshot) stay in rwire-components: claw P5c's read-only
  tree must not need the kit.
- **A new component genre**: apps embed `FileEditorState` in their own state,
  register ONE dispatcher handler, and call `state.apply(workspace, ctx)` —
  the kit's reducer decodes actions (select / toggle / edit / save / resolve /
  managed ops) from the existing param-bytes channel. Fits the
  one-state-per-handler model; examples/editor drops ~240 → ~40 lines.

## Locked scope (v1)

| Decision | Call |
|---|---|
| Keyboard save | **In** — generic `data-save-key` runtime behavior (~120B, sibling of `data-enter-submit`) |
| Managed ops (create/rename/delete) | **In**, behind `.managed(true)`; all IO via `FsSnapshot::resolve` — adoption-complete for claw `files.rs` |
| Conflict resolution | **Reload theirs / Overwrite** banner (mtime-gated, never silent); no merge machinery |
| Autosave | **In — toggleable, ON by default**: status-bar switch backed by `FileEditorState.autosave`; app default via `.autosave_default(bool)`. On: flushes on the debounced path, no Save button. Off: manual Save + ⌘S. ⌘S saves immediately in either mode |
| Binary/image preview | **In** — preview panel instead of the "too large" error |
| Multi-file tabs | **Deferred** — the only item that would change `FileEditorState`'s shape |

## UX refinements (from the mock)

Breadcrumb path toolbar · dirty chip (changed-line count) · segmented
View/Edit · autosave toggle in the status bar (ON default; manual mode restores Save + ⌘S) · tree accent rail + modified dot ·
current-line tint across gutter+code · status bar (lang · lines · changed ·
saved) · **unsaved-changes guard on file switch** (fixes the example silently
discarding edits) · tree folds into `Drawer` on mobile.

## Plan

Scaffold crate → port state machine + managed ops + autosave with tests →
surface per the mock (kbd save, conflict banner, preview) → migrate
examples/editor → claw adoption note. Est. 2–3 days.
