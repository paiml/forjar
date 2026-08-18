//! A minimal HTTP/1.1 request reader and response writer.
//!
//! forjar's HTTP surface is a control plane for a handful of routes, so it uses
//! `std::net` rather than adding an async web stack to a crate that has no other
//! use for one. Nothing here is general-purpose: it reads a request line,
//! headers, and a `Content-Length` body, and it writes a response. Chunked
//! transfer encoding, keep-alive, and TLS are deliberately absent — the server
//! binds loopback by default and closes every connection after one exchange.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};

/// The largest request body accepted, in bytes.
///
/// A verb's params object is small. Without a cap, `Content-Length:
/// 99999999999` makes the server allocate until it dies.
pub const MAX_BODY: usize = 1024 * 1024;

/// A parsed HTTP request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The HTTP method, uppercased.
    pub method: String,
    /// The path with any query string removed.
    pub path: String,
    /// Header names lowercased.
    pub headers: BTreeMap<String, String>,
    /// The raw body.
    pub body: String,
}

/// Read one request from a stream.
///
/// # Errors
///
/// A string describing why the request could not be read or was malformed.
pub fn read_request<R: Read>(stream: R) -> Result<Request, String> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|e| format!("read request line: {e}"))?;

    let mut parts = line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| "empty request line".to_string())?
        .to_ascii_uppercase();
    let target = parts.next().ok_or_else(|| "missing path".to_string())?;
    let path = target.split('?').next().unwrap_or(target).to_string();

    let mut headers = BTreeMap::new();
    loop {
        let mut h = String::new();
        let n = reader
            .read_line(&mut h)
            .map_err(|e| format!("read header: {e}"))?;
        if n == 0 || h.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }

    let len: usize = headers
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    if len > MAX_BODY {
        return Err(format!("body of {len} bytes exceeds the {MAX_BODY} limit"));
    }
    let mut buf = vec![0u8; len];
    if len > 0 {
        reader
            .read_exact(&mut buf)
            .map_err(|e| format!("read body: {e}"))?;
    }

    Ok(Request {
        method,
        path,
        headers,
        body: String::from_utf8_lossy(&buf).into_owned(),
    })
}

/// Write a JSON response and close.
///
/// # Errors
///
/// Propagates the underlying write failure.
pub fn write_json<W: Write>(
    stream: &mut W,
    status: u16,
    body: &serde_json::Value,
) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(body).unwrap_or_else(|_| "{}".into());
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\n\
         content-type: application/json\r\n\
         content-length: {len}\r\n\
         connection: close\r\n\
         \r\n{text}",
        reason = reason(status),
        len = text.len(),
    )?;
    stream.flush()
}

/// The reason phrase for the statuses this server emits.
#[must_use]
pub fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(s: &str) -> Result<Request, String> {
        read_request(std::io::Cursor::new(s.as_bytes().to_vec()))
    }

    #[test]
    fn parses_a_get_with_no_body() {
        let r = raw("GET /health HTTP/1.1\r\nhost: x\r\n\r\n").unwrap();
        assert_eq!(r.method, "GET");
        assert_eq!(r.path, "/health");
        assert_eq!(r.body, "");
        assert_eq!(r.headers.get("host").unwrap(), "x");
    }

    #[test]
    fn parses_a_post_body_of_exactly_content_length() {
        let r = raw("POST /v1/plan HTTP/1.1\r\ncontent-length: 7\r\n\r\n{\"a\":1}").unwrap();
        assert_eq!(r.method, "POST");
        assert_eq!(r.body, "{\"a\":1}");
    }

    #[test]
    fn a_query_string_is_not_part_of_the_path() {
        // `/v1/verbs?x=1` must route as `/v1/verbs`, not 404.
        let r = raw("GET /v1/verbs?x=1&y=2 HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(r.path, "/v1/verbs");
    }

    #[test]
    fn header_names_are_case_insensitive() {
        let r = raw("GET / HTTP/1.1\r\nContent-Length: 0\r\nX-Thing: v\r\n\r\n").unwrap();
        assert!(r.headers.contains_key("content-length"));
        assert_eq!(r.headers.get("x-thing").unwrap(), "v");
    }

    #[test]
    fn the_method_is_normalised_to_uppercase() {
        assert_eq!(raw("get / HTTP/1.1\r\n\r\n").unwrap().method, "GET");
    }

    #[test]
    fn an_oversized_content_length_is_refused_without_allocating() {
        // The declared length is far beyond MAX_BODY; if the cap were checked
        // after the allocation this test would exhaust memory instead of
        // returning an error.
        let e = raw("POST /v1/plan HTTP/1.1\r\ncontent-length: 99999999999\r\n\r\n").unwrap_err();
        assert!(e.contains("exceeds"), "{e}");
    }

    #[test]
    fn a_truncated_body_is_an_error_not_a_silent_short_read() {
        let e = raw("POST /x HTTP/1.1\r\ncontent-length: 100\r\n\r\nshort").unwrap_err();
        assert!(e.contains("read body"), "{e}");
    }

    #[test]
    fn an_empty_request_is_rejected() {
        assert!(raw("").is_err());
        assert!(raw("GET\r\n\r\n").is_err());
    }

    #[test]
    fn responses_declare_a_matching_content_length() {
        let mut out = Vec::new();
        write_json(&mut out, 200, &serde_json::json!({"a": 1})).unwrap();
        let s = String::from_utf8(out).unwrap();
        let (head, body) = s.split_once("\r\n\r\n").unwrap();
        let declared: usize = head
            .lines()
            .find_map(|l| l.strip_prefix("content-length: "))
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(
            declared,
            body.len(),
            "a wrong content-length hangs every client"
        );
        assert!(head.starts_with("HTTP/1.1 200 OK"));
        assert!(head.contains("connection: close"));
    }

    #[test]
    fn every_status_the_router_emits_has_a_reason_phrase() {
        for s in [200, 400, 403, 404, 405, 413, 500] {
            assert_ne!(reason(s), "Unknown", "status {s} has no reason phrase");
        }
    }
}
