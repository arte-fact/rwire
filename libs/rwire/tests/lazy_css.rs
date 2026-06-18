//! Verifies lazy CSS delivery: class-referenced rules are rendered correctly,
//! accumulated as a renderer emits them, and delivered to each connection exactly
//! once (per-connection dedup). See `docs/tree-shaking-redesign.md` (Phase 2).

// Rust guideline compliant 2026-02-21

use std::collections::{BTreeSet, HashSet};

use rwire::builder::{style_def_prefix, BuildContext};
use rwire::{El, St, StyleKey, el};

#[test]
fn style_key_renders_each_kind() {
    // Util: `.u{code}{decl}`
    assert_eq!(
        StyleKey::Util(0x02).to_css_rule().as_deref(),
        Some(".u2{display:flex}")
    );
    // Pseudo: `.h{pc}u{st}{selector}{decl}`
    let pseudo = StyleKey::Pseudo(0x00, 0x02).to_css_rule().unwrap();
    assert!(pseudo.starts_with(".h0u2:hover{"), "got {pseudo}");
    // Breakpoint: wrapped in @media
    let bp = StyleKey::Breakpoint(0x01, 0x02).to_css_rule().unwrap();
    assert!(bp.starts_with("@media(min-width:768px){.b1u2{"), "got {bp}");
}

#[test]
fn style_def_prefix_delivers_once_per_connection() {
    let mut referenced = BTreeSet::new();
    referenced.insert(StyleKey::Util(0x02));
    referenced.insert(StyleKey::Util(0x03));

    let mut sent = HashSet::new();
    let first = style_def_prefix(&referenced, &mut sent);
    assert!(!first.is_empty(), "first delivery must carry the new rules");
    assert!(sent.contains(&StyleKey::Util(0x02)));
    assert!(sent.contains(&StyleKey::Util(0x03)));

    // Same rules again on the same connection → nothing re-sent.
    let second = style_def_prefix(&referenced, &mut sent);
    assert!(second.is_empty(), "already-sent rules must not be re-delivered");

    // A fresh connection (fresh `sent`) re-receives everything — this is how a
    // hard refresh recovers: a new WebSocket connection starts with an empty set.
    let mut fresh = HashSet::new();
    assert!(!style_def_prefix(&referenced, &mut fresh).is_empty());
}

#[test]
fn build_context_records_referenced_styles_across_emit() {
    let tree = el(El::Div)
        .st([St::DisplayFlex])
        .append([el(El::Span).st([St::RoundedFull])]);

    let mut ctx = BuildContext::new();
    let state = ();
    ctx.collect_symbols(&tree, &state);
    ctx.emit(&tree, &state);

    let refs = ctx.referenced_styles();
    assert!(refs.contains(&StyleKey::Util(St::DisplayFlex.as_u16())));
    assert!(refs.contains(&StyleKey::Util(St::RoundedFull.as_u16())));
}
