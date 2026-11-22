//! Generate capsule HTML for the rwire runtime.
//!
//! The capsule ships only the runtime logic and global/composite CSS (embedded in a
//! `<style>` tag within `<head>`). The element/event/attribute/style-token **name maps**
//! ship empty: each `(kind, code)→name` entry is delivered lazily over the WebSocket via
//! `MAP_DEF` the first time a connection references the code — the name-map analogue of
//! the lazy CSS delivery (`STYLE_DEF`) used for utility/pseudo/breakpoint rules.

use std::borrow::Cow;

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
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover"></head><body>
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
    /// Google Fonts CDN import; `italic` also imports each weight's italic.
    Google { weights: Vec<u16>, italic: bool },
    /// Self-hosted font file.
    Custom {
        url: String,
        format: String,
        weight: u16,
    },
    /// A woff2 file the app serves itself (`asset()` / `asset_dir()`), at a
    /// path under the app's mount, over a weight range (a variable font).
    Local { path: String, weights: (u16, u16) },
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
                italic: false,
            },
            display: FontDisplay::Swap,
        }
    }

    /// Also import the italic of each weight (Google Fonts only).
    pub fn italic(mut self) -> Self {
        if let FontSource::Google { italic, .. } = &mut self.source {
            *italic = true;
        }
        self
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
            FontSource::Google { weights, italic } => {
                let encoded_family = self.family.replace(' ', "+");
                let (axes, styles): (&str, &[u8]) = if *italic {
                    ("ital,wght", &[0, 1])
                } else {
                    ("wght", &[0])
                };
                let tuples = styles
                    .iter()
                    .flat_map(|ital| {
                        weights.iter().map(move |w| match italic {
                            true => format!("{ital},{w}"),
                            false => w.to_string(),
                        })
                    })
                    .collect::<Vec<_>>()
                    .join(";");
                format!(
                    "@import url('https://fonts.googleapis.com/css2?family={}:{}@{}&display={}');\n",
                    encoded_family, axes, tuples, display
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
            FontSource::Local {
                path,
                weights: (low, high),
            } => {
                let weight = if low == high {
                    low.to_string()
                } else {
                    format!("{low} {high}")
                };
                format!(
                    "@font-face{{font-family:'{}';src:url('{}') format('woff2');font-weight:{};font-display:{}}}\n",
                    self.family, path, weight, display
                )
            }
        }
    }

    /// A [`FontSource::Local`] face with its path under the app's mount.
    fn under_base(&self, base_path: &str) -> Self {
        let mut face = self.clone();
        if let FontSource::Local { path, .. } = &mut face.source {
            *path = format!("{}{}", base_path.trim_end_matches('/'), path);
        }
        face
    }
}

/// The icon of [`CapsuleConfig::favicon`].
#[derive(Clone)]
pub(crate) struct Favicon {
    pub(crate) bytes: Cow<'static, [u8]>,
    pub(crate) content_type: String,
}

impl std::fmt::Debug for Favicon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Favicon")
            .field("bytes", &self.bytes.len())
            .field("content_type", &self.content_type)
            .finish()
    }
}

/// Configuration for capsule generation with styling.
#[derive(Clone, Debug, Default)]
pub struct CapsuleConfig {
    /// Theme configuration for CSS variables
    pub theme: Theme,
    /// Pre-generated composite CSS from style grouping (`.c{id}{declarations}`)
    pub(crate) composite_css: String,
    /// Render the page into the capsule for a static first paint — crawlers and
    /// no-JS clients see real HTML; the live render replaces it when the
    /// WebSocket delivers the first frame.
    pub(crate) ssr: bool,
    /// The markup of the mount div (with `ssr`, the server's per-request slot).
    pub(crate) ssr_html: String,
    /// Route patterns the static first paint skips (see [`Self::ssr_exclude`]).
    pub(crate) ssr_exclude: Vec<String>,
    /// Keep the static first paint behind `auth()` / `auth_with()` (see
    /// [`Self::ssr_behind_auth`]).
    pub(crate) ssr_behind_auth: bool,
    /// Registered font faces for the capsule.
    pub fonts: Vec<FontFace>,
    /// Optional PWA configuration (manifest, service worker, icons, head tags).
    pub(crate) pwa: Option<crate::pwa::Pwa>,
    /// The icon served at `/favicon.ico` (see [`Self::favicon`]).
    pub(crate) favicon: Option<Favicon>,
    /// The page's `<title>` (see [`Self::title`]).
    pub(crate) title: Option<String>,
    /// `<html lang>`: the page language (also the locale of grouped live values).
    pub(crate) lang: Option<String>,
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

    /// Enable a static first paint: each page load ships the root tree
    /// rendered for the requested route (plus the CSS its classes need). Regions
    /// over `#[storage(shared)]` state show its current value; memory and
    /// persisted state (per connection or session) show their default. The live
    /// render replaces it on the first WebSocket frame.
    ///
    /// The paint is rendered without a connection: the app's `on_route` handler
    /// does not run and there is no visitor identity. A view that hides shared
    /// data behind connection state (a login flow in `on_route`, a role check)
    /// would show that data in the paint: leave such routes out with
    /// [`Self::ssr_exclude`]. For the same reason the paint is off behind
    /// `auth()` / `auth_with()` unless [`Self::ssr_behind_auth`] allows it.
    ///
    /// With a PWA, the service worker's offline fallback is the `/` page cached at
    /// install, so its paint shows the shared state of that time.
    pub fn ssr(mut self, ssr: bool) -> Self {
        self.ssr = ssr;
        self
    }

    /// Leave the routes matching `pattern` (router syntax: `/admin`, `/users/:id`,
    /// `/account/*`) out of the static first paint: they are served with an empty
    /// mount and render live only. For views whose content depends on the
    /// connection (`on_route`, identity, memory or persisted state).
    pub fn ssr_exclude(mut self, pattern: impl Into<String>) -> Self {
        self.ssr_exclude.push(pattern.into());
        self
    }

    /// Render the static first paint behind `auth()` / `auth_with()` too.
    ///
    /// Off by default: the gate admits the visitor, but the paint is rendered
    /// without their identity, so a view that shows shared data only to some
    /// identities would show it to every authenticated visitor. Turn it on when
    /// every authenticated visitor may see all the shared data the paint renders
    /// (or exclude the other routes with [`Self::ssr_exclude`]).
    pub fn ssr_behind_auth(mut self, allow: bool) -> Self {
        self.ssr_behind_auth = allow;
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

    /// The page's `<title>`, shown in the browser tab and history. With a PWA,
    /// it replaces the title the PWA's name would give.
    ///
    /// One title for the whole app: it does not follow client-side
    /// navigation between routes (that would need the runtime to set
    /// `document.title` on every route change), so it is not set per route,
    /// not even in the `ssr(true)` paint, which would then disagree with the
    /// live page after the first navigation.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Serve `bytes` at `/favicon.ico` with `content_type` (`"image/x-icon"`,
    /// `"image/png"`, `"image/svg+xml"`…) and link it from the page's `<head>`.
    /// Without it, browsers still ask for `/favicon.ico` and get a short 404.
    /// With a PWA, this icon replaces the PWA's in the `<link rel="icon">`; the
    /// PWA icons still serve the manifest and the home screen.
    pub fn favicon(
        mut self,
        bytes: impl Into<Cow<'static, [u8]>>,
        content_type: impl Into<String>,
    ) -> Self {
        self.favicon = Some(Favicon {
            bytes: bytes.into(),
            content_type: content_type.into(),
        });
        self
    }

    /// Set the page language (`<html lang="…">`) — used by assistive tech and by
    /// [`ElementBuilder::live_text_grouped`](crate::ElementBuilder::live_text_grouped)
    /// for thousands separators.
    pub fn lang(mut self, lang: impl Into<String>) -> Self {
        self.lang = Some(lang.into());
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

    /// Load a web font from a woff2 file the app serves itself, with
    /// [`asset()`](crate::ServerWithRoot::asset) or
    /// [`asset_dir()`](crate::ServerWithRoot::asset_dir): no third-party
    /// request. `weights` is the weight range the file covers (`400..=400`
    /// for one weight, `100..=900` for a variable font); `path` is the path
    /// the app serves it at, from its root (`/fonts/inter.woff2`). A
    /// `base_path` mount is prefixed for you.
    ///
    /// Name the family in a [`Fonts`](crate::Fonts) stack to use it:
    ///
    /// ```ignore
    /// CapsuleConfig::new()
    ///     .font_face("Inter", 100..=900, "/fonts/inter.woff2")
    ///     .font_face("Source Serif 4", 400..=700, "/fonts/source-serif-4.woff2");
    /// // with
    /// Theme::new().fonts(Fonts::new().ui("Inter, system-ui, sans-serif"));
    /// // and
    /// Server::bind(addr)?.asset_dir("/fonts", "assets/fonts")
    /// ```
    ///
    /// # Panics
    ///
    /// If `path` is not a path from the app's root (it must start with `/`
    /// and not with `//`): a URL on another domain is what this method is
    /// there to avoid.
    pub fn font_face(
        mut self,
        family: &str,
        weights: std::ops::RangeInclusive<u16>,
        path: &str,
    ) -> Self {
        assert!(
            path.starts_with('/') && !path.starts_with("//"),
            "font_face({family:?}): {path:?} must be a path the app serves (\"/fonts/x.woff2\"), not a URL"
        );
        let family: String = family
            .chars()
            .filter(|c| !matches!(c, '\'' | '\\' | '<' | ';' | '{' | '}') && !c.is_control())
            .collect();
        let path: String = path
            .chars()
            .filter(|c| !matches!(c, '\'' | '\\' | '<' | ')' | ';' | '{' | '}') && !c.is_control())
            .collect();
        self.fonts.push(FontFace {
            family,
            source: FontSource::Local {
                path,
                weights: (*weights.start(), *weights.end()),
            },
            display: FontDisplay::Swap,
        });
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
            let font_css = font.under_base(&config.base_path).to_css();
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
            .map(|p| {
                p.head(
                    &config.theme,
                    config.favicon.is_none(),
                    config.title.is_none(),
                )
            })
            .unwrap_or_default()
    } else {
        String::new()
    };
    // Under a base path too: the proxy strips the prefix, the server answers
    // `/favicon.ico`.
    let favicon_link = config
        .favicon
        .as_ref()
        .map(|icon| {
            format!(
                "<link rel=\"icon\" href=\"{}/favicon.ico\" type=\"{}\">",
                attr_escape(&config.base_path),
                attr_escape(&icon.content_type)
            )
        })
        .unwrap_or_default();
    let title = config
        .title
        .as_deref()
        .map(|t| format!("<title>{}</title>", attr_escape(t)))
        .unwrap_or_default();

    let lang = config
        .lang
        .as_deref()
        .map(|l| format!(" lang=\"{}\"", l.replace('"', "")))
        .unwrap_or_default();
    let base_js = base_path_js(&config.base_path);
    let ssr_html = &config.ssr_html;
    let rwv = ext_version();

    format!(
        r#"<!DOCTYPE html><html{lang}><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover">{title}{favicon_link}{pwa_head}
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

/// Escape a value for a double-quoted HTML attribute.
fn attr_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
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
    #[test]
    fn the_title_is_escaped_and_replaces_the_pwa_one() {
        let html = generate_styled_capsule(&CapsuleConfig::new().title("Inbox <3 & co"), "");
        assert!(
            html.contains("<title>Inbox &lt;3 &amp; co</title>"),
            "{html}"
        );

        let config = CapsuleConfig::new()
            .title("Office")
            .pwa(crate::pwa::Pwa::new("Office PWA"));
        let html = generate_styled_capsule(&config, "");
        assert_eq!(html.matches("<title>").count(), 1, "{html}");
        assert!(html.contains("<title>Office</title>"));

        let pwa_only = CapsuleConfig::new().pwa(crate::pwa::Pwa::new("Office PWA"));
        let html = generate_styled_capsule(&pwa_only, "");
        assert!(html.contains("<title>Office PWA</title>"));
        assert!(!generate_styled_capsule(&CapsuleConfig::new(), "").contains("<title>"));
    }

    #[test]
    fn the_favicon_link_follows_the_base_path_and_is_escaped() {
        let config = CapsuleConfig::new()
            .favicon(&b"x"[..], "image/svg+xml\"><script>")
            .base_path("/preview/7");
        let html = generate_styled_capsule(&config, "");
        assert!(
            html.contains(
                "<link rel=\"icon\" href=\"/preview/7/favicon.ico\" \
                 type=\"image/svg+xml&quot;&gt;&lt;script&gt;\">"
            ),
            "{html}"
        );
        let without = generate_styled_capsule(&CapsuleConfig::new(), "");
        assert!(!without.contains("rel=\"icon\""));
    }

    #[test]
    fn a_google_face_imports_its_italics_on_request() {
        let upright = FontFace::google("EB Garamond", &[400, 600]).to_css();
        assert!(
            upright.contains("family=EB+Garamond:wght@400;600&"),
            "{upright}"
        );
        let both = FontFace::google("EB Garamond", &[400, 600])
            .italic()
            .to_css();
        assert!(
            both.contains("family=EB+Garamond:ital,wght@0,400;0,600;1,400;1,600&"),
            "{both}"
        );
    }

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
    fn font_face_loads_a_file_the_app_serves() {
        let config = CapsuleConfig::new()
            .font_face("Inter", 100..=900, "/fonts/inter.woff2")
            .font_face("Source Serif 4", 400..=400, "/fonts/serif.woff2");
        let css = generate_capsule_css(&config);
        assert!(
            css.contains("@font-face{font-family:'Inter';src:url('/fonts/inter.woff2') format('woff2');font-weight:100 900;font-display:swap}"),
            "{css}"
        );
        assert!(css.contains("font-family:'Source Serif 4';src:url('/fonts/serif.woff2') format('woff2');font-weight:400;"));
        // No third-party request: no @import, no other origin.
        assert!(
            !css.contains("@import") && !css.contains("https://"),
            "{css}"
        );
        // Under a base_path mount, the path follows the mount.
        let mounted = generate_capsule_css(&config.base_path("/preview/x"));
        assert!(
            mounted.contains("url('/preview/x/fonts/inter.woff2')"),
            "{mounted}"
        );
    }

    #[test]
    fn font_face_drops_characters_that_could_break_out() {
        let css = generate_capsule_css(&CapsuleConfig::new().font_face(
            "In\\ter</style>\n",
            400..=400,
            "/fonts/a\\b<c\u{7}.woff2",
        ));
        assert!(css.contains("font-family:'Inter/style>'"), "{css}");
        assert!(css.contains("url('/fonts/abc.woff2')"), "{css}");
    }

    #[test]
    #[should_panic(expected = "must be a path the app serves")]
    fn font_face_refuses_a_url() {
        let _ =
            CapsuleConfig::new().font_face("Inter", 400..=400, "https://fonts.example/inter.woff2");
    }

    #[test]
    #[should_panic(expected = "must be a path the app serves")]
    fn font_face_refuses_a_protocol_relative_url() {
        let _ = CapsuleConfig::new().font_face("Inter", 400..=400, "//cdn.example/inter.woff2");
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
