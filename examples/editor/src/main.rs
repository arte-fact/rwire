//! Editor example — file explorer + view/edit over a sandboxed sample tree.
//!
//! Open http://127.0.0.1:9008: browse `examples/editor/sample/` in a
//! resizable split (drag the divider — the `BIND_RESIZE` primitive), view
//! markdown rendered and code highlighted, then Edit: line numbers, per-line
//! dirty marks as you type (server-side diff on the debounced input), and a
//! Save button gated on the dirty state. Saving checks the file's mtime and
//! refuses to clobber external changes. Edits write into the sample tree —
//! `git diff` shows what you did; `git checkout` undoes it.

use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::SystemTime;

use async_std::main;
use rwire::capsule_gen::CapsuleConfig;
use rwire::style_tokens::St;
use rwire::theme::Theme;
use rwire::{el, handler, renderer, theme, El, ElementBuilder, Server, State};
use rwire_components::{Chip, CodeEditor, DocumentView, FileTree, FsSnapshot, SplitPane, Text};
use rwire_markdown::{highlight_code, Markdown};
use rwire_themes::palettes;

const MAX_FILE_BYTES: u64 = 256 * 1024;

/// One snapshot per server run; per-connection state holds only view state.
fn snapshot() -> &'static FsSnapshot {
    static SNAP: OnceLock<FsSnapshot> = OnceLock::new();
    SNAP.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sample");
        FsSnapshot::scan(&root, 8).expect("scan sample tree")
    })
}

#[derive(State, Default)]
#[storage(memory)]
struct Ed {
    selected: Option<usize>,
    editing: bool,
    /// Content at open/last save — the dirty diff's baseline.
    baseline: String,
    /// Live working copy, maintained by the debounced input handler.
    working: String,
    /// The file's mtime at open/save; save refuses if disk moved past it.
    opened_mtime: Option<SystemTime>,
    error: Option<String>,
}

fn mtime(path: &PathBuf) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

// ============================================================================
// Handlers
// ============================================================================

#[handler]
fn select_file(ed: &mut Ed, ctx: &rwire::EventContext) {
    let Some(idx) = ctx.item_index() else { return };
    let snap = snapshot();
    let Some(entry) = snap.entries.get(idx) else {
        return;
    };
    if entry.is_dir {
        return;
    }
    ed.error = None;
    let Some(path) = snap.resolve(&entry.rel) else {
        ed.error = Some("path escapes the sandbox".into());
        return;
    };
    if fs::metadata(&path).map(|m| m.len()).unwrap_or(u64::MAX) > MAX_FILE_BYTES {
        ed.error = Some("file too large for the demo editor".into());
        return;
    }
    match fs::read_to_string(&path) {
        Ok(content) => {
            ed.selected = Some(idx);
            ed.baseline = content.clone();
            ed.working = content;
            ed.opened_mtime = mtime(&path);
            ed.editing = false;
        }
        Err(e) => ed.error = Some(format!("read failed: {e}")),
    }
}

#[handler]
fn toggle_edit(ed: &mut Ed) {
    if ed.selected.is_some() {
        ed.editing = !ed.editing;
    }
}

#[handler]
fn edit_input(ed: &mut Ed, ctx: &rwire::EventContext) {
    if let Some(text) = ctx.text() {
        ed.working = text.to_string();
    }
}

#[handler]
fn save(ed: &mut Ed) {
    let Some(idx) = ed.selected else { return };
    let snap = snapshot();
    let Some(entry) = snap.entries.get(idx) else {
        return;
    };
    let Some(path) = snap.resolve(&entry.rel) else {
        return;
    };
    // Conflict gate: never silently clobber an external change.
    if mtime(&path) != ed.opened_mtime {
        ed.error = Some("file changed on disk since you opened it — not saving".into());
        return;
    }
    match fs::write(&path, &ed.working) {
        Ok(()) => {
            ed.baseline = ed.working.clone();
            ed.opened_mtime = mtime(&path);
            ed.error = None;
        }
        Err(e) => ed.error = Some(format!("save failed: {e}")),
    }
}

// ============================================================================
// UI
// ============================================================================

#[theme]
fn app_theme() -> Theme {
    Theme::dark().palette(palettes::nord())
}

#[main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!(
        "editor on http://127.0.0.1:9008 — {} entries under sample/",
        snapshot().entries.len()
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
                        .text("sandboxed · dirty-tracked · conflict-safe"),
                ]),
            render_explorer(),
        ])
}

fn dirty_flags(baseline: &str, working: &str) -> Vec<bool> {
    let base: Vec<&str> = baseline.lines().collect();
    working
        .lines()
        .enumerate()
        .map(|(i, line)| base.get(i).map(|b| *b != line).unwrap_or(true))
        .collect()
}

fn lang_of(rel: &str) -> Option<&str> {
    rel.rsplit_once('.').map(|(_, ext)| ext)
}

#[renderer]
fn render_explorer(ed: &Ed) -> ElementBuilder {
    let snap = snapshot();
    let tree = FileTree::new(&snap.entries)
        .selected(ed.selected)
        .on_select(select_file())
        .expand_all()
        .build();

    let doc: ElementBuilder = match ed.selected {
        None => el(El::Div)
            .st([
                St::DisplayFlex,
                St::ItemsCenter,
                St::JustifyCenter,
                St::Flex1,
                St::TextMuted,
            ])
            .text("Select a file to view it."),
        Some(idx) => {
            let entry = &snap.entries[idx];
            let dirty = dirty_flags(&ed.baseline, &ed.working);
            let is_dirty = ed.working != ed.baseline;
            let body: ElementBuilder = if ed.editing {
                CodeEditor::new("ed-field", &ed.working)
                    .dirty_lines(&dirty)
                    .on_edit(edit_input())
                    .save_bar(save(), is_dirty)
                    .build()
            } else if lang_of(&entry.rel) == Some("md") {
                Markdown::new(ed.working.clone()).build()
            } else {
                highlight_code(&ed.working, lang_of(&entry.rel))
            };
            let mut view = DocumentView::new(entry.rel.clone(), body).action(
                Chip::new(if ed.editing { "View" } else { "Edit" })
                    .active(ed.editing)
                    .on_click(toggle_edit())
                    .build(),
            );
            if ed.editing {
                view = view.editor_body();
            }
            if let Some(err) = &ed.error {
                view = view.action(el(El::Span).st([St::TextXs, St::TextError]).text(err));
            }
            view.build()
        }
    };

    SplitPane::new(
        tree,
        el(El::Div)
            .st([St::PlMd, St::DisplayFlex, St::FlexCol, St::Flex1, St::MinH0])
            .append([doc]),
    )
    .initial("16rem")
    .build()
    .st([St::Flex1, St::MinH0])
}
