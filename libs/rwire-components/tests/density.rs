//! The sizes icon buttons resolve to from the theme's density hooks (`--Qz*`):
//! `/app` measures 32 × 32 px with an 18 px glyph on desktop under a dense
//! theme, 44 × 44 px under a finger (the base CSS rule).

use rwire::theme::{generate_theme_css, Density, Theme};
use rwire::St;

/// The value `prop` resolves to in `token`'s CSS, given the theme `css`:
/// `var(--x,fallback)` takes `--x` when the theme declares it (on a desktop:
/// a fine pointer above `touch_below`).
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
    // The desktop value: the theme's block, before the touch-target media rule.
    let css = css.split("@media (pointer").next().unwrap();
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

/// Square, not just tall: width, min-width and height all come from `--Qzi`,
/// and the square does not shrink in a flex row (mcp-agents measured a
/// 20 × 32 toggle in its sidebar footer).
#[test]
fn icon_buttons_are_square_from_qzi_and_do_not_shrink() {
    let css = generate_theme_css(&Theme::new().density(Density::Compact));
    for token in [St::SqIconButton, St::SqIconToggle] {
        for prop in ["width", "min-width", "height"] {
            assert_eq!(resolve(token, prop, &css), "32px", "{token:?} {prop}");
            assert!(
                token.css().contains(&format!("{prop}:var(--Qzi,")),
                "{token:?}: {prop} from --Qzi"
            );
        }
        assert_eq!(resolve(token, "flex-shrink", &css), "0", "{token:?}");
    }
    // Under a finger, both dimensions reach 44 px, `touch_below` included.
    for theme in [Theme::new(), Theme::new().touch_below(760)] {
        let css = generate_theme_css(&theme.density(Density::Compact));
        let block = css.split_once("@media (pointer:coarse)").unwrap().1;
        assert!(block.contains("{min-height:44px}"));
        assert!(block.contains(":where(button,[role=tab],[role=button]){min-width:44px}"));
        // The icon squares follow through their variable: 44 × 44, not 32 wide.
        assert!(
            block.starts_with(",(max-width:") && block.contains("{:root{--Qzi:44px}"),
            "{block}"
        );
    }
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
