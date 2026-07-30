//! Tests for [`crate::core::webhook_http`].
//!
//! The framing tests use a reader that hands out the request in ARBITRARY chunks,
//! because that is exactly what the old code got wrong: it did one `read()` and
//! treated whatever arrived as the whole message. Its own tests wrote a ~19-byte
//! body in a single `write_all` on loopback, which always lands in one segment, so
//! they could never fail.

use super::webhook_http::*;
use std::io::Read;
use std::time::Duration;

/// A reader that yields pre-set chunks, then EOF.
///
/// Models a body split across TCP segments without needing a real socket. A
/// `read()` never spans two chunks, so each chunk is a segment boundary the
/// implementation has to cope with.
///
/// Tracks an offset WITHIN the current chunk: the first version advanced to the
/// next chunk after every read, so a chunk larger than the caller's buffer
/// silently lost its tail. That made two tests fail against correct product code
/// — a harness defect, and a reminder that a fixture can be the thing that's
/// wrong.
struct ChunkReader {
    chunks: Vec<Vec<u8>>,
    idx: usize,
    off: usize,
}

impl ChunkReader {
    fn new(chunks: &[&[u8]]) -> Self {
        Self {
            chunks: chunks.iter().map(|c| c.to_vec()).collect(),
            idx: 0,
            off: 0,
        }
    }
}

impl Read for ChunkReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.idx >= self.chunks.len() {
            return Ok(0);
        }
        let chunk = &self.chunks[self.idx];
        let remaining = &chunk[self.off..];
        let n = remaining.len().min(buf.len());
        buf[..n].copy_from_slice(&remaining[..n]);
        self.off += n;
        if self.off >= chunk.len() {
            self.idx += 1;
            self.off = 0;
        }
        Ok(n)
    }
}

fn deadline() -> Duration {
    Duration::from_secs(5)
}

fn head(body_len: usize) -> String {
    format!(
        "POST /webhook HTTP/1.1\r\nHost: x\r\nContent-Type: application/json\r\nContent-Length: {body_len}\r\n\r\n"
    )
}

// ── Framing ──────────────────────────────────────────────────────────────────

#[test]
fn body_in_one_segment_is_read() {
    let body = br#"{"action":"deploy"}"#;
    let raw = format!("{}{}", head(body.len()), String::from_utf8_lossy(body));
    let mut r = ChunkReader::new(&[raw.as_bytes()]);
    match read_request(&mut r, 4096, deadline()) {
        ReadOutcome::Complete {
            method,
            path,
            body: b,
            ..
        } => {
            assert_eq!(method, "POST");
            assert_eq!(path, "/webhook");
            assert_eq!(b, body);
        }
        other => panic!("expected Complete, got {other:?}"),
    }
}

/// THE regression. Head and body in separate segments — measured to return 400
/// with the event silently dropped, because Content-Length was never consulted.
#[test]
fn body_split_across_segments_is_reassembled() {
    let body = br#"{"action":"deploy"}"#;
    let h = head(body.len());
    let mut r = ChunkReader::new(&[h.as_bytes(), body]);
    match read_request(&mut r, 4096, deadline()) {
        ReadOutcome::Complete { body: b, .. } => assert_eq!(b, body),
        other => panic!("split delivery must reassemble, got {other:?}"),
    }
}

/// Byte-at-a-time is the pathological case a single read() can never survive.
#[test]
fn body_split_one_byte_at_a_time() {
    let body = br#"{"a":1}"#;
    let h = head(body.len());
    let mut chunks: Vec<&[u8]> = vec![h.as_bytes()];
    let singles: Vec<[u8; 1]> = body.iter().map(|b| [*b]).collect();
    for s in &singles {
        chunks.push(s);
    }
    let mut r = ChunkReader::new(&chunks);
    match read_request(&mut r, 4096, deadline()) {
        ReadOutcome::Complete { body: b, .. } => assert_eq!(b, body),
        other => panic!("expected Complete, got {other:?}"),
    }
}

/// Even the header block can arrive split.
#[test]
fn head_split_across_segments() {
    let body = b"{}";
    let h = head(body.len());
    let (a, b) = h.split_at(12);
    let mut r = ChunkReader::new(&[a.as_bytes(), b.as_bytes(), body]);
    assert!(matches!(
        read_request(&mut r, 4096, deadline()),
        ReadOutcome::Complete { .. }
    ));
}

/// A body longer than Content-Length must not bleed into the request.
#[test]
fn body_longer_than_content_length_is_truncated() {
    let h = head(2);
    let raw = format!("{h}{{}}EXTRA");
    let mut r = ChunkReader::new(&[raw.as_bytes()]);
    match read_request(&mut r, 4096, deadline()) {
        ReadOutcome::Complete { body, .. } => assert_eq!(body, b"{}"),
        other => panic!("expected Complete, got {other:?}"),
    }
}

#[test]
fn missing_content_length_means_empty_body() {
    let raw = "POST /webhook HTTP/1.1\r\nHost: x\r\n\r\n";
    let mut r = ChunkReader::new(&[raw.as_bytes()]);
    match read_request(&mut r, 4096, deadline()) {
        ReadOutcome::Complete { body, .. } => assert!(body.is_empty()),
        other => panic!("expected Complete, got {other:?}"),
    }
}

// ── Size limits ──────────────────────────────────────────────────────────────

/// 413 decided from the HEADER, before the body is buffered — otherwise the cap
/// does not actually bound memory. The old code clamped its read buffer to
/// `min(max_body_bytes, 65536) + 4096`, so a configured max above 64 KiB was
/// silently inert AND `BodyTooLarge` was unreachable from the server.
#[test]
fn oversize_content_length_is_rejected_before_reading_the_body() {
    let h = head(10_000);
    // Note: no body chunk at all. If the implementation tried to read it first
    // this would time out or under-read instead of answering 413.
    let mut r = ChunkReader::new(&[h.as_bytes()]);
    assert_eq!(
        read_request(&mut r, 1024, deadline()),
        ReadOutcome::Rejected {
            status: 413,
            code: "body_too_large"
        }
    );
}

/// A max above 64 KiB must be honoured, not clamped.
#[test]
fn max_body_above_64k_is_honoured() {
    let body = vec![b'x'; 200_000];
    let h = head(body.len());
    let mut r = ChunkReader::new(&[h.as_bytes(), &body]);
    match read_request(&mut r, 1024 * 1024, deadline()) {
        ReadOutcome::Complete { body: b, .. } => assert_eq!(b.len(), 200_000),
        other => panic!("1MiB max must accept a 200KB body, got {other:?}"),
    }
}

#[test]
fn oversize_head_is_rejected_with_431() {
    let mut big = String::from("POST /webhook HTTP/1.1\r\n");
    for i in 0..2000 {
        big.push_str(&format!("X-Pad-{i}: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\r\n"));
    }
    let mut r = ChunkReader::new(&[big.as_bytes()]);
    assert_eq!(
        read_request(&mut r, 4096, deadline()),
        ReadOutcome::Rejected {
            status: 431,
            code: "head_too_large"
        }
    );
}

// ── Malformed requests ───────────────────────────────────────────────────────

/// A duplicate Content-Length is a request-smuggling primitive when a proxy and
/// the origin pick different ones. The old header map silently kept the last.
#[test]
fn duplicate_content_length_is_rejected() {
    let raw = "POST /webhook HTTP/1.1\r\nContent-Length: 2\r\nContent-Length: 3\r\n\r\n{}";
    let mut r = ChunkReader::new(&[raw.as_bytes()]);
    assert_eq!(
        read_request(&mut r, 4096, deadline()),
        ReadOutcome::Rejected {
            status: 400,
            code: "duplicate_content_length"
        }
    );
}

#[test]
fn non_numeric_content_length_is_rejected() {
    let raw = "POST /webhook HTTP/1.1\r\nContent-Length: banana\r\n\r\n";
    let mut r = ChunkReader::new(&[raw.as_bytes()]);
    assert_eq!(
        read_request(&mut r, 4096, deadline()),
        ReadOutcome::Rejected {
            status: 400,
            code: "invalid_content_length"
        }
    );
}

/// Chunked bodies are refused outright rather than mis-framed.
#[test]
fn transfer_encoding_is_rejected_with_501() {
    let raw = "POST /webhook HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n";
    let mut r = ChunkReader::new(&[raw.as_bytes()]);
    assert_eq!(
        read_request(&mut r, 4096, deadline()),
        ReadOutcome::Rejected {
            status: 501,
            code: "transfer_encoding_unsupported"
        }
    );
}

#[test]
fn empty_connection_owes_no_response() {
    let mut r = ChunkReader::new(&[]);
    assert_eq!(read_request(&mut r, 4096, deadline()), ReadOutcome::Empty);
}

#[test]
fn malformed_request_line_is_rejected() {
    let raw = "BADLINE\r\n\r\n";
    let mut r = ChunkReader::new(&[raw.as_bytes()]);
    assert_eq!(
        read_request(&mut r, 4096, deadline()),
        ReadOutcome::Rejected {
            status: 400,
            code: "malformed_request_line"
        }
    );
}

/// Truncated body (client closed early) is a clean 400, not a hang.
#[test]
fn truncated_body_is_rejected() {
    let h = head(100);
    let mut r = ChunkReader::new(&[h.as_bytes(), b"short"]);
    assert_eq!(
        read_request(&mut r, 4096, deadline()),
        ReadOutcome::Rejected {
            status: 400,
            code: "incomplete_body"
        }
    );
}

/// A bare LF head terminator is not valid HTTP framing and is not accepted.
#[test]
fn bare_lf_is_not_a_head_terminator() {
    let raw = "POST /webhook HTTP/1.1\nContent-Length: 2\n\n{}";
    let mut r = ChunkReader::new(&[raw.as_bytes()]);
    // No CRLFCRLF, so the head never terminates and the read ends at EOF.
    assert!(matches!(
        read_request(&mut r, 4096, deadline()),
        ReadOutcome::Rejected { .. }
    ));
}

/// Headers are lowercased so lookup is unambiguous, and the query string is
/// preserved on the path for signature purposes.
#[test]
fn headers_are_lowercased_and_query_preserved() {
    let raw = "POST /webhook?x=1 HTTP/1.1\r\nX-Forjar-Signature: t=1,v1=ab\r\n\r\n";
    let mut r = ChunkReader::new(&[raw.as_bytes()]);
    match read_request(&mut r, 4096, deadline()) {
        ReadOutcome::Complete { path, headers, .. } => {
            assert_eq!(path, "/webhook?x=1");
            assert!(headers
                .iter()
                .any(|(k, v)| k == "x-forjar-signature" && v == "t=1,v1=ab"));
        }
        other => panic!("expected Complete, got {other:?}"),
    }
}

// ── Responses ────────────────────────────────────────────────────────────────

/// EVERY status must produce parseable JSON. The old builder emitted
/// `{"status":"PathNotAllowed { path: "/evil" }"}` — invalid JSON, advertised as
/// application/json, echoing attacker input.
#[test]
fn every_response_body_is_valid_json() {
    for status in [
        200u16, 400, 401, 403, 404, 405, 408, 413, 431, 500, 501, 503,
    ] {
        let raw = response(status, "some_code");
        let text = String::from_utf8(raw).expect("response must be UTF-8");
        let (_, body) = text
            .split_once("\r\n\r\n")
            .unwrap_or_else(|| panic!("no header/body split for {status}"));
        let parsed: serde_json::Value = serde_json::from_str(body)
            .unwrap_or_else(|e| panic!("status {status} body is not JSON: {e} ({body})"));
        assert_eq!(parsed["status"], "some_code");
    }
}

/// A code containing quotes and braces — the shape that used to break the JSON —
/// must be escaped, not interpolated.
#[test]
fn response_escapes_hostile_codes() {
    let raw = response(404, r#"PathNotAllowed { path: "/evil" }"#);
    let text = String::from_utf8(raw).unwrap();
    let (_, body) = text.split_once("\r\n\r\n").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(body).expect("must stay valid JSON");
    assert_eq!(parsed["status"], r#"PathNotAllowed { path: "/evil" }"#);
}

#[test]
fn response_declares_length_and_closes() {
    let text = String::from_utf8(response(200, "accepted")).unwrap();
    let (head, body) = text.split_once("\r\n\r\n").unwrap();
    assert!(head.contains("Connection: close"));
    assert!(head.contains(&format!("Content-Length: {}", body.len())));
}

/// RFC 9110 9.5: a 405 MUST carry Allow.
#[test]
fn method_not_allowed_carries_allow_header() {
    let text = String::from_utf8(response(405, "method_not_allowed")).unwrap();
    assert!(text.contains("Allow: POST"), "405 must advertise Allow");
    assert!(text.starts_with("HTTP/1.1 405 Method Not Allowed"));
}

/// 503 tells the sender when to come back rather than leaving it to guess.
#[test]
fn service_unavailable_carries_retry_after() {
    let text = String::from_utf8(response(503, "queue_full")).unwrap();
    assert!(text.contains("Retry-After:"));
}

/// Statuses that used to be dead code in the reason table now render properly.
#[test]
fn previously_unreachable_statuses_have_reasons() {
    for (status, reason) in [
        (401u16, "Unauthorized"),
        (404, "Not Found"),
        (405, "Method Not Allowed"),
        (413, "Content Too Large"),
        (431, "Request Header Fields Too Large"),
        (501, "Not Implemented"),
    ] {
        let text = String::from_utf8(response(status, "c")).unwrap();
        assert!(
            text.starts_with(&format!("HTTP/1.1 {status} {reason}")),
            "status {status} rendered as {:?}",
            text.lines().next()
        );
    }
}
