//! gzip for the capsule and the text files the server sends as they are.
//!
//! What can be compressed once is compressed at startup (the fixed capsule,
//! fixed text `asset()`s, the bundled extension modules); the per-request
//! `ssr(true)` page is compressed as it is served. A gzipped variant is sent
//! only to a client whose `Accept-Encoding` takes gzip, always with
//! `Vary: Accept-Encoding`. Asset folders (`asset_dir`, streamed with ranges)
//! and the small PWA files are sent as they are.

use std::io::Write;

use flate2::write::GzEncoder;
use flate2::Compression;

/// `bytes` gzipped (default level).
pub(crate) fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::with_capacity(bytes.len() / 3), Compression::default());
    // Writing to a Vec can't fail.
    let _ = encoder.write_all(bytes);
    encoder.finish().unwrap_or_default()
}

/// `bytes` gzipped if the type is text and it saves at least a tenth.
pub(crate) fn gzip_if_worth_it(content_type: &str, bytes: &[u8]) -> Option<Vec<u8>> {
    if !is_compressible(content_type) || bytes.len() < 512 {
        return None;
    }
    let gz = gzip(bytes);
    (gz.len() * 10 < bytes.len() * 9).then_some(gz)
}

/// Text types worth compressing (images, video and fonts are compressed
/// already).
pub(crate) fn is_compressible(content_type: &str) -> bool {
    let ct = content_type.to_ascii_lowercase();
    let ct = ct.split(';').next().unwrap_or("").trim();
    ct.starts_with("text/")
        || matches!(
            ct,
            "application/javascript"
                | "application/json"
                | "application/manifest+json"
                | "application/xml"
                | "application/wasm"
                | "image/svg+xml"
        )
}

/// Whether the request's `Accept-Encoding` takes gzip (`gzip`, `x-gzip` or
/// `*`, with a non-zero `q`).
pub(crate) fn accepts_gzip(request: &str) -> bool {
    let Some(value) = request
        .lines()
        .skip(1)
        .take_while(|line| !line.is_empty())
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.trim()
                .eq_ignore_ascii_case("accept-encoding")
                .then_some(value)
        })
    else {
        return false;
    };
    value.split(',').any(|item| {
        let mut parts = item.split(';');
        let coding = parts.next().unwrap_or("").trim().to_ascii_lowercase();
        let q = parts
            .filter_map(|p| p.trim().strip_prefix("q="))
            .filter_map(|q| q.trim().parse::<f32>().ok())
            .next()
            .unwrap_or(1.0);
        q > 0.0 && matches!(coding.as_str(), "gzip" | "x-gzip" | "*")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn head(accept: &str) -> String {
        format!("GET / HTTP/1.1\r\nHost: x\r\nAccept-Encoding: {accept}\r\n\r\n")
    }

    #[test]
    fn accept_encoding_is_parsed() {
        for yes in [
            "gzip",
            "gzip, deflate, br",
            "br;q=1.0, gzip;q=0.8",
            "*",
            "x-gzip",
            "GZIP",
        ] {
            assert!(accepts_gzip(&head(yes)), "{yes}");
        }
        for no in ["identity", "br", "gzip;q=0", "deflate, gzip;q=0.0", ""] {
            assert!(!accepts_gzip(&head(no)), "{no}");
        }
        assert!(!accepts_gzip("GET / HTTP/1.1\r\nHost: x\r\n\r\n"));
        // A body line is not a header.
        assert!(!accepts_gzip(
            "POST / HTTP/1.1\r\n\r\nAccept-Encoding: gzip"
        ));
    }

    #[test]
    fn gzip_round_trips() {
        let text = "<p>hello</p>".repeat(500);
        let gz = gzip(text.as_bytes());
        assert!(gz.len() < text.len() / 10);
        let mut back = String::new();
        flate2::read::GzDecoder::new(&gz[..])
            .read_to_string(&mut back)
            .unwrap();
        assert_eq!(back, text);
    }

    #[test]
    fn only_text_worth_it_is_compressed() {
        let js = "let a = 1;\n".repeat(200);
        assert!(gzip_if_worth_it("text/javascript; charset=utf-8", js.as_bytes()).is_some());
        assert!(gzip_if_worth_it("image/svg+xml", js.as_bytes()).is_some());
        assert!(gzip_if_worth_it("image/png", js.as_bytes()).is_none());
        assert!(gzip_if_worth_it("text/css", b"a{}").is_none(), "too small");
        let mut x: u64 = 0x9e37_79b9_7f4a_7c15;
        let noise: Vec<u8> = (0..4096)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                x as u8
            })
            .collect();
        assert!(gzip_if_worth_it("text/plain", &noise).is_none(), "no gain");
    }
}
