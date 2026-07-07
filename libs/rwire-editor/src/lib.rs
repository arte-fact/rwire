//! FileEditor — a stateful component kit for rwire: a sandboxed file
//! explorer/editor with dirty tracking, toggleable autosave (on by default),
//! guarded saves, managed file operations, and image previews.
//!
//! Design: `docs/file-editor-design.md` (decisions locked 2026-07-07).
//!
//! ```ignore
//! #[derive(State, Default)]
//! #[storage(memory)]
//! struct App { editor: FileEditorState }
//!
//! #[renderer]
//! fn files(app: &App) -> ElementBuilder {
//!     FileEditor::new(&app.editor, workspace())
//!         .on_event(editor_event())
//!         .managed(true)
//!         .build()
//! }
//!
//! #[handler]
//! fn editor_event(app: &mut App, ctx: &EventContext) {
//!     app.editor.apply(workspace(), ctx);
//! }
//! ```

mod state;
mod surface;

pub use state::{Action, FileEditorState, FileKind, Pending, MAX_PREVIEW_BYTES, MAX_TEXT_BYTES};
pub use surface::FileEditor;
