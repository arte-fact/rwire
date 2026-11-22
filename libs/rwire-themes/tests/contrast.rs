//! WCAG contrast of the text tokens on the "subtle" backgrounds they are set
//! on, for every built-in palette and style, in light and dark mode, as the
//! theme CSS actually resolves them (style overrides included).
//!
//! The pairs are named by their `St` tokens; the CSS variable each one reads
//! is taken from the token itself, so the test follows the tokens.

use std::collections::HashMap;

use rwire::theme::{contrast_ratio, generate_theme_css, Theme, ThemeStyle, MIN_TEXT_CONTRAST};
use rwire::tokens::palette::css_color_to_hex;
use rwire::{ColorPalette, St};
use rwire_themes::{palettes, styles};

/// (text token, background token).
const PAIRS: &[(St, St)] = &[
    // A status color as text on its own subtle ground (the Nord report).
    (St::TextWarning, St::BgWarningSubtle),
    (St::TextSuccess, St::BgSuccessSubtle),
    (St::TextError, St::BgErrorSubtle),
    // The dedicated on-subtle texts.
    (St::TextOnWarningSubtle, St::BgWarningSubtle),
    (St::TextOnSuccessSubtle, St::BgSuccessSubtle),
    (St::TextOnErrorSubtle, St::BgErrorSubtle),
    (St::TextOnInfoSubtle, St::BgInfoSubtle),
    (St::TextOnDestructiveSubtle, St::BgDestructiveSubtle),
    (St::TextOnPrimarySubtle, St::BgPrimarySubtle),
    // Ordinary text on the accent-subtle ground (a selected row).
    (St::TextDefault, St::BgAccentSubtle),
    (St::TextHigh, St::BgAccentSubtle),
    // Ordinary text on the plain subtle ground.
    (St::TextDefault, St::BgSubtle),
    (St::TextHigh, St::BgSubtle),
    // A status color as text on the plain grounds it also lands on: the subtle
    // ground, a surface and a raised surface (a card), as each style sets them.
    (St::TextWarning, St::BgSubtle),
    (St::TextSuccess, St::BgSubtle),
    (St::TextError, St::BgSubtle),
    (St::TextWarning, St::BgSurface),
    (St::TextSuccess, St::BgSurface),
    (St::TextError, St::BgSurface),
    (St::TextWarning, St::BgSurfaceRaised),
    (St::TextSuccess, St::BgSurfaceRaised),
    (St::TextError, St::BgSurfaceRaised),
];

fn palettes() -> Vec<(&'static str, Option<ColorPalette>)> {
    vec![
        ("default", None),
        ("nord", Some(palettes::nord())),
        ("catppuccin", Some(palettes::catppuccin())),
        ("dracula", Some(palettes::dracula())),
        ("solarized", Some(palettes::solarized())),
        ("gruvbox", Some(palettes::gruvbox())),
        ("tokyo_night", Some(palettes::tokyo_night())),
        ("rose_pine", Some(palettes::rose_pine())),
        ("one_dark", Some(palettes::one_dark())),
        ("indigo", Some(palettes::indigo())),
        ("cloth", Some(palettes::cloth())),
    ]
}

fn styles() -> Vec<(&'static str, ThemeStyle)> {
    vec![
        ("soft", ThemeStyle::soft()),
        ("flat", ThemeStyle::flat()),
        ("solid", styles::solid()),
        ("brutalist", styles::brutalist()),
        ("minimal", styles::minimal()),
        ("glass", styles::glass()),
        ("neon", styles::neon()),
        ("cloth", styles::cloth()),
    ]
}

/// The variable a single-declaration color token reads: `color:var(--p3)` → `p3`.
fn var_of(token: St) -> &'static str {
    let css = token.css();
    let v = css
        .split_once("var(--")
        .unwrap_or_else(|| panic!("{token:?} reads no variable: {css}"))
        .1;
    &v[..v.find(')').unwrap()]
}

/// `--name:value;` pairs of the theme's `:root` block; a later declaration wins.
fn vars(css: &str) -> HashMap<String, String> {
    let root = &css[css.find(":root{").unwrap() + 6..];
    let root = &root[..root.find('}').unwrap()];
    root.split(';')
        .filter_map(|d| d.strip_prefix("--"))
        .filter_map(|d| d.split_once(':'))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// A variable's color as `#rrggbb`, following `var(--x)` indirections.
fn resolve(vars: &HashMap<String, String>, name: &str) -> String {
    let mut v = vars
        .get(name)
        .unwrap_or_else(|| panic!("--{name} is not defined"))
        .clone();
    for _ in 0..8 {
        match v.strip_prefix("var(--").and_then(|r| r.strip_suffix(')')) {
            Some(inner) => v = vars[inner].clone(),
            None => break,
        }
    }
    css_color_to_hex(&v)
}

/// Every (palette, style, mode, pair) below the AA ratio.
fn failures() -> Vec<String> {
    let mut out = Vec::new();
    for (palette_name, palette) in palettes() {
        for (style_name, style) in styles() {
            for dark in [false, true] {
                let mut theme = if dark { Theme::dark() } else { Theme::light() }.style(style);
                if let Some(p) = palette.clone() {
                    theme = theme.palette(p);
                }
                let v = vars(&generate_theme_css(&theme));
                for &(text, ground) in PAIRS {
                    let (t, g) = (resolve(&v, var_of(text)), resolve(&v, var_of(ground)));
                    let r = contrast_ratio(&t, &g);
                    if r < MIN_TEXT_CONTRAST {
                        out.push(format!(
                            "{palette_name}/{style_name}/{}: {text:?} on {ground:?} = {r:.2} ({t} on {g})",
                            if dark { "dark" } else { "light" }
                        ));
                    }
                }
            }
        }
    }
    out
}

#[test]
fn every_subtle_pair_reads_at_aa_in_every_builtin_theme() {
    let f = failures();
    assert!(
        f.is_empty(),
        "{} pairs below 4.5:1:\n{}",
        f.len(),
        f.join("\n")
    );
}

#[test]
fn the_nord_dark_report_is_fixed() {
    let v = vars(&generate_theme_css(
        &Theme::dark().palette(palettes::nord()),
    ));
    let warning = contrast_ratio(&resolve(&v, "p3"), &resolve(&v, "N"));
    let selected = contrast_ratio(&resolve(&v, "k"), &resolve(&v, "n0"));
    assert!(
        warning >= MIN_TEXT_CONTRAST,
        "TextWarning on BgWarningSubtle: {warning:.2}"
    );
    assert!(
        selected >= MIN_TEXT_CONTRAST,
        "TextDefault on BgAccentSubtle: {selected:.2}"
    );
}

#[test]
fn the_ratio_matches_known_wcag_values() {
    assert!((contrast_ratio("#000000", "#ffffff") - 21.0).abs() < 1e-9);
    assert!((contrast_ratio("#777777", "#ffffff") - 4.48).abs() < 0.01);
    assert!((contrast_ratio("#ffffff", "#ffffff") - 1.0).abs() < 1e-9);
}
