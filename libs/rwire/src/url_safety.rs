//! URL scheme filtering: one normalization, two policies.
//!
//! [`scheme`] finds the scheme a browser could see in a URL, after undoing
//! what could hide it: HTML character references (`&#106;`, `&colon;`),
//! percent-encoding (`%6A`), whitespace and control characters anywhere
//! (`java\tscript:`), and case. It errs on the side of seeing a scheme.
//!
//! - **Untrusted content** (Markdown): an allowlist, [`is_allowed_link`] and
//!   [`is_allowed_image`]. Relative URLs (a path, `?query`, `#anchor`),
//!   `http:`, `https:`, `mailto:` and `tel:`; images also take raster
//!   `data:image/*` (a PNG can't run script). Everything else is dropped.
//! - **Attributes the application writes** (`href`, `src`, `xlink:href`,
//!   `action`, `formaction`, in pre-rendered HTML): a denylist of the schemes
//!   that run code, [`is_executable_attr`]: `javascript:`, `vbscript:`, and
//!   `data:` except non-SVG images in `src` (`data:image/png;…`, and
//!   `data:image;base64,…` as rwire-editor's previews write it). An allowlist there would break
//!   the app's own schemes (`magnet:`, `sms:`, `geo:`). The browser runtime
//!   applies the same rule to attributes set over the WebSocket.

/// The normalized scheme of `url` (lowercase, without the `:`), or `None`
/// for a relative URL.
pub fn scheme(url: &str) -> Option<String> {
    let normalized = normalize(url);
    let colon = normalized.find(':')?;
    let head = &normalized[..colon];
    if head.contains(['/', '?', '#']) {
        return None;
    }
    Some(head.to_string())
}

/// Whether a Markdown link may keep `url` as its `href`.
pub fn is_allowed_link(url: &str) -> bool {
    match scheme(url) {
        None => true,
        Some(s) => matches!(s.as_str(), "http" | "https" | "mailto" | "tel"),
    }
}

/// Whether a Markdown image may keep `url` as its `src`: a link URL, or a
/// raster `data:image/*`.
pub fn is_allowed_image(url: &str) -> bool {
    is_allowed_link(url) || is_raster_data(&normalize(url))
}

/// Whether the attribute `name="value"` would run code: a URL-bearing
/// attribute with a `javascript:` or `vbscript:` URL, or a `data:` URL other
/// than a non-SVG image in `src` (the runtime's `sanitize.ts` rule).
pub fn is_executable_attr(name: &str, value: &str) -> bool {
    let name = name.to_ascii_lowercase();
    if !matches!(
        name.as_str(),
        "href" | "src" | "xlink:href" | "action" | "formaction"
    ) {
        return false;
    }
    match scheme(value).as_deref() {
        Some("javascript" | "vbscript") => true,
        Some("data") => !(name == "src" && is_inert_data_image(&normalize(value))),
        _ => false,
    }
}

/// A `data:image` URL (`/png`, or no subtype as in `data:image;base64,`)
/// that is not SVG: inert in `src`. The runtime's `sanitize.ts` rule.
fn is_inert_data_image(normalized: &str) -> bool {
    normalized
        .strip_prefix("data:image")
        .and_then(|rest| rest.strip_prefix(['/', ';', ',']))
        .is_some_and(|rest| !rest.starts_with("svg"))
}

/// The raster image types a Markdown image may embed as `data:`.
const RASTER: [&str; 6] = ["png", "jpeg", "jpg", "gif", "webp", "avif"];

/// `data:image/<raster type>` followed by `;` or `,` (SVG is not raster: it
/// can carry script).
fn is_raster_data(normalized: &str) -> bool {
    let Some(rest) = normalized.strip_prefix("data:image/") else {
        return false;
    };
    let subtype = rest.split([';', ',']).next().unwrap_or("");
    rest.len() > subtype.len() && RASTER.contains(&subtype)
}

/// Decode character references and percent-encoding (repeatedly, so a
/// double encoding can't hide a scheme), drop whitespace and control
/// characters, lowercase.
fn normalize(url: &str) -> String {
    let mut s = url.to_string();
    for _ in 0..4 {
        let decoded = percent_decode(&decode_references(&s));
        if decoded == s {
            break;
        }
        s = decoded;
    }
    s.chars()
        .filter(|c| !(c.is_whitespace() || c.is_control() || *c == '\u{feff}'))
        .flat_map(char::to_lowercase)
        .collect()
}

/// `&#106;`, `&#x6A;` (the `;` optional, as browsers accept) and the named
/// references that can hide a scheme or break it up.
fn decode_references(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        match reference(rest) {
            Some((c, len)) => {
                out.push(c);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The character a reference at the start of `s` stands for, and its length.
fn reference(s: &str) -> Option<(char, usize)> {
    let body = s.strip_prefix('&')?;
    if let Some(num) = body.strip_prefix('#') {
        let (digits, radix, skip) = match num.strip_prefix(['x', 'X']) {
            Some(hex) => (hex, 16, 2),
            None => (num, 10, 1),
        };
        let len = digits
            .find(|c: char| !c.is_digit(radix))
            .unwrap_or(digits.len());
        if len == 0 {
            return None;
        }
        let code = u32::from_str_radix(&digits[..len.min(8)], radix).ok()?;
        let semi = usize::from(digits[len..].starts_with(';'));
        let c = char::from_u32(code).unwrap_or('\u{fffd}');
        return Some((c, 1 + skip + len + semi));
    }
    const NAMED: [(&str, char); 6] = [
        ("colon;", ':'),
        ("tab;", '\t'),
        ("newline;", '\n'),
        ("nbsp;", '\u{a0}'),
        ("sol;", '/'),
        ("amp;", '&'),
    ];
    let lower = body.get(..8).unwrap_or(body).to_ascii_lowercase();
    NAMED
        .iter()
        .find(|(name, _)| lower.starts_with(name))
        .map(|(name, c)| (*c, 1 + name.len()))
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(b)) => {
                out.push(b);
                i += 3;
            }
            (b, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const HIDDEN_JS: [&str; 14] = [
        "javascript:alert(1)",
        "JaVaScRiPt:alert(1)",
        "java\tscript:alert(1)",
        "java\nscript:alert(1)",
        " \u{1}javascript:alert(1)",
        "&#106;avascript:alert(1)",
        "&#x6A;avascript:alert(1)",
        "&#0000106avascript:alert(1)",
        "javascript&#58;alert(1)",
        "javascript&colon;alert(1)",
        "javascript&COLON;alert(1)",
        "%6Aavascript:alert(1)",
        "%256Aavascript:alert(1)",
        "java&Tab;script:alert(1)",
    ];

    #[test]
    fn hidden_javascript_is_seen() {
        for url in HIDDEN_JS {
            assert_eq!(scheme(url).as_deref(), Some("javascript"), "{url:?}");
            assert!(!is_allowed_link(url), "{url:?}");
            assert!(!is_allowed_image(url), "{url:?}");
            assert!(is_executable_attr("href", url), "{url:?}");
        }
    }

    #[test]
    fn links_keep_safe_urls_only() {
        for ok in [
            "/docs/intro",
            "docs/intro.md",
            "../up",
            "?q=1",
            "#section",
            "",
            "https://example.com/a?b=c#d",
            "HTTP://example.com",
            "mailto:me@example.com",
            "tel:+33123456789",
            "/search?q=a:b",
            "./file:name",
        ] {
            assert!(is_allowed_link(ok), "{ok:?} refused");
        }
        for bad in [
            "vbscript:msgbox(1)",
            "data:text/html,<script>alert(1)</script>",
            "data:image/png;base64,AAAA",
            "file:///etc/passwd",
            "magnet:?xt=urn:btih:x",
            "foo:bar",
        ] {
            assert!(!is_allowed_link(bad), "{bad:?} allowed");
        }
    }

    #[test]
    fn images_also_take_raster_data() {
        assert!(is_allowed_image("data:image/png;base64,AAAA"));
        assert!(is_allowed_image("DATA:image/JPEG,xx"));
        assert!(is_allowed_image("/img/a.png"));
        for bad in [
            "data:image/svg+xml,<svg onload=alert(1)>",
            "data:image/png",
            "data:text/html,x",
            "data:image/pngx;base64,AA",
        ] {
            assert!(!is_allowed_image(bad), "{bad:?} allowed");
        }
    }

    #[test]
    fn attributes_deny_executable_schemes_only() {
        assert!(is_executable_attr("HREF", "vbscript:x"));
        assert!(is_executable_attr("xlink:href", "javascript:x"));
        assert!(is_executable_attr("action", "javascript:x"));
        assert!(is_executable_attr("formaction", "javascript:x"));
        assert!(is_executable_attr("href", "data:text/html,x"));
        assert!(is_executable_attr("src", "data:text/html,x"));
        assert!(is_executable_attr("src", "data:image/svg+xml,x"));
        assert!(!is_executable_attr("src", "data:image/png;base64,AA"));
        assert!(!is_executable_attr("src", "data:image/x-icon;base64,AA"));
        assert!(!is_executable_attr("src", "data:image;base64,AA"));
        assert!(is_executable_attr("src", "data:image/SVG+xml;base64,AA"));
        assert!(is_executable_attr("href", "data:image/png;base64,AA"));
        // The app's own schemes and other attributes are left alone.
        assert!(!is_executable_attr("href", "magnet:?xt=urn:btih:x"));
        assert!(!is_executable_attr("href", "sms:+331"));
        assert!(!is_executable_attr("href", "https://x"));
        assert!(!is_executable_attr("title", "javascript:x"));
        assert!(!is_executable_attr("data-url", "javascript:x"));
    }
}
