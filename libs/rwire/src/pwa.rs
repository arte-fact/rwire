//! Progressive Web App support.
//!
//! Configure via [`crate::capsule_gen::CapsuleConfig::pwa`]: a [`Pwa`] yields a web
//! app manifest, a service worker (shell-caching, "update on next launch"), the
//! `<head>` tags, and the served icon bytes — making an rwire app installable.
//!
//! rwire is server-rendered over a WebSocket, so a cached shell makes the app
//! *launch* offline but it can't function without the server; offline shows the
//! runtime's reconnect overlay. See `PWA_ROADMAP.md`.
//!
//! ```ignore
//! CapsuleConfig::new().theme(app_theme()).pwa(
//!     Pwa::new("My App").short_name("App").display(PwaDisplay::Standalone),
//! )
//! ```

use std::borrow::Cow;

use crate::theme::{ResolvedPalette, Theme, ThemeMode};

/// Built-in fallback icon (512×512) used when no icons are supplied.
const DEFAULT_ICON: &[u8] = include_bytes!("../assets/pwa-icon-default.png");

/// How an installed app is displayed (manifest `display`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PwaDisplay {
    /// App window without browser chrome (the usual choice).
    #[default]
    Standalone,
    /// Fills the display, no OS UI.
    Fullscreen,
    /// Minimal browser UI (back/reload).
    MinimalUi,
    /// A normal browser tab.
    Browser,
}

impl PwaDisplay {
    fn as_str(self) -> &'static str {
        match self {
            PwaDisplay::Standalone => "standalone",
            PwaDisplay::Fullscreen => "fullscreen",
            PwaDisplay::MinimalUi => "minimal-ui",
            PwaDisplay::Browser => "browser",
        }
    }
}

/// One manifest icon. Bytes are embedded and served from memory.
#[derive(Clone, Debug)]
struct PwaIcon {
    size: u32,
    bytes: Cow<'static, [u8]>,
    maskable: bool,
}

impl PwaIcon {
    /// Served path, e.g. `/pwa/icon-512.png` or `/pwa/icon-512-maskable.png`.
    fn path(&self) -> String {
        if self.maskable {
            format!("/pwa/icon-{}-maskable.png", self.size)
        } else {
            format!("/pwa/icon-{}.png", self.size)
        }
    }
}

/// High-level PWA configuration. Attach via `CapsuleConfig::pwa`.
#[derive(Clone, Debug)]
pub struct Pwa {
    name: Cow<'static, str>,
    short_name: Option<Cow<'static, str>>,
    description: Option<Cow<'static, str>>,
    display: PwaDisplay,
    start_url: Cow<'static, str>,
    scope: Cow<'static, str>,
    theme_color: Option<String>,
    background_color: Option<String>,
    icons: Vec<PwaIcon>,
    push: bool,
    manifest_credentials: bool,
}

impl Pwa {
    /// Create a PWA config with the given application name.
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            short_name: None,
            description: None,
            display: PwaDisplay::Standalone,
            start_url: Cow::Borrowed("/"),
            scope: Cow::Borrowed("/"),
            theme_color: None,
            background_color: None,
            icons: Vec::new(),
            push: false,
            manifest_credentials: true,
        }
    }

    /// Homescreen label (defaults to `name` when unset).
    pub fn short_name(mut self, s: impl Into<Cow<'static, str>>) -> Self {
        self.short_name = Some(s.into());
        self
    }

    /// App description (manifest `description`).
    pub fn description(mut self, s: impl Into<Cow<'static, str>>) -> Self {
        self.description = Some(s.into());
        self
    }

    /// Display mode (default [`PwaDisplay::Standalone`]).
    pub fn display(mut self, d: PwaDisplay) -> Self {
        self.display = d;
        self
    }

    /// Add a PNG icon (`purpose: "any"`). Provide at least a 192 and a 512 for
    /// best results; if no icons are added, a built-in default glyph is used.
    pub fn icon(mut self, size: u32, bytes: impl Into<Cow<'static, [u8]>>) -> Self {
        self.icons.push(PwaIcon {
            size,
            bytes: bytes.into(),
            maskable: false,
        });
        self
    }

    /// Add a maskable PNG icon (`purpose: "maskable"`, needs safe-zone padding).
    pub fn maskable_icon(mut self, size: u32, bytes: impl Into<Cow<'static, [u8]>>) -> Self {
        self.icons.push(PwaIcon {
            size,
            bytes: bytes.into(),
            maskable: true,
        });
        self
    }

    /// Override the manifest `theme_color` (defaults to the theme's `bg-app`).
    pub fn theme_color(mut self, hex: impl Into<String>) -> Self {
        self.theme_color = Some(hex.into());
        self
    }

    /// Override the manifest `background_color` (defaults to the theme's `bg-app`).
    pub fn background_color(mut self, hex: impl Into<String>) -> Self {
        self.background_color = Some(hex.into());
        self
    }

    /// Override the launch URL (default `"/"`).
    pub fn start_url(mut self, s: impl Into<Cow<'static, str>>) -> Self {
        self.start_url = s.into();
        self
    }

    /// Handle Web Push in the service worker (default off): a `push` event
    /// shows a notification from its JSON payload (`title`, `body`, `url`,
    /// optional `icon` and `tag`), and a click on it focuses the app's window
    /// at that URL or opens one. Only same-origin URLs are followed; any
    /// other falls back to the start URL. Subscribing and sending live in
    /// the `rwire-push` crate; this is the browser half it relies on.
    pub fn push_notifications(mut self, on: bool) -> Self {
        self.push = on;
        self
    }

    /// Override the navigation scope (default `"/"`).
    pub fn scope(mut self, s: impl Into<Cow<'static, str>>) -> Self {
        self.scope = s.into();
        self
    }

    /// Fetch the manifest with the page's credentials (cookies, HTTP auth):
    /// `crossorigin="use-credentials"` on its `<link>`. **On by default.**
    ///
    /// Without it the browser requests the manifest *without* credentials, so
    /// behind an HTTP auth in front of the app (a proxy's basic auth, an SSO
    /// cookie) it gets a 401 and the app is not installable. The manifest is
    /// served by the app itself, on the page's origin, so the credentials never
    /// go anywhere else. rwire's own `auth()` / `auth_with()` serve the
    /// manifest, the service worker and the icons before the gate, so they need
    /// neither this nor a proxy exemption.
    ///
    /// Turn it off only to restore the previous markup.
    pub fn manifest_credentials(mut self, on: bool) -> Self {
        self.manifest_credentials = on;
        self
    }

    /// Resolve `theme_color`/`background_color`: explicit override, else the theme's
    /// resolved `bg-app` (mode-aware) converted to hex.
    fn colors(&self, theme: &Theme) -> (String, String) {
        let bg_app = {
            let rp = ResolvedPalette::new(theme);
            let v = if theme.mode == ThemeMode::Dark {
                &rp.neutral[11]
            } else {
                &rp.neutral[0]
            };
            crate::tokens::palette::css_color_to_hex(v)
        };
        (
            self.theme_color.clone().unwrap_or_else(|| bg_app.clone()),
            self.background_color.clone().unwrap_or(bg_app),
        )
    }

    /// The effective icon list (the configured icons, or the default glyph).
    fn effective_icons(&self) -> Vec<PwaIcon> {
        if self.icons.is_empty() {
            vec![PwaIcon {
                size: 512,
                bytes: Cow::Borrowed(DEFAULT_ICON),
                maskable: false,
            }]
        } else {
            self.icons.clone()
        }
    }

    /// The `<head>` fragment + SW-registration `<script>`, injected into the capsule
    /// before it is hashed (this doesn't depend on the shell hash). `icon_link`
    /// and `title`: whether to add `<link rel="icon">` and `<title>` (not when
    /// the capsule has its own favicon or title).
    pub(crate) fn head(&self, theme: &Theme, icon_link: bool, title: bool) -> String {
        let (theme_color, _) = self.colors(theme);
        self.head_fragment(&theme_color, &self.effective_icons(), icon_link, title)
    }

    /// Freeze the served artifacts (manifest, service worker, icon bytes).
    /// `shell_hash` (of the generated capsule HTML) versions the service-worker
    /// cache so a new build invalidates the old shell.
    pub(crate) fn freeze_served(&self, theme: &Theme, shell_hash: u64) -> PwaAssets {
        let (theme_color, background_color) = self.colors(theme);
        let icons = self.effective_icons();
        PwaAssets {
            manifest: self.manifest_json(&theme_color, &background_color, &icons),
            service_worker: self.service_worker_js(shell_hash, &icons),
            icons: icons
                .iter()
                .map(|i| (i.path(), "image/png", i.bytes.clone()))
                .collect(),
        }
    }

    fn manifest_json(
        &self,
        theme_color: &str,
        background_color: &str,
        icons: &[PwaIcon],
    ) -> String {
        let short = self.short_name.as_deref().unwrap_or(&self.name);
        let mut s = String::with_capacity(512);
        s.push('{');
        s.push_str(&format!("\"name\":{},", json_str(&self.name)));
        s.push_str(&format!("\"short_name\":{},", json_str(short)));
        if let Some(d) = &self.description {
            s.push_str(&format!("\"description\":{},", json_str(d)));
        }
        s.push_str(&format!("\"start_url\":{},", json_str(&self.start_url)));
        s.push_str(&format!("\"scope\":{},", json_str(&self.scope)));
        s.push_str(&format!("\"display\":\"{}\",", self.display.as_str()));
        s.push_str(&format!("\"theme_color\":{},", json_str(theme_color)));
        s.push_str(&format!(
            "\"background_color\":{},",
            json_str(background_color)
        ));
        s.push_str("\"icons\":[");
        for (i, ic) in icons.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            let purpose = if ic.maskable { "maskable" } else { "any" };
            s.push_str(&format!(
                "{{\"src\":{},\"sizes\":\"{s0}x{s0}\",\"type\":\"image/png\",\"purpose\":\"{purpose}\"}}",
                json_str(&ic.path()),
                s0 = ic.size,
            ));
        }
        s.push_str("]}");
        s
    }

    fn service_worker_js(&self, shell_hash: u64, icons: &[PwaIcon]) -> String {
        // Precache the shell + manifest + icons; update on next launch (no skipWaiting
        // / claim). Never intercept the reachability/health probes — the reconnect loop
        // hits /ready and must always reach the network, not a cached 200.
        let mut shell = String::from("'/','/manifest.webmanifest'");
        for ic in icons {
            shell.push_str(&format!(",'{}'", ic.path()));
        }
        format!(
            "const V='rw-v{shell_hash}';\n\
             const SHELL=[{shell}];\n\
             addEventListener('install',e=>e.waitUntil(caches.open(V).then(c=>c.addAll(SHELL))));\n\
             addEventListener('activate',e=>e.waitUntil(caches.keys().then(ks=>Promise.all(ks.filter(k=>k!==V).map(k=>caches.delete(k))))));\n\
             addEventListener('fetch',e=>{{const r=e.request;if(r.method!=='GET')return;const u=new URL(r.url);\
             if(u.pathname==='/ready'||u.pathname==='/health'||u.pathname==='/metrics')return;\
             if(r.mode==='navigate'){{e.respondWith(fetch(r).catch(()=>caches.match('/')));return;}}\
             if(SHELL.includes(u.pathname))e.respondWith(caches.match(r).then(c=>c||fetch(r)));}});\n{push}",
            push = if self.push {
                self.push_js(icons)
            } else {
                String::new()
            },
        )
    }

    /// The `push` and `notificationclick` handlers (see [`Pwa::push_notifications`]).
    fn push_js(&self, icons: &[PwaIcon]) -> String {
        let icon = icons.first().map(|i| i.path()).unwrap_or_default();
        format!(
            "const PN={name},PI={icon},PS={start};\n\
             const same=u=>{{const h=s=>new URL(s,location.origin);try{{const x=h(u||PS);if(x.origin===location.origin)return x.href}}catch(_){{}}return h(PS).href}};\n\
             addEventListener('push',e=>{{let d={{}};if(e.data){{try{{d=e.data.json()}}catch(_){{d={{body:e.data.text()}}}}}}\
             e.waitUntil(registration.showNotification(d.title||PN,{{body:d.body||'',icon:d.icon||PI,tag:d.tag||undefined,data:{{url:same(d.url)}}}}));}});\n\
             addEventListener('notificationclick',e=>{{e.notification.close();const u=same(e.notification.data&&e.notification.data.url);\
             e.waitUntil(clients.matchAll({{type:'window',includeUncontrolled:true}}).then(cs=>{{for(const c of cs)if(c.url===u&&'focus'in c)return c.focus();return clients.openWindow(u)}}));}});\n",
            name = json_str(&self.name),
            icon = json_str(&icon),
            start = json_str(&self.start_url),
        )
    }

    /// `<head>` fragment + the service-worker registration `<script>`.
    fn head_fragment(
        &self,
        theme_color: &str,
        icons: &[PwaIcon],
        icon_link: bool,
        title: bool,
    ) -> String {
        let first_icon = icons.first().map(|i| i.path()).unwrap_or_default();
        let icon_link = if icon_link {
            format!("<link rel=\"icon\" href=\"{first_icon}\">")
        } else {
            String::new()
        };
        let title = if title {
            format!("<title>{}</title>", html_escape(&self.name))
        } else {
            String::new()
        };
        let crossorigin = if self.manifest_credentials {
            " crossorigin=\"use-credentials\""
        } else {
            ""
        };
        format!(
            "{title}\
             <meta name=\"theme-color\" content=\"{theme_color}\">\
             <link rel=\"manifest\" href=\"/manifest.webmanifest\"{crossorigin}>\
             {icon_link}\
             <link rel=\"apple-touch-icon\" href=\"{icon}\">\
             <meta name=\"apple-mobile-web-app-capable\" content=\"yes\">\
             <meta name=\"apple-mobile-web-app-title\" content=\"{short}\">\
             <script>if('serviceWorker'in navigator)navigator.serviceWorker.register('/sw.js')</script>",
            short = html_escape(self.short_name.as_deref().unwrap_or(&self.name)),
            icon = first_icon,
        )
    }
}

/// Frozen, served PWA artifacts (the `<head>` is injected separately via [`Pwa::head`]).
pub(crate) struct PwaAssets {
    pub manifest: String,
    pub service_worker: String,
    /// `(served_path, content_type, bytes)` for each icon.
    pub icons: Vec<(String, &'static str, Cow<'static, [u8]>)>,
}

/// Minimal JSON string literal (quotes + escapes the few chars that matter).
fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Escape text for an HTML attribute/text node (title, meta content).
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Pwa {
        Pwa::new("My \"App\"")
            .short_name("App")
            .description("A test app")
    }

    fn assets() -> PwaAssets {
        sample().freeze_served(&Theme::dark(), 12345)
    }

    #[test]
    fn manifest_has_required_fields() {
        let a = assets();
        let m = &a.manifest;
        assert!(
            m.contains("\"name\":\"My \\\"App\\\"\""),
            "name escaped: {m}"
        );
        assert!(m.contains("\"short_name\":\"App\""));
        assert!(m.contains("\"start_url\":\"/\""));
        assert!(m.contains("\"display\":\"standalone\""));
        assert!(m.contains("\"theme_color\":\"#"));
        assert!(m.contains("\"background_color\":\"#"));
        assert!(m.contains("\"icons\":["));
        assert!(m.contains("\"sizes\":\"512x512\""));
    }

    #[test]
    fn default_glyph_used_when_no_icons() {
        let a = assets();
        assert_eq!(a.icons.len(), 1);
        assert_eq!(a.icons[0].0, "/pwa/icon-512.png");
        assert_eq!(a.icons[0].1, "image/png");
        assert!(!a.icons[0].2.is_empty());
    }

    #[test]
    fn service_worker_caches_shell_and_skips_probes() {
        let a = assets();
        let sw = &a.service_worker;
        assert!(sw.contains("const V='rw-v12345'"));
        assert!(sw.contains("'/','/manifest.webmanifest'"));
        assert!(sw.contains("/pwa/icon-512.png"));
        assert!(sw.contains("u.pathname==='/ready'"));
        assert!(
            !sw.contains("skipWaiting"),
            "update-on-next-launch: no skipWaiting"
        );
        assert!(!sw.contains("clients.claim"));
    }

    #[test]
    fn head_has_pwa_tags_and_registration() {
        let h = sample().head(&Theme::dark(), true, true);
        let h = &h;
        assert!(h.contains(
            "<link rel=\"manifest\" href=\"/manifest.webmanifest\" crossorigin=\"use-credentials\">"
        ));
        assert!(h.contains("<meta name=\"theme-color\""));
        assert!(h.contains("apple-touch-icon"));
        assert!(h.contains("serviceWorker"));
        assert!(
            h.contains("<title>My &quot;App&quot;</title>"),
            "title escaped: {h}"
        );
    }

    #[test]
    fn push_handlers_follow_the_option() {
        let off = assets().service_worker;
        assert!(!off.contains("'push'"), "off by default: {off}");
        assert!(!off.contains("notificationclick"));

        let on = sample()
            .push_notifications(true)
            .freeze_served(&Theme::dark(), 12345)
            .service_worker;
        assert!(on.contains("addEventListener('push'"), "{on}");
        assert!(on.contains("showNotification"));
        assert!(on.contains("addEventListener('notificationclick'"));
        assert!(on.contains("clients.openWindow"));
        // The shell caching is untouched: the handlers are appended.
        assert!(on.starts_with(off.as_str()));
        // The app name is a JS string literal, escaped.
        assert!(on.contains(r#"const PN="My \"App\"""#), "{on}");
        // Off-origin URLs are not followed.
        assert!(on.contains("x.origin===location.origin"));
    }

    /// The generated worker parses as JavaScript, with and without push
    /// (needs node; skipped without it, like `wire_roundtrip`).
    #[test]
    fn service_worker_is_valid_javascript() {
        if std::process::Command::new("node")
            .arg("--version")
            .output()
            .is_err()
        {
            eprintln!("node not found: skipping");
            return;
        }
        for push in [false, true] {
            let sw = sample()
                .push_notifications(push)
                .freeze_served(&Theme::dark(), 7)
                .service_worker;
            let path =
                std::env::temp_dir().join(format!("rwire-sw-{}-{push}.js", std::process::id()));
            std::fs::write(&path, &sw).unwrap();
            let out = std::process::Command::new("node")
                .arg("--check")
                .arg(&path)
                .output()
                .unwrap();
            std::fs::remove_file(&path).unwrap();
            assert!(
                out.status.success(),
                "push={push}: {}\n{sw}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }

    #[test]
    fn manifest_credentials_can_be_turned_off() {
        let h = sample()
            .manifest_credentials(false)
            .head(&Theme::dark(), true, true);
        assert!(h.contains("<link rel=\"manifest\" href=\"/manifest.webmanifest\">"));
        assert!(!h.contains("crossorigin"), "{h}");
        // The rest of the fragment, escaping included, is unchanged.
        let on = sample().head(&Theme::dark(), true, true);
        assert_eq!(h, on.replace(" crossorigin=\"use-credentials\"", ""));
        assert!(h.contains("<title>My &quot;App&quot;</title>"), "{h}");
    }

    #[test]
    fn manifest_credentials_keep_names_escaped() {
        let h = Pwa::new("<b>A & \"B\"</b>")
            .short_name("'s<")
            .head(&Theme::dark(), true, true);
        assert_eq!(h.matches("crossorigin=\"use-credentials\"").count(), 1);
        assert!(
            h.contains("<title>&lt;b&gt;A &amp; &quot;B&quot;&lt;/b&gt;</title>"),
            "{h}"
        );
        assert!(!h.contains("<b>"), "{h}");
        assert!(!h.contains("content=\"'s<\""), "{h}");
    }

    #[test]
    fn theme_color_override_wins() {
        let a = Pwa::new("X")
            .theme_color("#abcdef")
            .freeze_served(&Theme::dark(), 1);
        assert!(a.manifest.contains("\"theme_color\":\"#abcdef\""));
    }

    #[test]
    fn explicit_icons_replace_default() {
        let a = Pwa::new("X")
            .icon(192, vec![1u8, 2, 3])
            .maskable_icon(512, vec![4u8, 5])
            .freeze_served(&Theme::light(), 1);
        let paths: Vec<&str> = a.icons.iter().map(|i| i.0.as_str()).collect();
        assert_eq!(paths, ["/pwa/icon-192.png", "/pwa/icon-512-maskable.png"]);
        assert!(a.manifest.contains("\"purpose\":\"maskable\""));
    }
}
