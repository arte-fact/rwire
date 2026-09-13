//! The FileEditor state machine — embed [`FileEditorState`] in your app state,
//! route one dispatcher handler to [`FileEditorState::apply`], and the kit
//! owns selection, the working copy, dirty diffing, autosave, guarded saves,
//! and (when enabled) managed file operations. All IO goes through the sealed
//! [`FsSnapshot`] the app provides.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use rwire::EventContext;
use rwire_components::{FsEntry, FsSnapshot};

/// Files larger than this refuse to open in the text editor (previews have
/// their own cap).
pub const MAX_TEXT_BYTES: u64 = 256 * 1024;
/// Images larger than this show the info card instead of an inline preview.
pub const MAX_PREVIEW_BYTES: u64 = 512 * 1024;

/// Action codes carried in the event param bytes: `[code, index_varint?]`.
/// Encoded by the builder, decoded by [`FileEditorState::apply`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Action {
    Select = 1,
    ToggleMode = 2,
    Edit = 3,
    Save = 4,
    ToggleAutosave = 5,
    ConflictReload = 6,
    ConflictOverwrite = 7,
    GuardDiscard = 8,
    GuardKeep = 9,
    CreateStart = 10,
    CreateSubmit = 11,
    RenameStart = 12,
    RenameSubmit = 13,
    Delete = 14,
    DeleteConfirm = 15,
    CancelOp = 16,
    CreateDirStart = 17,
    Undo = 18,
    Redo = 19,
    ToggleVim = 20,
}

impl Action {
    fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            1 => Self::Select,
            2 => Self::ToggleMode,
            3 => Self::Edit,
            4 => Self::Save,
            5 => Self::ToggleAutosave,
            6 => Self::ConflictReload,
            7 => Self::ConflictOverwrite,
            8 => Self::GuardDiscard,
            9 => Self::GuardKeep,
            10 => Self::CreateStart,
            11 => Self::CreateSubmit,
            12 => Self::RenameStart,
            13 => Self::RenameSubmit,
            14 => Self::Delete,
            15 => Self::DeleteConfirm,
            16 => Self::CancelOp,
            17 => Self::CreateDirStart,
            18 => Self::Undo,
            19 => Self::Redo,
            20 => Self::ToggleVim,
            _ => return None,
        })
    }

    /// Param bytes for this action (+ optional entry index).
    pub fn params(self, index: Option<u32>) -> Vec<u8> {
        let mut out = vec![self as u8];
        if let Some(i) = index {
            let mut v = i;
            // varint, matching protocol/varint.rs widths
            if v < 128 {
                out.push(v as u8);
            } else if v < 16512 {
                v -= 128;
                out.push(0x80 | (v >> 8) as u8);
                out.push((v & 0xFF) as u8);
            } else {
                v -= 16512;
                out.push(0xC0 | (v >> 16) as u8);
                out.push(((v >> 8) & 0xFF) as u8);
                out.push((v & 0xFF) as u8);
            }
        }
        out
    }
}

fn read_varint(b: &[u8]) -> Option<u32> {
    let first = *b.first()?;
    Some(match first {
        0..=0x7F => u32::from(first),
        0x80..=0xBF => 0x80 + ((u32::from(first & 0x3F)) << 8) + u32::from(*b.get(1)?),
        _ => {
            0x4080
                + ((u32::from(first & 0x3F)) << 16)
                + (u32::from(*b.get(1)?) << 8)
                + u32::from(*b.get(2)?)
        }
    })
}

/// What the body should show for the selected file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    Text,
    Image,
    Binary,
}

/// A pending inline prompt/confirmation in the surface.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Pending {
    #[default]
    None,
    /// Manual-mode guard: switching to `to` would discard unsaved lines.
    Guard { to: usize },
    /// The file changed on disk; offer Reload theirs / Overwrite.
    Conflict,
    /// Inline name input for a new file or folder.
    Create { dir: bool },
    /// Inline rename input for entry `index`.
    Rename { index: usize },
    /// Confirm deletion of entry `index`.
    Delete { index: usize },
}

/// The embeddable state. `Default` enables autosave; use
/// [`FileEditorState::manual`] for an off-by-default editor.
pub struct FileEditorState {
    pub entries: Vec<FsEntry>,
    pub selected: Option<usize>,
    pub editing: bool,
    pub autosave: bool,
    /// Vim-mode editing (the lazy `vim` runtime extension). Per-session
    /// preference; off by default.
    pub vim: bool,
    pub kind: FileKind,
    pub baseline: String,
    pub working: String,
    /// Base64 payload for image previews (data URI body).
    pub preview_b64: Option<String>,
    pub pending: Pending,
    pub error: Option<String>,
    /// `HH:MM` of the last successful save (UTC), for the status bar.
    pub saved_at: Option<String>,
    /// Bumped whenever the SERVER changes the working copy (open, undo, redo,
    /// conflict reload) — keys the textarea so the morph replaces the node
    /// and the browser adopts the new content instead of its stale value.
    pub generation: u32,
    /// Caret hint for the re-keyed textarea (UTF-16 units): undo/redo place
    /// the caret at the start of the reverted change, like vim.
    pub caret: Option<usize>,
    undo: Vec<String>,
    redo: Vec<String>,
    opened_mtime: Option<SystemTime>,
    scanned: bool,
}

impl Default for FileEditorState {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            selected: None,
            editing: false,
            autosave: true,
            vim: false,
            kind: FileKind::Text,
            baseline: String::new(),
            working: String::new(),
            preview_b64: None,
            pending: Pending::None,
            error: None,
            saved_at: None,
            generation: 0,
            caret: None,
            undo: Vec::new(),
            redo: Vec::new(),
            opened_mtime: None,
            scanned: false,
        }
    }
}

fn now_hm() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{:02}:{:02}", (secs / 3600) % 24, (secs / 60) % 60)
}

fn mtime(path: &PathBuf) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "svg", "ico", "bmp"];

fn kind_of(rel: &str, bytes_are_text: bool) -> FileKind {
    let ext = rel.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some(e) if IMAGE_EXTS.contains(&e) => FileKind::Image,
        _ if bytes_are_text => FileKind::Text,
        _ => FileKind::Binary,
    }
}

impl FileEditorState {
    /// An editor with autosave off by default (the user can still toggle it on).
    pub fn manual() -> Self {
        Self {
            autosave: false,
            ..Self::default()
        }
    }

    /// Whether an Undo / Redo step exists (for the toolbar affordances).
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Unsaved lines vs the baseline (content diff, not keystrokes).
    pub fn dirty(&self) -> bool {
        self.working != self.baseline
    }

    /// First differing position between two texts in UTF-16 code units —
    /// the browser's selection coordinate space.
    fn first_diff_utf16(a: &str, b: &str) -> usize {
        let mut units = 0;
        let mut bc = b.chars();
        for ca in a.chars() {
            match bc.next() {
                Some(cb) if ca == cb => units += ca.len_utf16(),
                _ => break,
            }
        }
        units
    }

    /// Per-line dirty flags for the gutter.
    pub fn dirty_lines(&self) -> Vec<bool> {
        let base: Vec<&str> = self.baseline.lines().collect();
        self.working
            .lines()
            .enumerate()
            .map(|(i, line)| base.get(i).map(|b| *b != line).unwrap_or(true))
            .collect()
    }

    /// The entries to render: the kit's own (post-managed-op) list once
    /// scanned, else the app's startup snapshot.
    pub fn entries_or<'a>(&'a self, initial: &'a [FsEntry]) -> &'a [FsEntry] {
        if self.scanned {
            &self.entries
        } else {
            initial
        }
    }

    fn rescan(&mut self, snap: &FsSnapshot) {
        match FsSnapshot::scan(snap.root(), 8) {
            Ok(fresh) => {
                self.entries = fresh.entries;
                self.scanned = true;
            }
            Err(e) => self.error = Some(format!("rescan failed: {e}")),
        }
    }

    fn entry<'a>(&'a self, snap: &'a FsSnapshot, index: usize) -> Option<&'a FsEntry> {
        if self.scanned {
            self.entries.get(index)
        } else {
            snap.entries.get(index)
        }
    }

    /// The kit's reducer. Decodes the action from `ctx` params; all IO through
    /// the sealed snapshot.
    pub fn apply(&mut self, snap: &FsSnapshot, ctx: &EventContext) {
        let params = ctx.param_bytes();
        let Some(action) = params.first().copied().and_then(Action::from_u8) else {
            return;
        };
        let index = read_varint(&params[1..]).map(|v| v as usize);
        self.error = None;
        match action {
            Action::Select => {
                let Some(i) = index else { return };
                // Manual-mode guard: don't silently discard unsaved lines.
                if !self.autosave && self.dirty() && self.selected != Some(i) {
                    self.pending = Pending::Guard { to: i };
                    return;
                }
                self.open(snap, i);
            }
            Action::GuardDiscard => {
                if let Pending::Guard { to } = self.pending {
                    self.pending = Pending::None;
                    self.open(snap, to);
                }
            }
            Action::GuardKeep => self.pending = Pending::None,
            Action::ToggleMode => {
                if self.selected.is_some() && self.kind == FileKind::Text {
                    self.editing = !self.editing;
                }
            }
            Action::Edit => {
                if let Some(text) = ctx.text() {
                    if text != self.working {
                        if self.undo.len() >= 100 {
                            self.undo.remove(0);
                        }
                        self.undo.push(std::mem::take(&mut self.working));
                        self.redo.clear();
                        self.working = text.to_string();
                        self.caret = None;
                    }
                    if self.autosave && self.dirty() {
                        self.save(snap);
                    }
                }
            }
            Action::Save => self.save(snap),
            Action::ToggleVim => self.vim = !self.vim,
            Action::ToggleAutosave => {
                self.autosave = !self.autosave;
                if self.autosave && self.dirty() {
                    self.save(snap);
                }
            }
            Action::ConflictReload => {
                if let Some(i) = self.selected {
                    self.pending = Pending::None;
                    self.open(snap, i);
                }
            }
            Action::ConflictOverwrite => {
                self.pending = Pending::None;
                self.write_through(snap);
            }
            Action::Undo => {
                if let Some(prev) = self.undo.pop() {
                    self.caret = Some(Self::first_diff_utf16(&self.working, &prev));
                    self.redo.push(std::mem::replace(&mut self.working, prev));
                    self.generation += 1;
                    if self.autosave {
                        self.save(snap);
                    }
                }
            }
            Action::Redo => {
                if let Some(next) = self.redo.pop() {
                    self.caret = Some(Self::first_diff_utf16(&self.working, &next));
                    self.undo.push(std::mem::replace(&mut self.working, next));
                    self.generation += 1;
                    if self.autosave {
                        self.save(snap);
                    }
                }
            }
            Action::CreateStart => self.pending = Pending::Create { dir: false },
            Action::CreateDirStart => self.pending = Pending::Create { dir: true },
            Action::CreateSubmit => self.create(snap, ctx),
            Action::RenameStart => {
                if let Some(i) = index {
                    self.pending = Pending::Rename { index: i };
                }
            }
            Action::RenameSubmit => self.rename(snap, ctx),
            Action::Delete => {
                if let Some(i) = index {
                    self.pending = Pending::Delete { index: i };
                }
            }
            Action::DeleteConfirm => self.delete(snap),
            Action::CancelOp => self.pending = Pending::None,
        }
    }

    fn open(&mut self, snap: &FsSnapshot, index: usize) {
        let Some(entry) = self.entry(snap, index).cloned() else {
            return;
        };
        if entry.is_dir {
            return;
        }
        let Some(path) = snap.resolve(&entry.rel) else {
            self.error = Some("path escapes the sandbox".into());
            return;
        };
        let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(u64::MAX);
        self.selected = Some(index);
        self.editing = false;
        self.preview_b64 = None;
        self.pending = Pending::None;
        self.undo.clear();
        self.redo.clear();
        self.generation += 1;
        self.caret = None;
        match fs::read(&path) {
            Ok(bytes) => match String::from_utf8(bytes.clone()) {
                Ok(text) if size <= MAX_TEXT_BYTES => {
                    self.kind = FileKind::Text;
                    self.baseline = text.clone();
                    self.working = text;
                    self.opened_mtime = mtime(&path);
                }
                _ => {
                    self.kind = kind_of(&entry.rel, false);
                    self.baseline = String::new();
                    self.working = String::new();
                    if self.kind == FileKind::Image && size <= MAX_PREVIEW_BYTES {
                        self.preview_b64 = Some(base64(&bytes));
                    }
                }
            },
            Err(e) => self.error = Some(format!("read failed: {e}")),
        }
    }

    fn save(&mut self, snap: &FsSnapshot) {
        if !self.dirty() {
            return;
        }
        let Some(path) = self.selected_path(snap) else {
            return;
        };
        if mtime(&path) != self.opened_mtime {
            self.pending = Pending::Conflict;
            return;
        }
        self.write(&path);
    }

    /// Overwrite regardless of disk state (the conflict banner's second action).
    fn write_through(&mut self, snap: &FsSnapshot) {
        if let Some(path) = self.selected_path(snap) {
            self.write(&path);
        }
    }

    fn write(&mut self, path: &PathBuf) {
        match fs::write(path, &self.working) {
            Ok(()) => {
                self.baseline = self.working.clone();
                self.opened_mtime = mtime(path);
                self.saved_at = Some(now_hm());
            }
            Err(e) => self.error = Some(format!("save failed: {e}")),
        }
    }

    fn selected_path(&self, snap: &FsSnapshot) -> Option<PathBuf> {
        let i = self.selected?;
        let entry = self.entry(snap, i)?;
        snap.resolve(&entry.rel)
    }

    fn form_name(ctx: &EventContext) -> Option<String> {
        if let rwire::EventPayload::Form(fields) = ctx.payload() {
            let name = fields.get("name")?.trim();
            // One path segment only: managed ops never traverse.
            if name.is_empty() || name.contains('/') || name.contains('\\') || name.starts_with('.')
            {
                return None;
            }
            return Some(name.to_string());
        }
        None
    }

    fn create(&mut self, snap: &FsSnapshot, ctx: &EventContext) {
        let Pending::Create { dir } = self.pending else {
            return;
        };
        let Some(name) = Self::form_name(ctx) else {
            self.error = Some("names are one plain segment (no /, no leading dot)".into());
            return;
        };
        let Some(path) = snap.resolve(&name) else {
            self.error = Some("path escapes the sandbox".into());
            return;
        };
        if path.exists() {
            self.error = Some(format!("{name} already exists"));
            return;
        }
        let result = if dir {
            fs::create_dir(&path)
        } else {
            fs::write(&path, "")
        };
        match result {
            Ok(()) => {
                self.pending = Pending::None;
                self.rescan(snap);
                if !dir {
                    if let Some(i) = self.index_of(&name) {
                        self.open(snap, i);
                        self.editing = true;
                    }
                }
            }
            Err(e) => self.error = Some(format!("create failed: {e}")),
        }
    }

    fn rename(&mut self, snap: &FsSnapshot, ctx: &EventContext) {
        let Pending::Rename { index } = self.pending else {
            return;
        };
        let Some(name) = Self::form_name(ctx) else {
            self.error = Some("names are one plain segment (no /, no leading dot)".into());
            return;
        };
        let Some(entry) = self.entry(snap, index).cloned() else {
            return;
        };
        let Some(from) = snap.resolve(&entry.rel) else {
            return;
        };
        let to_rel = match entry.rel.rsplit_once('/') {
            Some((dir, _)) => format!("{dir}/{name}"),
            None => name.clone(),
        };
        let Some(to) = snap.resolve(&to_rel) else {
            self.error = Some("path escapes the sandbox".into());
            return;
        };
        match fs::rename(&from, &to) {
            Ok(()) => {
                self.pending = Pending::None;
                let was_selected = self.selected == Some(index);
                self.rescan(snap);
                self.selected = None;
                if was_selected {
                    if let Some(i) = self.index_of(&to_rel) {
                        self.open(snap, i);
                    }
                }
            }
            Err(e) => self.error = Some(format!("rename failed: {e}")),
        }
    }

    fn delete(&mut self, snap: &FsSnapshot) {
        let Pending::Delete { index } = self.pending else {
            return;
        };
        let Some(entry) = self.entry(snap, index).cloned() else {
            return;
        };
        if entry.is_dir {
            self.error = Some("directories can't be deleted from the editor".into());
            self.pending = Pending::None;
            return;
        }
        let Some(path) = snap.resolve(&entry.rel) else {
            return;
        };
        match fs::remove_file(&path) {
            Ok(()) => {
                self.pending = Pending::None;
                if self.selected == Some(index) {
                    self.selected = None;
                    self.working = String::new();
                    self.baseline = String::new();
                }
                self.rescan(snap);
            }
            Err(e) => self.error = Some(format!("delete failed: {e}")),
        }
    }

    fn index_of(&self, rel: &str) -> Option<usize> {
        self.entries.iter().position(|e| e.rel == rel)
    }
}

/// Minimal base64 (standard alphabet, padded) — avoids a dependency for the
/// image-preview data URIs.
fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(action: Action, index: Option<u32>) -> EventContext {
        EventContext::new_with_params(Vec::new(), action.params(index))
    }

    fn text_ctx(action: Action, text: &str) -> EventContext {
        EventContext::new_with_params(
            format!(r#"{{"t":"text","v":{}}}"#, serde_json_escape(text)).into_bytes(),
            action.params(None),
        )
    }

    fn form_ctx(action: Action, name: &str) -> EventContext {
        EventContext::new_with_params(
            format!(
                r#"{{"t":"form","v":{{"name":{}}}}}"#,
                serde_json_escape(name)
            )
            .into_bytes(),
            action.params(None),
        )
    }

    fn serde_json_escape(s: &str) -> String {
        format!("{s:?}")
    }

    fn sandbox() -> (tempfile::TempDir, FsSnapshot) {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").unwrap();
        fs::write(dir.path().join("README.md"), "# hi\nline two\n").unwrap();
        let snap = FsSnapshot::scan(dir.path(), 8).unwrap();
        (dir, snap)
    }

    fn idx(snap: &FsSnapshot, rel: &str) -> u32 {
        snap.entries.iter().position(|e| e.rel == rel).unwrap() as u32
    }

    #[test]
    fn select_edit_autosave_flushes_to_disk() {
        let (_d, snap) = sandbox();
        let mut ed = FileEditorState::default();
        assert!(ed.autosave, "autosave on by default");
        ed.apply(&snap, &ctx(Action::Select, Some(idx(&snap, "README.md"))));
        assert_eq!(ed.working, "# hi\nline two\n");
        ed.apply(&snap, &text_ctx(Action::Edit, "# hi\nline TWO\n"));
        assert!(!ed.dirty(), "autosave flushed immediately");
        let on_disk = fs::read_to_string(snap.resolve("README.md").unwrap()).unwrap();
        assert_eq!(on_disk, "# hi\nline TWO\n");
        assert!(ed.saved_at.is_some());
    }

    #[test]
    fn manual_mode_guards_switches_and_gates_save() {
        let (_d, snap) = sandbox();
        let mut ed = FileEditorState::manual();
        ed.apply(&snap, &ctx(Action::Select, Some(idx(&snap, "README.md"))));
        ed.apply(&snap, &text_ctx(Action::Edit, "changed\n"));
        assert!(ed.dirty());
        assert_eq!(ed.dirty_lines(), vec![true]);
        // switching away is guarded
        ed.apply(&snap, &ctx(Action::Select, Some(idx(&snap, "src/main.rs"))));
        assert!(matches!(ed.pending, Pending::Guard { .. }));
        ed.apply(&snap, &ctx(Action::GuardKeep, None));
        assert_eq!(ed.pending, Pending::None);
        assert!(ed.dirty(), "kept editing");
        // explicit save persists
        ed.apply(&snap, &ctx(Action::Save, None));
        assert!(!ed.dirty());
        let on_disk = fs::read_to_string(snap.resolve("README.md").unwrap()).unwrap();
        assert_eq!(on_disk, "changed\n");
    }

    #[test]
    fn guard_discard_switches_and_drops_edits() {
        let (_d, snap) = sandbox();
        let mut ed = FileEditorState::manual();
        ed.apply(&snap, &ctx(Action::Select, Some(idx(&snap, "README.md"))));
        ed.apply(&snap, &text_ctx(Action::Edit, "unsaved\n"));
        ed.apply(&snap, &ctx(Action::Select, Some(idx(&snap, "src/main.rs"))));
        ed.apply(&snap, &ctx(Action::GuardDiscard, None));
        assert_eq!(ed.working, "fn main() {}\n");
        let untouched = fs::read_to_string(snap.resolve("README.md").unwrap()).unwrap();
        assert_eq!(
            untouched, "# hi\nline two\n",
            "discarded edits never hit disk"
        );
    }

    #[test]
    fn conflict_reload_and_overwrite() {
        let (_d, snap) = sandbox();
        let mut ed = FileEditorState::manual();
        ed.apply(&snap, &ctx(Action::Select, Some(idx(&snap, "README.md"))));
        ed.apply(&snap, &text_ctx(Action::Edit, "mine\n"));
        // disk changes underneath (force a different mtime)
        let path = snap.resolve("README.md").unwrap();
        fs::write(&path, "theirs\n").unwrap();
        let past = SystemTime::now() - std::time::Duration::from_secs(5);
        let _ = fs::File::open(&path).and_then(|f| f.set_modified(past));
        ed.apply(&snap, &ctx(Action::Save, None));
        assert_eq!(ed.pending, Pending::Conflict, "mtime gate fired");
        // reload theirs
        ed.apply(&snap, &ctx(Action::ConflictReload, None));
        assert_eq!(ed.working, "theirs\n");
        // now edit + conflict again, overwrite this time (a DIFFERENT tamper
        // time — reload pinned opened_mtime to `past`)
        ed.apply(&snap, &text_ctx(Action::Edit, "mine again\n"));
        fs::write(&path, "theirs again\n").unwrap();
        let past2 = SystemTime::now() - std::time::Duration::from_secs(10);
        let _ = fs::File::open(&path).and_then(|f| f.set_modified(past2));
        ed.apply(&snap, &ctx(Action::Save, None));
        assert_eq!(ed.pending, Pending::Conflict);
        ed.apply(&snap, &ctx(Action::ConflictOverwrite, None));
        let on_disk = fs::read_to_string(&path).unwrap();
        assert_eq!(on_disk, "mine again\n");
    }

    #[test]
    fn managed_create_rename_delete_stay_sandboxed() {
        let (_d, snap) = sandbox();
        let mut ed = FileEditorState::default();
        // create
        ed.apply(&snap, &ctx(Action::CreateStart, None));
        ed.apply(&snap, &form_ctx(Action::CreateSubmit, "notes.md"));
        assert!(snap.resolve("notes.md").unwrap().exists());
        assert!(ed.editing, "new file opens in edit mode");
        // folder creation
        ed.apply(&snap, &ctx(Action::CreateDirStart, None));
        ed.apply(&snap, &form_ctx(Action::CreateSubmit, "docs"));
        assert!(snap.resolve("docs").unwrap().is_dir());
        // traversal refused
        ed.apply(&snap, &ctx(Action::CreateStart, None));
        ed.apply(&snap, &form_ctx(Action::CreateSubmit, "../escape"));
        assert!(ed.error.is_some());
        // rename
        let i = ed.index_of("notes.md").unwrap() as u32;
        ed.apply(&snap, &ctx(Action::RenameStart, Some(i)));
        ed.apply(&snap, &form_ctx(Action::RenameSubmit, "renamed.md"));
        assert!(snap.resolve("renamed.md").unwrap().exists());
        assert!(!snap.resolve("notes.md").unwrap().exists());
        // delete (confirmed)
        let i = ed.index_of("renamed.md").unwrap() as u32;
        ed.apply(&snap, &ctx(Action::Delete, Some(i)));
        assert!(matches!(ed.pending, Pending::Delete { .. }));
        ed.apply(&snap, &ctx(Action::DeleteConfirm, None));
        assert!(!snap.resolve("renamed.md").unwrap().exists());
    }

    #[test]
    fn toggle_autosave_flushes_pending_edits() {
        let (_d, snap) = sandbox();
        let mut ed = FileEditorState::manual();
        ed.apply(&snap, &ctx(Action::Select, Some(idx(&snap, "README.md"))));
        ed.apply(&snap, &text_ctx(Action::Edit, "pending\n"));
        assert!(ed.dirty());
        ed.apply(&snap, &ctx(Action::ToggleAutosave, None));
        assert!(ed.autosave);
        assert!(!ed.dirty(), "turning autosave on flushes");
    }

    #[test]
    fn undo_redo_roundtrip_with_autosave() {
        let (_d, snap) = sandbox();
        let mut ed = FileEditorState::default();
        ed.apply(&snap, &ctx(Action::Select, Some(idx(&snap, "README.md"))));
        let g0 = ed.generation;
        ed.apply(&snap, &text_ctx(Action::Edit, "v1\n"));
        ed.apply(&snap, &text_ctx(Action::Edit, "v1\nv2\n"));
        assert!(ed.can_undo());
        ed.apply(&snap, &ctx(Action::Undo, None));
        assert_eq!(ed.working, "v1\n");
        assert!(ed.generation > g0, "undo re-keys the textarea");
        assert_eq!(
            ed.caret,
            Some(3),
            "caret at the start of the reverted change"
        );
        let disk = fs::read_to_string(snap.resolve("README.md").unwrap()).unwrap();
        assert_eq!(disk, "v1\n", "autosave flushed the undo");
        ed.apply(&snap, &ctx(Action::Redo, None));
        assert_eq!(ed.working, "v1\nv2\n");
        // a fresh edit clears the redo branch
        ed.apply(&snap, &ctx(Action::Undo, None));
        ed.apply(&snap, &text_ctx(Action::Edit, "v1\nv3\n"));
        assert!(!ed.can_redo(), "new edit invalidates redo");
    }

    #[test]
    fn vim_toggle_is_a_session_preference() {
        let (_d, snap) = sandbox();
        let mut ed = FileEditorState::default();
        assert!(!ed.vim, "off by default");
        ed.apply(&snap, &ctx(Action::ToggleVim, None));
        assert!(ed.vim);
        ed.apply(&snap, &ctx(Action::ToggleVim, None));
        assert!(!ed.vim);
    }

    #[test]
    fn action_params_roundtrip() {
        for (a, i) in [
            (Action::Select, Some(0)),
            (Action::Select, Some(300)),
            (Action::RenameStart, Some(70000)),
            (Action::Save, None),
        ] {
            let p = a.params(i);
            assert_eq!(Action::from_u8(p[0]), Some(a));
            assert_eq!(read_varint(&p[1..]), i);
        }
    }
}
