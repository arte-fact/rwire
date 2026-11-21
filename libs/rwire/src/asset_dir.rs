//! A folder of files served from disk on each request ([`AssetDir`]), for
//! media that changes while the server runs (images added to an article).
//!
//! Security first:
//! - the URL path is split into segments, each percent-decoded once; a segment
//!   that is empty, `.` or `..`, starts with `.` (hidden files), or decodes to
//!   `/`, `\`, `%` (double encoding) or a NUL byte is refused;
//! - the joined path is canonicalized and must stay inside the canonical
//!   folder, so a symbolic link may point inside the folder but not out of it;
//! - only regular files are served: no directory listing;
//! - the `Content-Type` comes from an allowlist of extensions, anything else is
//!   `application/octet-stream`; every response carries
//!   `X-Content-Type-Options: nosniff` and a sandboxing
//!   `Content-Security-Policy`, so an SVG or a renamed HTML file opened
//!   directly runs no script.
//!
//! Every refusal is a plain `404`, like a missing file.
//!
//! HTTP: `ETag` and `Last-Modified` with `304` on `If-None-Match` /
//! `If-Modified-Since`; `Cache-Control: no-cache` (the browser keeps the file
//! but revalidates it, since it may change on disk); a single `Range`
//! (`206`, or `416` when unsatisfiable), so videos can seek; the body is
//! streamed from the file, never read whole into memory.
//!
//! Limit: between the containment check and the open, a writer of the folder
//! could swap a file for a symbolic link. Whoever can write the folder is
//! trusted not to.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// A folder served under a URL prefix.
pub(crate) struct AssetDir {
    /// `/media`: a leading `/`, no trailing one, never empty.
    pub(crate) prefix: String,
    pub(crate) dir: PathBuf,
    /// Served before the auth gates (`public_asset_dir`).
    pub(crate) public: bool,
}

impl AssetDir {
    pub(crate) fn new(prefix: &str, dir: impl Into<PathBuf>, public: bool) -> Self {
        let trimmed = prefix.trim().trim_matches('/');
        assert!(
            !trimmed.is_empty(),
            "rwire: an asset folder needs a URL prefix (e.g. \"/media\"), not the root"
        );
        AssetDir {
            prefix: format!("/{trimmed}"),
            dir: dir.into(),
            public,
        }
    }

    /// The part of `path` (query already removed) under this folder's prefix,
    /// if the request is for this folder.
    pub(crate) fn claims<'a>(&self, path: &'a str) -> Option<&'a str> {
        let rest = path.strip_prefix(&self.prefix)?;
        if rest.is_empty() {
            return Some("");
        }
        rest.strip_prefix('/')
    }
}

/// The file the URL path `rest` (below the prefix) names inside `dir`, or
/// `None` when the path is refused or the file is not a regular file inside.
pub(crate) async fn resolve(dir: &Path, rest: &str) -> Option<(PathBuf, std::fs::Metadata)> {
    let mut relative = PathBuf::new();
    for raw in rest.split('/') {
        relative.push(decode_segment(raw)?);
    }
    if relative.as_os_str().is_empty() {
        return None;
    }
    let root = tokio::fs::canonicalize(dir).await.ok()?;
    let file = tokio::fs::canonicalize(root.join(&relative)).await.ok()?;
    if !file.starts_with(&root) {
        return None;
    }
    let meta = tokio::fs::metadata(&file).await.ok()?;
    meta.is_file().then_some((file, meta))
}

/// One URL path segment, percent-decoded once, if it is a safe file name.
fn decode_segment(raw: &str) -> Option<String> {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    let name = String::from_utf8(out).ok()?;
    let refused = name.is_empty()
        || name.starts_with('.')
        || name.contains(['/', '\\', '%', '\0'])
        || name.chars().any(char::is_control);
    (!refused).then_some(name)
}

/// The `Content-Type` of a served file, from its extension.
pub(crate) fn content_type(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "bmp" => "image/bmp",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "ogv" => "video/ogg",
        "mov" => "video/quicktime",
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "ogg" | "oga" | "opus" => "audio/ogg",
        "wav" => "audio/wav",
        "flac" => "audio/flac",
        "vtt" => "text/vtt; charset=utf-8",
        "txt" => "text/plain; charset=utf-8",
        "pdf" => "application/pdf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        _ => "application/octet-stream",
    }
}

/// Headers every response of an asset folder carries.
const SAFETY_HEADERS: &str = "X-Content-Type-Options: nosniff\r\n\
     Content-Security-Policy: default-src 'none'; img-src 'self' data:; \
     media-src 'self'; style-src 'unsafe-inline'; sandbox\r\n";

/// Answer a request for `rest` (below the prefix) from `dir`. `head_only`
/// for a `HEAD` request; `request` is the request head, for the conditional
/// and range headers.
pub(crate) async fn serve(
    stream: &mut TcpStream,
    dir: &Path,
    rest: &str,
    request: &str,
    head_only: bool,
    public: bool,
) -> std::io::Result<()> {
    let Some((file, meta)) = resolve(dir, rest).await else {
        return respond(stream, "404 Not Found", "", b"Not found").await;
    };
    let len = meta.len();
    let modified = meta.modified().unwrap_or(UNIX_EPOCH);
    let etag = etag(len, modified);
    let last_modified = http_date(modified);
    let cache = if public {
        "public, no-cache"
    } else {
        "private, no-cache"
    };
    let validators = format!(
        "ETag: {etag}\r\nLast-Modified: {last_modified}\r\nCache-Control: {cache}\r\n\
         Accept-Ranges: bytes\r\n"
    );

    let if_none_match = header(request, "if-none-match");
    let not_modified = match &if_none_match {
        Some(tags) => tags
            .split(',')
            .any(|t| t.trim() == "*" || weak_eq(t.trim(), &etag)),
        None => header(request, "if-modified-since").is_some_and(|d| d == last_modified),
    };
    if not_modified {
        return respond(stream, "304 Not Modified", &validators, b"").await;
    }

    // A single range, unless `If-Range` names another version of the file.
    let range_applies = header(request, "if-range").is_none_or(|v| v == etag || v == last_modified);
    let range = match header(request, "range").filter(|_| range_applies) {
        Some(spec) => match parse_range(&spec, len) {
            Some(range) => range,
            None => {
                let extra = format!("{validators}Content-Range: bytes */{len}\r\n");
                return respond(stream, "416 Range Not Satisfiable", &extra, b"").await;
            }
        },
        None => None,
    };

    let (status, start, count, extra) = match range {
        Some((start, end)) => (
            "206 Partial Content",
            start,
            end - start + 1,
            format!("Content-Range: bytes {start}-{end}/{len}\r\n"),
        ),
        None => ("200 OK", 0, len, String::new()),
    };
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {}\r\nContent-Length: {count}\r\n{validators}{extra}\
         {SAFETY_HEADERS}Connection: close\r\n\r\n",
        content_type(&file)
    );
    stream.write_all(head.as_bytes()).await?;
    if !head_only && count > 0 {
        let mut body = tokio::fs::File::open(&file).await?;
        if start > 0 {
            body.seek(std::io::SeekFrom::Start(start)).await?;
        }
        // Streamed in small buffers: a large video is never held in memory.
        tokio::io::copy(&mut body.take(count), stream).await?;
    }
    stream.flush().await
}

async fn respond(
    stream: &mut TcpStream,
    status: &str,
    headers: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\n\
         {headers}{SAFETY_HEADERS}Connection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await
}

fn header(request: &str, name: &str) -> Option<String> {
    request
        .lines()
        .skip(1)
        .take_while(|line| !line.is_empty())
        .find_map(|line| {
            let (n, v) = line.split_once(':')?;
            n.trim()
                .eq_ignore_ascii_case(name)
                .then(|| v.trim().to_string())
        })
}

/// A weak validator from the size and the modification time: a file rewritten
/// in place gets a new one.
fn etag(len: u64, modified: SystemTime) -> String {
    let nanos = modified
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    format!("W/\"{len:x}-{nanos:x}\"")
}

/// `If-None-Match` uses the weak comparison.
fn weak_eq(a: &str, b: &str) -> bool {
    a.trim_start_matches("W/") == b.trim_start_matches("W/")
}

/// The inclusive byte range a `Range` header asks of a `len`-byte file:
/// `Some(None)` to ignore it (not bytes, several ranges, malformed: the whole
/// file is served), `None` when it is unsatisfiable.
fn parse_range(spec: &str, len: u64) -> Option<Option<(u64, u64)>> {
    let Some(set) = spec.trim().strip_prefix("bytes=") else {
        return Some(None);
    };
    if set.contains(',') {
        return Some(None);
    }
    let Some((first, last)) = set.trim().split_once('-') else {
        return Some(None);
    };
    let (first, last) = (first.trim(), last.trim());
    let range = if first.is_empty() {
        // The last `n` bytes.
        let Ok(n) = last.parse::<u64>() else {
            return Some(None);
        };
        if n == 0 || len == 0 {
            return None;
        }
        (len.saturating_sub(n), len - 1)
    } else {
        let Ok(start) = first.parse::<u64>() else {
            return Some(None);
        };
        let end = if last.is_empty() {
            len.saturating_sub(1)
        } else {
            match last.parse::<u64>() {
                Ok(end) if end >= start => end.min(len.saturating_sub(1)),
                _ => return Some(None),
            }
        };
        if start >= len {
            return None;
        }
        (start, end)
    };
    Some(Some(range))
}

/// An IMF-fixdate (`Sun, 06 Nov 1994 08:49:37 GMT`), to the second.
fn http_date(time: SystemTime) -> String {
    let secs = time.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rem) = (secs / 86_400, secs % 86_400);
    let (hour, min, sec) = (rem / 3600, rem % 3600 / 60, rem % 60);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    const WEEKDAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!(
        "{}, {day:02} {} {year} {hour:02}:{min:02}:{sec:02} GMT",
        WEEKDAYS[(days % 7) as usize],
        MONTHS[(month - 1) as usize]
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn segments_refuse_traversal_and_encodings() {
        assert_eq!(decode_segment("photo.png").as_deref(), Some("photo.png"));
        assert_eq!(decode_segment("a%20b.png").as_deref(), Some("a b.png"));
        assert_eq!(decode_segment("a..b.png").as_deref(), Some("a..b.png"));
        for bad in [
            "",
            ".",
            "..",
            "%2e%2e",
            "%2E%2E",
            ".env",
            "%2eenv",
            "%252e%252e",
            "a%2fb",
            "a%5cb",
            "a\\b",
            "a%00b",
            "a%0ab",
            "%zz",
            "%2",
            "%ff",
        ] {
            assert_eq!(decode_segment(bad), None, "{bad:?} was accepted");
        }
    }

    #[test]
    fn prefixes_claim_their_folder_only() {
        let media = AssetDir::new("media/", "/srv/media", false);
        assert_eq!(media.prefix, "/media");
        assert_eq!(media.claims("/media/a.png"), Some("a.png"));
        assert_eq!(media.claims("/media"), Some(""));
        assert_eq!(media.claims("/mediax/a.png"), None);
        assert_eq!(media.claims("/other/a.png"), None);
    }

    #[test]
    #[should_panic(expected = "needs a URL prefix")]
    fn the_root_is_not_a_prefix() {
        AssetDir::new("/", "/srv/media", false);
    }

    #[test]
    fn content_types_come_from_an_allowlist() {
        assert_eq!(content_type(Path::new("a.PNG")), "image/png");
        assert_eq!(content_type(Path::new("clip.webm")), "video/webm");
        for other in ["page.html", "app.js", "x.css", "noext", "a.exe"] {
            assert_eq!(content_type(Path::new(other)), "application/octet-stream");
        }
    }

    #[test]
    fn ranges() {
        assert_eq!(parse_range("bytes=0-9", 100), Some(Some((0, 9))));
        assert_eq!(parse_range("bytes=90-", 100), Some(Some((90, 99))));
        assert_eq!(parse_range("bytes=-10", 100), Some(Some((90, 99))));
        assert_eq!(parse_range("bytes=50-500", 100), Some(Some((50, 99))));
        assert_eq!(parse_range("bytes=-500", 100), Some(Some((0, 99))));
        assert_eq!(parse_range("bytes=100-", 100), None);
        assert_eq!(parse_range("bytes=-0", 100), None);
        // Ignored: the whole file is served.
        assert_eq!(parse_range("bytes=0-1,5-6", 100), Some(None));
        assert_eq!(parse_range("items=0-1", 100), Some(None));
        assert_eq!(parse_range("bytes=9-2", 100), Some(None));
        assert_eq!(parse_range("bytes=x-", 100), Some(None));
    }

    #[test]
    fn http_dates() {
        assert_eq!(http_date(UNIX_EPOCH), "Thu, 01 Jan 1970 00:00:00 GMT");
        let t = UNIX_EPOCH + Duration::from_secs(784_111_777);
        assert_eq!(http_date(t), "Sun, 06 Nov 1994 08:49:37 GMT");
        let t = UNIX_EPOCH + Duration::from_secs(951_782_400);
        assert_eq!(http_date(t), "Tue, 29 Feb 2000 00:00:00 GMT");
    }
}
