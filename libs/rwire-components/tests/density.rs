//! The sizes icon buttons resolve to from the theme's density hooks (`--Qz*`):
//! `/app` measures 32 × 32 px with an 18 px glyph on desktop under a dense
//! theme, 44 × 44 px under a finger (the base CSS rule).

use rwire::theme::{generate_theme_css, Density, Theme};
use rwire::St;

/// The value `prop` resolves to in `token`'s CSS, given the theme `css`:
/// `var(--x,fallback)` takes `--x` when the theme declares it.
fn resolve(token: St, prop: &str, css: &str) -> String {
    let decl = token
        .css()
        .split(';')
        .find_map(|d| d.strip_prefix(&format!("{prop}:")))
        .unwrap_or_else(|| panic!("{token:?} sets no {prop}"));
    let Some(inner) = decl
        .strip_prefix("var(--")
        .and_then(|v| v.strip_suffix(')'))
    else {
        return decl.to_string();
    };
    let (name, fallback) = inner.split_once(',').unwrap_or((inner, ""));
    match css.split(&format!("--{name}:")).nth(1) {
        Some(rest) => rest.split(';').next().unwrap().to_string(),
        None => fallback.to_string(),
    }
}

#[test]
fn compact_icon_buttons_are_32px_with_an_18px_glyph() {
    let css = generate_theme_css(&Theme::new().density(Density::Compact));
    for token in [St::SqIconButton, St::SqIconToggle] {
        assert_eq!(resolve(token, "width", &css), "32px", "{token:?}");
        assert_eq!(resolve(token, "height", &css), "32px", "{token:?}");
    }
    for token in [St::GlyphIconButton, St::GlyphIconToggle] {
        assert_eq!(resolve(token, "width", &css), "18px", "{token:?}");
    }
    assert_eq!(resolve(St::PadIconToggle, "padding", &css), "0");
}

#[test]
fn comfortable_icon_buttons_keep_their_sizes() {
    let css = generate_theme_css(&Theme::new());
    assert_eq!(resolve(St::SqIconButton, "width", &css), "2.25rem");
    assert_eq!(resolve(St::GlyphIconButton, "width", &css), "14px");
    // The toggle: its padding around its glyph, as before (8 + 24 + 8 + borders).
    assert_eq!(resolve(St::SqIconToggle, "width", &css), "auto");
    assert_eq!(resolve(St::PadIconToggle, "padding", &css), "var(--S2)");
    assert_eq!(resolve(St::GlyphIconToggle, "width", &css), "24px");
}

#[test]
fn under_a_finger_or_below_sm_every_button_is_44px() {
    let base = generate_theme_css(&Theme::new());
    let block = base
        .split_once("@media (pointer:coarse),(max-width:639.98px){")
        .expect("touch-target media block")
        .1;
    // min-height / min-width beat the 32 px square.
    assert!(block.contains("[role=button]){min-height:44px}"), "{block}");
    assert!(block.contains(":where(button,[role=tab],[role=button]){min-width:44px}"));
}
