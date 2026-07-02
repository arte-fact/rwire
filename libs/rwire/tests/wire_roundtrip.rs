//! Wire round-trip: emit op streams with the Rust encoder, then parse them with the
//! REAL runtime parser `x()` (via tests/wire_roundtrip.mjs) to catch wire desyncs.
//!
//! Seeded with the claw-rwire repro: cards bearing a multi-property custom
//! `-webkit-line-clamp` inline style, plus the new form/viewport tokens. If a stream
//! desyncs, the node harness prints the PARSE ERROR and this test fails.

use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};

use rwire::attr_tokens::{At, Av};
use rwire::builder::{
    build_synced_update_with_known_symbols, BuildContext, SyncedElement, SyncedRenderer,
};
use rwire::state::{ChangeSet, RendererDeps};
use rwire::style::Style;
use rwire::style_tokens::StyleKey;
use rwire::{el, El, ElementBuilder, HandlerFn, St};

/// Emit the full initial-render stream for a (non-synced) tree, including the lazy
/// MAP_DEF/STYLE_DEF prefixes (everything delivered, since the `sent` sets start empty).
fn emit_initial(root: &ElementBuilder) -> Vec<u8> {
    let mut ctx = BuildContext::new();
    let state: () = ();
    ctx.collect_symbols(root, &state);
    ctx.analyze_style_patterns(root);
    ctx.emit(root, &state);
    let mut sent_css: HashSet<StyleKey> = HashSet::new();
    let mut sent_maps: HashSet<(u8, u8)> = HashSet::new();
    ctx.finish_with_style_defs(&mut sent_css, &mut sent_maps)
        .to_vec()
}

/// The claw trigger: a clamped card with a multi-property `-webkit-*` inline style.
fn clamp_card() -> ElementBuilder {
    el(El::Div)
        .st([St::BgSurface, St::RoundedMd, St::PMd])
        .append([el(El::P)
            .style(
                Style::new()
                    .set("display", "-webkit-box")
                    .set("-webkit-line-clamp", "2")
                    .set("-webkit-box-orient", "vertical")
                    .set("overflow", "hidden"),
            )
            .text("Some clamped body text that overflows past two lines and is ellipsized.")])
}

/// Exercises the in-flight tokens (new St codes 0x343+ via STYLE_DEF, new At/Av via MAP_DEF).
fn new_tokens() -> ElementBuilder {
    el(El::Div)
        .st([St::FontInheritAll, St::HDvh, St::MinHDvh, St::MaxHDvh])
        .append([el(El::Input)
            .at(At::Type, Av::Text)
            .at(At::Autocomplete, Av::Off)
            .at(At::Spellcheck, Av::False)
            .at_str(At::Min, "0")
            .at_str(At::Max, "100")
            .at_str(At::Step, "5")
            .bool_attr(At::Autofocus)])
}

/// A long inline style string that crosses the 128-byte single-byte varint boundary,
/// exercising the 2-byte symbol-length encoding.
fn long_style() -> ElementBuilder {
    let mut s = Style::new();
    for (p, v) in [
        ("display", "-webkit-box"),
        ("-webkit-line-clamp", "3"),
        ("-webkit-box-orient", "vertical"),
        ("overflow", "hidden"),
        ("text-overflow", "ellipsis"),
        ("max-width", "min(42rem, 90vw)"),
        ("transition", "transform .2s ease, opacity .2s ease"),
        ("transform", "translateY(calc(-50% + 1px))"),
    ] {
        s = s.set(p, v);
    }
    el(El::Div).style(s).text("long-style card")
}

/// A large tree (>127 element refs) so element refs cross into 2-byte varints —
/// the regime claw's big UI runs in.
fn large_tree() -> ElementBuilder {
    let mut root = el(El::Div).st([St::DisplayFlex, St::FlexCol, St::GapSm]);
    for i in 0..80 {
        root = root.append([el(El::Div).st([St::BgSurface, St::PSm]).append([
            el(El::Span).st([St::FontInheritAll]).text("row"),
            el(El::P)
                .style(
                    Style::new()
                        .set("display", "-webkit-box")
                        .set("-webkit-line-clamp", if i % 3 == 0 { "1" } else { "2" })
                        .set("-webkit-box-orient", "vertical")
                        .set("overflow", "hidden"),
                )
                .text("clamped row body content"),
        ])]);
    }
    root
}

/// A heavier list: several clamped cards with mixed tokens + custom styles.
fn card_list() -> ElementBuilder {
    let mut list = el(El::Div).st([St::DisplayFlex, St::FlexCol, St::GapMd]);
    for i in 0..6 {
        list = list.append([el(El::Div)
            .st([St::BgSurface, St::PMd, St::RoundedMd, St::FontInheritAll])
            .append([
                el(El::H3)
                    .st([St::TextLg, St::FontSemibold])
                    .text("Card title"),
                el(El::P)
                    .style(
                        Style::new()
                            .set("display", "-webkit-box")
                            .set("-webkit-line-clamp", "2")
                            .set("-webkit-box-orient", "vertical")
                            .set("overflow", "hidden"),
                    )
                    .text(if i % 2 == 0 {
                        "even card body"
                    } else {
                        "odd card body, longer"
                    }),
            ])]);
    }
    list
}

/// State that varies a synced region's rendered content across re-renders.
#[derive(Default)]
struct CardState {
    variant: u8,
}

/// A synced region whose content (and inline `-webkit-*` style) changes per variant,
/// so a re-render interns NEW symbols and emits `SYMBOLS_EXTEND` — the incremental
/// update path the claw freeze appeared on ("after several interactions").
struct CardRenderer;
impl SyncedRenderer for CardRenderer {
    fn render_with_state(&self, state: &dyn Any) -> Option<ElementBuilder> {
        let v = state
            .downcast_ref::<CardState>()
            .map(|s| s.variant)
            .unwrap_or(0);
        let (clamp, text): (&str, &str) = match v {
            0 => ("2", "first card body"),
            1 => (
                "3",
                "second card body, noticeably longer so it clamps differently",
            ),
            _ => (
                "4",
                "third card body — different again, with more distinct content here",
            ),
        };
        Some(
            el(El::Div)
                .st([St::BgSurface, St::PMd, St::FontInheritAll])
                .append([el(El::P)
                    .style(
                        Style::new()
                            .set("display", "-webkit-box")
                            .set("-webkit-line-clamp", clamp)
                            .set("-webkit-box-orient", "vertical")
                            .set("overflow", "hidden"),
                    )
                    .text(text)]),
        )
    }
    fn clone_box(&self) -> Box<dyn SyncedRenderer> {
        Box::new(CardRenderer)
    }
    fn state_type_id(&self) -> TypeId {
        TypeId::of::<CardState>()
    }
    fn create_default_state(&self) -> Box<dyn Any + Send + Sync> {
        Box::new(CardState::default())
    }
    fn deps(&self) -> RendererDeps {
        RendererDeps::always()
    }
}

/// Emit an incremental update stream: render the region across a sequence of variants,
/// reusing one symbol/css/map state (so later renders go through `SYMBOLS_EXTEND` and
/// lazy STYLE_DEF/MAP_DEF dedup). Returns the LAST render's bytes — the incremental one.
fn emit_updates(variants: &[u8]) -> Vec<u8> {
    let synced = vec![SyncedElement::new_with_deps(
        0,
        Box::new(CardRenderer),
        TypeId::of::<CardState>(),
        RendererDeps::always(),
    )];
    let mut handlers: HashMap<u32, HandlerFn> = HashMap::new();
    let mut known: HashMap<String, u32> = HashMap::new();
    let mut hashes: HashMap<u32, u64> = HashMap::new();
    let mut sent_css: HashSet<StyleKey> = HashSet::new();
    let mut sent_maps: HashSet<(u8, u8)> = HashSet::new();

    let mut last = Vec::new();
    for &v in variants {
        let state = CardState { variant: v };
        let boxed: Box<dyn Any + Send + Sync> = Box::new(state);
        let mut states: HashMap<TypeId, &(dyn Any + Send + Sync)> = HashMap::new();
        states.insert(TypeId::of::<CardState>(), boxed.as_ref());
        let bytes = build_synced_update_with_known_symbols(
            &synced,
            &states,
            &mut handlers,
            ChangeSet::all(),
            Some(&mut known),
            Some(TypeId::of::<CardState>()),
            Some(&mut hashes),
            Some(&mut sent_css),
            Some(&mut sent_maps),
            None,
            0,
            None,
        );
        if !bytes.is_empty() {
            last = bytes.to_vec();
        }
    }
    last
}

#[test]
fn wire_streams_parse_cleanly() {
    let fixtures: Vec<(&str, Vec<u8>)> = vec![
        ("clamp_card", emit_initial(&clamp_card())),
        ("new_tokens", emit_initial(&new_tokens())),
        ("long_style", emit_initial(&long_style())),
        ("card_list", emit_initial(&card_list())),
        ("large_tree", emit_initial(&large_tree())),
        // Incremental re-renders (SYMBOLS_EXTEND + lazy dedup) across changing content.
        ("update_v0_v1", emit_updates(&[0, 1])),
        ("update_v0_v1_v2", emit_updates(&[0, 1, 2])),
        ("update_churn", emit_updates(&[0, 1, 2, 0, 1, 2, 0, 1])),
    ];

    let dir = std::env::temp_dir().join(format!("rwire_wire_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create fixture dir");
    for (name, bytes) in &fixtures {
        std::fs::write(dir.join(format!("{name}.bin")), bytes).expect("write fixture");
    }

    let harness = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/wire_roundtrip.mjs");
    let out = match std::process::Command::new("node")
        .arg(harness)
        .arg(&dir)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            // Node is the harness runtime; without it we can't parse. Don't fail CI
            // that lacks node — just note the skip.
            eprintln!("SKIP wire_roundtrip (node unavailable: {e})");
            let _ = std::fs::remove_dir_all(&dir);
            return;
        }
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        out.status.success(),
        "wire desync detected by the round-trip harness:\n{stdout}\n{stderr}"
    );
    println!("{stdout}");
}
