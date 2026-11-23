//! Genericity guard for rwire.
//!
//! Two rules, checked against the sources instead of trusted to review:
//!
//! 1. **No application.** A component library never names the app it serves: no
//!    product name, no host, no app path. Values inherited from one app's front
//!    (a width, a breakpoint, an ink) land as props or tokens, and the docs
//!    describe the role, not its ancestor.
//! 2. **The gallery rule.** Every component module is demonstrated in
//!    `catalog.rs`, or exempted below with a reason. An affordance only one app
//!    needs is a design flaw, not a feature: it has to be expressible with the
//!    props the gallery shows.

use std::path::{Path, PathBuf};

fn libs() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Words that belong to one application, never to a component library.
const APP_VOCABULARY: &[&str] = &[
    "mcp-agents",
    "mcp_agents",
    "mcpa",
    "web-rwire",
    "web_rwire",
    "arte-fact",
    "raptors",
    "old front",
];

#[test]
fn the_libraries_never_name_an_application() {
    let mut files = Vec::new();
    rust_sources(&libs(), &mut files);
    // The guard itself holds the word list: it is the one file it cannot scan.
    files.retain(|file| {
        file.file_name()
            .is_some_and(|name| name != "generic_api.rs")
    });
    assert!(
        !files.is_empty(),
        "no library source found under {:?}",
        libs()
    );

    let mut hits = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).expect("read a library source");
        for (line_no, line) in text.lines().enumerate() {
            let lower = line.to_lowercase();
            if let Some(word) = APP_VOCABULARY.iter().find(|word| lower.contains(*word)) {
                hits.push(format!(
                    "{}:{}: {:?} in {:?}",
                    file.display(),
                    line_no + 1,
                    word,
                    line.trim()
                ));
            }
        }
    }

    assert!(
        hits.is_empty(),
        "a library names an application. Move the value out of `libs/` (a prop, a token, or an \
         app-side override), then retry:\n{}",
        hits.join("\n")
    );
}

/// Modules with no gallery entry of their own, and why.
const NO_DEMO: &[(&str, &str)] = &[
    (
        "action_errors",
        "behaviour on a failing action, rendered through ActionToasts",
    ),
    (
        "action_toasts",
        "behaviour on a submitted action, no markup of its own",
    ),
    (
        "chat_item",
        "parts of ChatTranscript, demonstrated through it",
    ),
    (
        "fs_source",
        "data source for FileTree, demonstrated through it",
    ),
];

#[test]
fn every_component_is_demonstrated_in_the_gallery() {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lib_rs = std::fs::read_to_string(crate_dir.join("src/lib.rs")).expect("read lib.rs");
    let gallery =
        std::fs::read_to_string(crate_dir.join("src/catalog.rs")).expect("read the gallery");

    let mut missing = Vec::new();
    for line in lib_rs.lines() {
        let Some(rest) = line.strip_prefix("mod ") else {
            continue;
        };
        let Some(module) = rest.strip_suffix(';') else {
            continue;
        };
        if NO_DEMO.iter().any(|(exempt, _)| *exempt == module) {
            continue;
        }
        let type_name: String = module
            .split('_')
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                    None => String::new(),
                }
            })
            .collect();
        if !gallery.contains(&type_name) {
            missing.push(module.to_string());
        }
    }

    assert!(
        missing.is_empty(),
        "these component modules have no gallery entry: {:?}. Add a demo in `src/catalog.rs`, or \
         add the module to `NO_DEMO` with the reason it has no markup of its own.",
        missing
    );
}
