//! Editor example — the FileEditor kit over a sandboxed sample tree.
//!
//! Open http://127.0.0.1:9008: browse `examples/editor/sample/` in a
//! resizable split, view markdown rendered and code highlighted, edit with
//! line numbers and dirty marks, autosave (toggleable in the status bar,
//! Cmd/Ctrl+S saves immediately), managed create/rename/delete, guarded
//! conflicts, and vim-mode editing (status-bar toggle; the interpreter
//! lazy-loads as a runtime extension on first use). Edits write into the sample tree — `git diff` shows what you
//! did; `git checkout` undoes it.
//!
//! The whole integration is this file: embed `FileEditorState`, render
//! `FileEditor`, route one handler to `apply`.

use std::error::Error;
use std::path::PathBuf;
use std::sync::OnceLock;

use async_std::main;
use rwire::capsule_gen::CapsuleConfig;
use rwire::style_tokens::St;
use rwire::theme::Theme;
use rwire::{el, handler, renderer, theme, El, ElementBuilder, Server, State};
use rwire_components::{FsSnapshot, Text};
use rwire_editor::{FileEditor, FileEditorState};
use rwire_themes::palettes;

fn workspace() -> &'static FsSnapshot {
    static SNAP: OnceLock<FsSnapshot> = OnceLock::new();
    SNAP.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sample");
        FsSnapshot::scan(&root, 8).expect("scan sample tree")
    })
}

#[derive(State, Default)]
#[storage(memory)]
struct App {
    editor: FileEditorState,
}

#[handler]
fn editor_event(app: &mut App, ctx: &rwire::EventContext) {
    app.editor.apply(workspace(), ctx);
}

#[theme]
fn app_theme() -> Theme {
    Theme::dark().palette(palettes::nord())
}

#[main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!(
        "editor on http://127.0.0.1:9008 — {} entries under sample/",
        workspace().entries.len()
    );
    Server::bind("0.0.0.0:9008")?
        .root(app)
        .capsule_config(CapsuleConfig::new())
        .theme(app_theme())
        .run()
        .await
}

fn app() -> ElementBuilder {
    el(El::Div)
        .st([
            St::DisplayFlex,
            St::FlexCol,
            St::HDvh,
            St::BgApp,
            St::MinH0,
            St::PMd,
            St::GapSm,
        ])
        .append([
            el(El::Header)
                .st([St::DisplayFlex, St::ItemsCenter, St::GapSm, St::FlexShrink0])
                .append([
                    Text::heading3("rwire editor").build(),
                    el(El::Span)
                        .st([St::TextXs, St::TextMuted])
                        .text("FileEditor kit · autosave · managed · sandboxed"),
                ]),
            // The synced wrapper is the flex item: without these tokens the
            // region collapses instead of filling the page.
            render_editor().st([St::Flex1, St::MinH0, St::DisplayFlex, St::FlexCol]),
        ])
}

#[renderer]
fn render_editor(app: &App) -> ElementBuilder {
    FileEditor::new(&app.editor, workspace())
        .on_event(editor_event())
        .managed(true)
        .build()
}
