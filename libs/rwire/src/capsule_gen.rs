//! Generate capsule HTML for the rwire runtime.
//!
//! The capsule ships only the runtime logic and global/composite CSS (embedded in a
//! `<style>` tag within `<head>`). The element/event/attribute/style-token **name maps**
//! ship empty: each `(kind, code)→name` entry is delivered lazily over the WebSocket via
//! `MAP_DEF` the first time a connection references the code — the name-map analogue of
//! the lazy CSS delivery (`STYLE_DEF`) used for utility/pseudo/breakpoint rules.

use crate::theme::Theme;

/// The runtime JavaScript — the built artifact from `runtime/` (TypeScript
/// source; `npm run sync` there is the only write path; CI rebuilds and fails
/// on drift). Fully static: the only capsule-injected config is
/// `const BASE='…'` — name maps, opcode table, client actions, and state live
/// inside the bundle, which exposes the opcode executor as `globalThis.__rwx`
/// for the wire harness. See `runtime/README.md` for the module map and rules.
const RUNTIME_JS: &str = include_str!("../assets/runtime.min.js");
const EXT_VIM_JS_FOR_VERSION: &str = include_str!("../assets/ext/vim.min.js");

/// Build stamp for lazy-extension URLs (`?v=`): changes whenever either
/// artifact changes, so a rebuilt server can never be served a cached stale
/// module. FNV-1a over both vendored artifacts.
pub(crate) fn ext_version() -> String {
    let mut h: u32 = 0x811c_9dc5;
    for b in RUNTIME_JS.bytes().chain(EXT_VIM_JS_FOR_VERSION.bytes()) {
        h ^= u32::from(b);
        h = h.wrapping_mul(0x0100_0193);
    }
    format!("{h:x}")
}

/// Generate a minimal (unstyled) capsule HTML.
///
/// The bundle carries its own (empty) name maps; entries arrive lazily via
/// `MAP_DEF` (see `generate_styled_capsule`).
pub fn generate_capsule() -> String {
    let rwv = ext_version();
    format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"></head><body>
<script>
const BASE='',RWV='{rwv}';
{RUNTIME_JS}
</script>
</body></html>"#
    )
}

/// How the browser should display text while the font loads.
#[derive(Clone, Debug, Default)]
pub enum FontDisplay {
    /// Show fallback immediately, swap when loaded (recommended).
    #[default]
    Swap,
    /// Brief invisible period, then fallback.
    Fallback,
    /// Use font only if already cached.
    Optional,
}

/// A font face definition that generates CSS at capsule build time.
///
/// Supports Google Fonts (via `@import`) and self-hosted fonts (via `@font-face`).
///
/// # Example
///
/// ```ignore
/// // Google Fonts
/// FontFace::google("Inter", &[400, 600])
/// FontFace::google("Fira Code", &[400])
///
/// // Self-hosted
/// FontFace::custom("MyFont")
///     .src("/fonts/myfont.woff2", "woff2")
///     .weight(400)
/// ```
#[derive(Clone, Debug)]
pub struct FontFace {
    family: String,
    source: FontSource,
    display: FontDisplay,
}

#[derive(Clone, Debug)]
enum FontSource {
    /// Google Fonts CDN import.
    Google { weights: Vec<u16> },
    /// Self-hosted font file.
    Custom {
        url: String,
        format: String,
        weight: u16,
    },
}

impl FontFace {
    /// Create a Google Fonts import.
    ///
    /// Generates `@import url('https://fonts.googleapis.com/css2?family=...')`.
    pub fn google(family: &str, weights: &[u16]) -> Self {
        Self {
            family: family.to_string(),
            source: FontSource::Google {
                weights: weights.to_vec(),
            },
            display: FontDisplay::Swap,
        }
    }

    /// Create a self-hosted font face definition.
    pub fn custom(family: &str) -> Self {
        Self {
            family: family.to_string(),
            source: FontSource::Custom {
                url: String::new(),
                format: "woff2".to_string(),
                weight: 400,
            },
            display: FontDisplay::Swap,
        }
    }

    /// Set the font source URL and format (for self-hosted fonts).
    pub fn src(mut self, url: &str, format: &str) -> Self {
        if let FontSource::Custom {
            url: ref mut u,
            format: ref mut f,
            ..
        } = self.source
        {
            *u = url.to_string();
            *f = format.to_string();
        }
        self
    }

    /// Set the font weight (for self-hosted fonts).
    pub fn weight(mut self, weight: u16) -> Self {
        if let FontSource::Custom {
            weight: ref mut w, ..
        } = self.source
        {
            *w = weight;
        }
        self
    }

    /// Set the font-display strategy.
    pub fn display(mut self, display: FontDisplay) -> Self {
        self.display = display;
        self
    }

    /// Get the font family name.
    pub fn family(&self) -> &str {
        &self.family
    }

    /// Generate CSS for this font face.
    pub fn to_css(&self) -> String {
        let display = match self.display {
            FontDisplay::Swap => "swap",
            FontDisplay::Fallback => "fallback",
            FontDisplay::Optional => "optional",
        };

        match &self.source {
            FontSource::Google { weights } => {
                let encoded_family = self.family.replace(' ', "+");
                let wght = weights
                    .iter()
                    .map(|w| w.to_string())
                    .collect::<Vec<_>>()
                    .join(";");
                format!(
                    "@import url('https://fonts.googleapis.com/css2?family={}:wght@{}&display={}');\n",
                    encoded_family, wght, display
                )
            }
            FontSource::Custom {
                url,
                format,
                weight,
            } => {
                format!(
                    "@font-face{{font-family:'{}';src:url('{}') format('{}');font-weight:{};font-display:{}}}\n",
                    self.family, url, format, weight, display
                )
            }
        }
    }
}

/// Configuration for capsule generation with styling.
#[derive(Clone, Debug, Default)]
pub struct CapsuleConfig {
    /// Theme configuration for CSS variables
    pub theme: Theme,
    /// Pre-generated composite CSS from style grouping (`.c{id}{declarations}`)
    pub(crate) composite_css: String,
    /// Render the root tree (default state) into the capsule for a static
    /// first paint — crawlers and no-JS clients see real HTML; the live render
    /// replaces it when the WebSocket delivers the first frame.
    pub(crate) ssr: bool,
    /// The pre-rendered HTML (filled by the server at startup when `ssr`).
    pub(crate) ssr_html: String,
    /// Registered font faces for the capsule.
    pub fonts: Vec<FontFace>,
    /// Optional PWA configuration (manifest, service worker, icons, head tags).
    pub(crate) pwa: Option<crate::pwa::Pwa>,
    /// URL prefix this app is mounted under (e.g. `/preview/<id>`), or empty for the root. When
    /// set, the client runtime prefixes its WebSocket URL and history entries with it while still
    /// speaking root-relative routes to the server — so a reverse proxy can strip the prefix and
    /// the app needs no server-side path changes. PWA install is suppressed under a prefix.
    pub(crate) base_path: String,
}

impl CapsuleConfig {
    /// Create a new config with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a light theme with default colors.
    pub fn light() -> Self {
        Self::default().theme(Theme::light())
    }

    /// Create a dark theme with default colors.
    pub fn dark() -> Self {
        Self::default().theme(Theme::dark())
    }

    /// Set the theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Mount this app under a URL prefix (e.g. `/preview/<id>`). Empty means the root (default).
    /// The client runtime prefixes its WebSocket URL and browser-history entries with it, while
    /// still sending root-relative routes to the server — so a same-origin reverse proxy strips the
    /// prefix and the server needs no path awareness. A leading `/` is ensured; a trailing `/` is
    /// trimmed.
    pub fn base_path(mut self, prefix: impl Into<String>) -> Self {
        let raw = prefix.into();
        let trimmed = raw.trim().trim_end_matches('/');
        self.base_path = if trimmed.is_empty() {
            String::new()
        } else if trimmed.starts_with('/') {
            trimmed.to_owned()
        } else {
            format!("/{trimmed}")
        };
        self
    }

    /// Enable a static first paint: the capsule ships the root tree rendered
    /// at its default state (plus the CSS its classes need). The live render
    /// replaces it on the first WebSocket frame.
    pub fn ssr(mut self, ssr: bool) -> Self {
        self.ssr = ssr;
        self
    }

    /// Set pre-generated composite CSS from style grouping analysis.
    pub(crate) fn with_composite_css(mut self, css: String) -> Self {
        self.composite_css = css;
        self
    }

    /// Enable PWA support (installable: manifest, service worker, icons, head tags).
    ///
    /// ```ignore
    /// CapsuleConfig::new().theme(app_theme())
    ///     .pwa(Pwa::new("My App").short_name("App"))
    /// ```
    pub fn pwa(mut self, pwa: crate::pwa::Pwa) -> Self {
        self.pwa = Some(pwa);
        self
    }

    /// Register a font face for the capsule.
    ///
    /// The font's CSS (`@import` or `@font-face`) is generated at capsule
    /// build time. Reference the font via inline `style` attributes.
    ///
    /// # Example
    ///
    /// ```ignore
    /// CapsuleConfig::new()
    ///     .font(FontFace::google("Inter", &[400, 600]))
    ///     .font(FontFace::google("Fira Code", &[400]))
    /// ```
    pub fn font(mut self, face: FontFace) -> Self {
        self.fonts.push(face);
        self
    }
}

/// Generate complete CSS for the capsule with resolved theme.
///
/// This is the **static** CSS embedded in the capsule `<style>` tag: the parts
/// that must exist before any class is applied. Class-referenced utility, pseudo
/// and breakpoint rules (`.u`/`.h`/`.b`) are NOT here — they are delivered lazily
/// over the wire via `STYLE_DEF` the first time a connection uses them. See
/// `docs/tree-shaking-redesign.md` (Phase 2).
///
/// Includes:
/// - Base reset CSS
/// - All non-color + color CSS variables (shipped whole, since lazy rules
///   reference them and the set is small and bounded)
/// - Theme `:root` semantic vars
/// - Keyframes referenced by animation utilities
/// - Composite classes (`.c{id}`), whose id set is fixed by startup analysis
pub fn generate_capsule_css(config: &CapsuleConfig) -> String {
    use crate::style_tokens::PSEUDO_GLOBAL_CSS;
    use crate::theme::{generate_base_css, generate_theme_css};
    use crate::tokens::css::{generate_color_css_all, generate_noncolor_primitive_css};

    let mut css = String::with_capacity(8192);

    // 1. Base reset (must come first).
    css.push_str(generate_base_css());

    // 2. All CSS variables (no tree-shaking): lazy rules arrive after load and
    //    reference these, so every var must already be defined. The full set is
    //    small and bounded (spacing/radius/type/shadow scales + ~62 colors).
    css.push_str(&generate_noncolor_primitive_css());
    css.push_str(&generate_color_css_all());

    // 3. Theme :root semantic vars (mode-aware colors, style preset Q-vars, radius).
    css.push_str(&generate_theme_css(&config.theme));

    // 4. Keyframes referenced by animation utilities — global, so always shipped.
    css.push_str(PSEUDO_GLOBAL_CSS);

    // 5. Composite classes (.c{id}). The emittable id set is fixed by the startup
    //    pattern analysis, so these never have the lazy/plain-helper gap.
    if !config.composite_css.is_empty() {
        css.push_str(&config.composite_css);
    }

    // 10. Font face CSS (@import and @font-face rules)
    if !config.fonts.is_empty() {
        // @import rules must appear before all other rules in CSS.
        // Collect them separately and prepend to the output.
        let mut imports = String::new();
        let mut font_faces = String::new();
        for font in &config.fonts {
            let font_css = font.to_css();
            if font_css.starts_with("@import") {
                imports.push_str(&font_css);
            } else {
                font_faces.push_str(&font_css);
            }
        }
        if !font_faces.is_empty() {
            css.push_str(&font_faces);
        }
        if !imports.is_empty() {
            css.insert_str(0, &imports);
        }
    }

    css
}

/// Render a self-contained HTML page for a tree built with the `el()` builder, for
/// content shown *before* the WebSocket capsule exists (e.g. an auth login).
///
/// Unlike the live capsule — which embeds only base/composite CSS and delivers
/// utility rules lazily over the wire — this inlines everything the page needs:
/// the capsule's static CSS (reset, variables, theme `:root`, composite classes)
/// plus the utility/pseudo/breakpoint rules for exactly the tokens `body` uses. So
/// `.st(..)`, `.hover`/`.focus`, and theme colors all resolve with no runtime and
/// no hand-written CSS.
pub fn render_static_page(
    config: &CapsuleConfig,
    title: &str,
    body: &crate::builder::ElementBuilder,
) -> String {
    let mut css = generate_capsule_css(config);
    let mut keys = std::collections::BTreeSet::new();
    body.collect_style_keys(&mut keys);
    for key in keys {
        if let Some(rule) = key.to_css_rule() {
            css.push_str(&rule);
        }
    }
    let title = title.replace('&', "&amp;").replace('<', "&lt;");
    let body_html = body.to_static_html();
    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{title}</title>\
<style>{css}</style></head><body>{body_html}</body></html>"
    )
}

/// Generate a capsule with styling support.
///
/// This is the recommended way to generate capsules for styled applications.
/// Includes:
/// - Empty element/event/attribute/style-token name maps (entries delivered lazily
///   over the wire via `MAP_DEF`, like utility CSS via `STYLE_DEF`)
/// - CSS embedded in `<style>` tag (composite + global CSS; utility/pseudo/
///   breakpoint rules are delivered lazily over the wire)
///
/// CSS is embedded directly in the capsule HTML `<style>` tag within `<head>`.
/// This ensures styles are available immediately when the page loads,
/// without waiting for the WebSocket connection.
pub fn generate_styled_capsule(config: &CapsuleConfig, css: &str) -> String {
    // The bundle's name maps start empty: each `(kind, code)→name` entry is
    // delivered lazily over the wire via `MAP_DEF` the first time a connection
    // references the code (the name-map analogue of lazy CSS / `STYLE_DEF`).
    // Client actions ride inside the bundle unconditionally (~250B).

    // PWA head tags (title, theme-color, manifest link, icons, SW registration) — suppressed under
    // a base path: those URLs are root-absolute (`/sw.js`, `/manifest.webmanifest`, `/pwa/*`) and a
    // mounted preview needn't be installable, so we simply don't advertise them.
    let pwa_head = if config.base_path.is_empty() {
        config
            .pwa
            .as_ref()
            .map(|p| p.head(&config.theme))
            .unwrap_or_default()
    } else {
        String::new()
    };

    let base_js = base_path_js(&config.base_path);
    let ssr_html = &config.ssr_html;
    let rwv = ext_version();

    format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">{pwa_head}
<style>{css}</style></head><body>
<div id="rw">{ssr_html}</div>
<script>
const RWV='{rwv}';
{base_js}
{RUNTIME_JS}
</script>
</body></html>"#
    )
}

/// The `const BASE=…` line the client runtime reads to prefix its WebSocket URL and history while
/// speaking root-relative routes to the server. Only `[A-Za-z0-9._/-]` survive, so the injected
/// literal can't break out of its quotes; anything else yields the safe empty base.
fn base_path_js(base_path: &str) -> String {
    let safe = base_path
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '/' | '-'));
    let value = if safe { base_path } else { "" };
    format!("const BASE='{value}';")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeMode;

    #[test]
    fn test_capsule_config_defaults() {
        let config = CapsuleConfig::new();
        assert_eq!(config.theme.mode, ThemeMode::Light);
        assert!(config.theme.palette_ref().is_none());
    }

    #[test]
    fn base_path_is_normalized_and_injected() {
        // Leading slash ensured, trailing trimmed.
        assert_eq!(
            CapsuleConfig::new().base_path("preview/x/").base_path,
            "/preview/x"
        );
        assert_eq!(
            CapsuleConfig::new().base_path("/preview/x").base_path,
            "/preview/x"
        );
        assert_eq!(CapsuleConfig::new().base_path("").base_path, "");

        // The default (no base) injects an empty BASE; a mount injects the prefix and drops PWA.
        let plain = generate_styled_capsule(&CapsuleConfig::dark(), ".c1{}");
        assert!(plain.contains("const BASE='';"));
        let mounted = generate_styled_capsule(
            &CapsuleConfig::dark()
                .pwa(crate::pwa::Pwa::new("x"))
                .base_path("/preview/ws-abc"),
            ".c1{}",
        );
        assert!(mounted.contains("const BASE='/preview/ws-abc';"));
        assert!(
            !mounted.contains("manifest.webmanifest"),
            "PWA head must be suppressed under a base path"
        );
        // The client runtime prefixes its socket URL and reads BASE back out.
        assert!(mounted.contains("location.host+BASE"));
    }

    #[test]
    fn base_path_js_rejects_unsafe_characters() {
        assert_eq!(base_path_js("/preview/ws-1"), "const BASE='/preview/ws-1';");
        // A quote/injection attempt collapses to the empty base.
        assert_eq!(base_path_js("/x';evil//"), "const BASE='';");
    }

    #[test]
    fn test_capsule_css_generation() {
        let config = CapsuleConfig::new();
        let css = generate_capsule_css(&config);

        // Should contain base reset
        assert!(css.contains("box-sizing"));
        // Should contain resolved semantic tokens with short names
        assert!(css.contains("--a:"), "Missing --a (bg-app)");
        assert!(css.contains("--k:"), "Missing --k (text-default)");
    }

    #[test]
    fn test_css_variables_resolved() {
        // Verify the resolved theme pipeline: semantic vars use short names with
        // direct oklch values, and all primitive vars are shipped (lazy CSS needs
        // every var present up-front).

        let config = CapsuleConfig::new();
        let css = generate_capsule_css(&config);

        // Non-color primitives are all shipped now.
        assert!(css.contains("--S4:"), "Missing --S4 (space-4)");
        assert!(css.contains("--R3:"), "Missing --R3 (radius-lg)");
        assert!(
            css.contains("--X3:"),
            "Missing --X3 (leading-normal, from base CSS)"
        );
        assert!(css.contains("--Z1:"), "Missing --Z1 (shadow-sm)");

        // Resolved semantic tokens use short names with direct oklch values
        assert!(
            css.contains("--a:oklch("),
            "Semantic --a should have resolved oklch value"
        );
        assert!(
            css.contains("--k:oklch("),
            "Semantic --k should have resolved oklch value"
        );
    }

    #[test]
    fn test_styled_capsule_structure() {
        let config = CapsuleConfig::new().theme(Theme::dark().accent("#00FF00"));

        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&config, &css);

        // Should have HTML structure
        assert!(capsule.contains("<!DOCTYPE html>"));

        // CSS should be embedded in <style> tag
        assert!(capsule.contains("<style>"));
        assert!(capsule.contains("box-sizing"));

        // No data-theme/data-style/data-palette attributes (theme-as-state handles CSS vars)
        assert!(
            !capsule.contains("data-theme"),
            "data-theme should not be emitted"
        );
        assert!(
            !capsule.contains("data-accent"),
            "data-accent should not be emitted"
        );

        // Should have div#rw for app root
        assert!(capsule.contains("<div id=\"rw\""));

        // Name maps ship empty (entries delivered lazily via MAP_DEF).
        assert!(
            capsule.contains("__rwx"),
            "runtime artifact must be embedded"
        );
    }

    #[test]
    fn test_styled_capsule_contains_css() {
        let config = CapsuleConfig::new();

        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&config, &css);

        // CSS should be in <style> tag within <head>
        assert!(capsule.contains("<style>"));
        assert!(capsule.contains("</style></head>"));

        // Should contain resolved semantic var --a (bg-app)
        assert!(capsule.contains("--a:"), "Missing resolved --a (bg-app)");
    }

    #[test]
    fn test_styled_capsule_size() {
        let config = CapsuleConfig::new();
        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&config, &css);

        // Capsule includes CSS in <style> tag - should be reasonable size
        println!(
            "Styled capsule size: {} bytes (CSS embedded)",
            capsule.len()
        );
    }

    #[test]
    fn test_capsule_ships_empty_name_maps() {
        // Name maps are delivered lazily over the wire (MAP_DEF), so the capsule ships them
        // empty rather than inlining every entry. A token reached only through a plain helper
        // can never be missing: its name arrives the first time its code is referenced.
        let capsule = generate_capsule();
        assert!(
            capsule.contains("__rwx"),
            "runtime artifact must be embedded"
        );
        assert!(!capsule.contains("0:'div'"), "names must not be inlined");
    }

    #[test]
    fn test_generate_capsule() {
        let capsule = generate_capsule();
        // Maps ship empty; element/event names arrive lazily via MAP_DEF.
        assert!(capsule.contains("__rwx"));
        assert!(!capsule.contains("0:'div'"));
        assert!(capsule.contains("<!DOCTYPE html>"));
        assert!(!capsule.contains("let ls={},lh={}"));
    }

    #[test]
    fn test_capsule_config_light() {
        let config = CapsuleConfig::light();
        assert_eq!(config.theme.mode, ThemeMode::Light);
        assert!(config.theme.palette_ref().is_none());
    }

    #[test]
    fn test_capsule_config_dark() {
        let config = CapsuleConfig::dark();
        assert_eq!(config.theme.mode, ThemeMode::Dark);
        assert!(config.theme.palette_ref().is_none());
    }

    #[test]
    fn test_composite_css_included_in_capsule() {
        let config = CapsuleConfig::new().with_composite_css(
            ".c256{display:flex;flex-direction:column;gap:var(--S4)}".to_string(),
        );

        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&config, &css);

        // Composite CSS should be embedded in the <style> tag
        assert!(
            css.contains(".c256{"),
            "Composite CSS missing from generated CSS"
        );
        assert!(
            capsule.contains(".c256{"),
            "Composite CSS missing from capsule HTML"
        );
    }

    #[test]
    fn test_utility_css_is_lazy_not_static() {
        // Utility rules are delivered lazily over the wire (STYLE_DEF), so they
        // must NOT appear in the static capsule CSS. Composites stay static.
        let config =
            CapsuleConfig::new().with_composite_css(".c256{display:flex;gap:1rem}".to_string());

        let css = generate_capsule_css(&config);

        assert!(
            !css.contains(".u2{"),
            "utility rule must be lazy, not in static CSS"
        );
        assert!(css.contains(".c256{"), "composite CSS should be static");
    }

    #[test]
    fn test_empty_composite_css_not_appended() {
        let config = CapsuleConfig::new();
        assert!(config.composite_css.is_empty());

        let css = generate_capsule_css(&config);
        // Should not contain any composite class patterns
        assert!(
            !css.contains(".c256"),
            "Empty composite CSS should not produce .c256"
        );
    }

    #[test]
    fn test_capsule_css_uses_theme_css() {
        let config = CapsuleConfig::new().theme(Theme::dark().accent("#5E81AC"));
        let css = generate_capsule_css(&config);

        // Should contain a single :root{} block with resolved theme CSS
        assert!(css.contains(":root{"), "Missing :root{{ block");
        assert!(css.contains("--a:"), "Missing --a (bg-app)");
        assert!(css.contains("--k:"), "Missing --k (text-default)");
        // Should NOT contain old dual-mode or data-attribute selectors
        assert!(
            !css.contains("[data-theme"),
            "Should not contain data-theme selectors"
        );
        assert!(
            !css.contains("[data-style"),
            "Should not contain data-style selectors"
        );
        assert!(
            !css.contains("[data-palette"),
            "Should not contain data-palette selectors"
        );
    }

    #[test]
    fn test_composite_css_vars_scanned() {
        // Composite CSS references var(--S6) which is not used by any utility token.
        // After the fix, generate_capsule_css should include --S6 in the :root block.
        let config = CapsuleConfig::new().with_composite_css(".c1{gap:var(--S6)}".to_string());
        let css = generate_capsule_css(&config);

        assert!(
            css.contains("--S6:"),
            "Composite CSS var(--S6) should cause --S6 to be included in :root"
        );
    }

    #[test]
    fn test_ssr_html_embedded_in_mount_div() {
        let mut config = CapsuleConfig::new().ssr(true);
        config.ssr_html = "<h1>Static paint</h1>".to_string();
        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&config, &css);
        assert!(capsule.contains(r#"<div id="rw"><h1>Static paint</h1></div>"#));
    }

    #[test]
    fn test_runtime_artifact_embedded() {
        // The runtime is the built artifact from runtime/ (see runtime/README.md).
        // Assert on minification-stable markers: string literals and the __rwx
        // debug hook survive; identifiers and hex literals do not.
        assert!(RUNTIME_JS.len() > 10_000, "artifact suspiciously small");
        assert!(!RUNTIME_JS.contains('\n'), "artifact must be a single line");
        let config = CapsuleConfig::new();
        let css = generate_capsule_css(&config);
        let capsule = generate_styled_capsule(&config, &css);
        assert!(capsule.contains("__rwx"), "executor hook missing");
        assert!(capsule.contains("__rwov"), "reconnect overlay missing");
        assert!(
            capsule.contains("PARSE ERROR at pos="),
            "executor error path missing"
        );
        assert!(
            capsule.contains("__synced_"),
            "synced-region addressing missing"
        );
    }
}
