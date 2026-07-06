//! Sandboxed filesystem snapshot for [`FileTree`](crate::FileTree).
//!
//! `scan` walks a canonicalized root (skipping hidden entries and symlinks);
//! `resolve` is the security boundary — every path an app reads or writes
//! goes through it, and escapes (`..`, symlinks out) are refused.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// One entry of a scanned tree, depth-first, directories before files.
#[derive(Clone, Debug)]
pub struct FsEntry {
    /// Path relative to the scan root (`src/main.rs`).
    pub rel: String,
    /// File or directory name (`main.rs`).
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
}

/// A scanned directory tree with a sealed root.
pub struct FsSnapshot {
    root: PathBuf,
    pub entries: Vec<FsEntry>,
}

impl FsSnapshot {
    /// Scan `root` (canonicalized) up to `max_depth` levels. Hidden entries
    /// (dot-prefixed) and symlinks are skipped — symlinks could point outside
    /// the sandbox.
    pub fn scan(root: &Path, max_depth: usize) -> io::Result<Self> {
        let root = root.canonicalize()?;
        let mut entries = Vec::new();
        walk(&root, &root, 0, max_depth, &mut entries)?;
        Ok(Self { root, entries })
    }

    /// The sealed root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolve a relative path inside the sandbox. Returns `None` for any
    /// escape attempt: absolute paths, `..` traversal, or symlinks resolving
    /// outside the root.
    pub fn resolve(&self, rel: &str) -> Option<PathBuf> {
        let rel_path = Path::new(rel);
        if rel_path.is_absolute() {
            return None;
        }
        let joined = self.root.join(rel_path);
        // Canonicalize the deepest existing ancestor so new files can still be
        // created, while symlink/.. escapes are caught.
        let check = if joined.exists() {
            joined.canonicalize().ok()?
        } else {
            let parent = joined.parent()?.canonicalize().ok()?;
            parent.join(joined.file_name()?)
        };
        check.starts_with(&self.root).then_some(check)
    }
}

fn walk(
    root: &Path,
    dir: &Path,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<FsEntry>,
) -> io::Result<()> {
    if depth >= max_depth {
        return Ok(());
    }
    let mut items: Vec<_> = fs::read_dir(dir)?.filter_map(Result::ok).collect();
    items.sort_by_key(|e| {
        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
        (!is_dir, e.file_name())
    });
    for item in items {
        let name = item.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let ft = item.file_type()?;
        if ft.is_symlink() {
            continue;
        }
        let path = item.path();
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        let is_dir = ft.is_dir();
        out.push(FsEntry {
            rel,
            name,
            is_dir,
            depth,
        });
        if is_dir {
            walk(root, &path, depth + 1, max_depth, out)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sandbox() -> (tempfile::TempDir, FsSnapshot) {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("README.md"), "# hi").unwrap();
        fs::write(dir.path().join(".secret"), "no").unwrap();
        let snap = FsSnapshot::scan(dir.path(), 8).unwrap();
        (dir, snap)
    }

    #[test]
    fn scan_orders_dirs_first_and_skips_hidden() {
        let (_dir, snap) = sandbox();
        let rels: Vec<_> = snap.entries.iter().map(|e| e.rel.as_str()).collect();
        assert_eq!(rels, ["src", "src/main.rs", "README.md"]);
    }

    #[test]
    fn resolve_refuses_escapes() {
        let (_dir, snap) = sandbox();
        assert!(snap.resolve("src/main.rs").is_some());
        assert!(
            snap.resolve("new-file.txt").is_some(),
            "new files inside are fine"
        );
        assert!(snap.resolve("../outside").is_none());
        assert!(snap.resolve("src/../../outside").is_none());
        assert!(snap.resolve("/etc/passwd").is_none());
    }
}
