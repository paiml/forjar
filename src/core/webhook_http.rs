//! FJ-3105: HTTP framing and response construction for the webhook receiver.
//!
//! # Why this module exists
//!
//! The previous server did ONE `stream.read()` and treated whatever followed the
//! first `\r\n\r\n` in that buffer as the whole body. `Content-Length` was parsed
//! into the header map and never read back. Measured: writing the head, flushing,
//! sleeping 200ms, then writing a correct body returned
//! `400 {"status":"invalid JSON: EOF while parsing a value..."}` and no event
//! reached the channel. With a secret configured the same truncation degraded into
//! `SignatureInvalid`, because the MAC was computed over the prefix.
//!
//! It passed its tests because they wrote a ~19-byte body in a single
//! `write_all` on loopback, which always lands in the first segment.
//!
//! Responses were also hand-built: `format!(r#"{{"status":"{message}"}}"#)` with
//! `{validation:?}` interpolated, so a rejection emitted
//! `{"status":"PathNotAllowed { path: "/evil" }"}` — invalid JSON, under
//! `Content-Type: application/json`, reflecting attacker-controlled input. And
//! every failure collapsed to 403 while the 401/405/413 arms of `status_reason`
//! sat unused.

use std::io::Read;
use std::time::{Duration, Instant};

/// Largest request head (request line + headers) we will buffer, before 431.
pub const MAX_HEAD_BYTES: usize = 8 * 1024;

/// How a read attempt ended.
#[derive(Debug, PartialEq, Eq)]
pub enum ReadOutcome {
    /// A complete request.
    Complete {
        /// Request method, uppercased by the caller if needed.
        method: String,
        /// Request target exactly as sent, query string included.
        path: String,
        /// Header names lowercased; values trimmed.
        headers: Vec<(String, String)>,
        /// Exact body octets. NOT a String — the MAC must cover what was sent.
        body: Vec<u8>,
    },
    /// Client sent nothing / closed early. No response is owed.
    Empty,
    /// Malformed enough to answer with a status and close.
    Rejected {
        /// HTTP status to return.
        status: u16,
        /// Stable machine-readable reason code (never echoes input).
        code: &'static str,
    },
}

/// Read one HTTP request from `stream` under a whole-connection deadline.
///
/// Framing rules, in order:
/// 1. Read until `\r\n\r\n`, capped at [`MAX_HEAD_BYTES`] → 431 if exceeded.
/// 2. Reject `Transfer-Encoding` with 501 — chunked decoding is not implemented,
///    and silently mis-framing a chunked body is worse than refusing it.
/// 3. Parse `Content-Length`: absent → 0; duplicate or non-numeric → 400;
///    greater than `max_body_bytes` → **413 before reading the body**, so the cap
///    actually bounds what we buffer.
/// 4. Read exactly that many octets, 408 if the deadline passes first.
pub fn read_request<R: Read>(
    stream: &mut R,
    max_body_bytes: usize,
    deadline: Duration,
) -> ReadOutcome {
    let started = Instant::now();
    let mut buf: Vec<u8> = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];

    // ── 1. head ──────────────────────────────────────────────────────────────
    let head_end = loop {
        if let Some(pos) = find_head_end(&buf) {
            break pos;
        }
        if buf.len() > MAX_HEAD_BYTES {
            return ReadOutcome::Rejected {
                status: 431,
                code: "head_too_large",
            };
        }
        if started.elapsed() >= deadline {
            return if buf.is_empty() {
                ReadOutcome::Empty
            } else {
                ReadOutcome::Rejected {
                    status: 408,
                    code: "request_timeout",
                }
            };
        }
        match stream.read(&mut chunk) {
            Ok(0) => {
                return if buf.is_empty() {
                    ReadOutcome::Empty
                } else {
                    ReadOutcome::Rejected {
                        status: 400,
                        code: "incomplete_head",
                    }
                };
            }
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if started.elapsed() >= deadline {
                    return ReadOutcome::Rejected {
                        status: 408,
                        code: "request_timeout",
                    };
                }
            }
            Err(_) => {
                return if buf.is_empty() {
                    ReadOutcome::Empty
                } else {
                    ReadOutcome::Rejected {
                        status: 400,
                        code: "read_error",
                    }
                };
            }
        }
    };

    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let Some((method, path, headers)) = parse_head(&head) else {
        return ReadOutcome::Rejected {
            status: 400,
            code: "malformed_request_line",
        };
    };

    // ── 2. no chunked ────────────────────────────────────────────────────────
    if headers.iter().any(|(k, _)| k == "transfer-encoding") {
        return ReadOutcome::Rejected {
            status: 501,
            code: "transfer_encoding_unsupported",
        };
    }

    // ── 3. Content-Length ────────────────────────────────────────────────────
    let content_length = match parse_content_length(&headers) {
        Ok(n) => n,
        Err(code) => {
            return ReadOutcome::Rejected { status: 400, code };
        }
    };
    if content_length > max_body_bytes {
        return ReadOutcome::Rejected {
            status: 413,
            code: "body_too_large",
        };
    }

    // ── 4. body ──────────────────────────────────────────────────────────────
    let mut body: Vec<u8> = buf[head_end + 4..].to_vec();
    if body.len() > content_length {
        body.truncate(content_length);
    }
    while body.len() < content_length {
        if started.elapsed() >= deadline {
            return ReadOutcome::Rejected {
                status: 408,
                code: "request_timeout",
            };
        }
        match stream.read(&mut chunk) {
            Ok(0) => {
                return ReadOutcome::Rejected {
                    status: 400,
                    code: "incomplete_body",
                };
            }
            Ok(n) => {
                let want = content_length - body.len();
                body.extend_from_slice(&chunk[..n.min(want)]);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => {
                return ReadOutcome::Rejected {
                    status: 400,
                    code: "read_error",
                };
            }
        }
    }

    ReadOutcome::Complete {
        method,
        path,
        headers,
        body,
    }
}

/// Request line and headers, as parsed from the head block.
type ParsedHead = (String, String, Vec<(String, String)>);

/// Locate the `\r\n\r\n` head/body boundary.
///
/// CRLFCRLF only. The old code also accepted a bare `\n\n`, which is not a valid
/// HTTP message framing and creates a needless divergence from whatever proxy sits
/// in front.
fn find_head_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

/// Split the request line and headers.
fn parse_head(head: &str) -> Option<ParsedHead> {
    let mut lines = head.lines();
    let request_line = lines.next()?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();

    let mut headers = Vec::new();
    for line in lines {
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_lowercase(), v.trim().to_string()));
        }
    }
    Some((method, path, headers))
}

/// Parse `Content-Length`, rejecting duplicates and non-numeric values.
///
/// A duplicate `Content-Length` is a request-smuggling primitive when a proxy and
/// an origin disagree about which one wins, so it is refused rather than resolved.
/// The old header map silently kept the LAST occurrence.
fn parse_content_length(headers: &[(String, String)]) -> Result<usize, &'static str> {
    let mut found: Option<usize> = None;
    for (k, v) in headers {
        if k != "content-length" {
            continue;
        }
        if found.is_some() {
            return Err("duplicate_content_length");
        }
        found = Some(v.parse::<usize>().map_err(|_| "invalid_content_length")?);
    }
    Ok(found.unwrap_or(0))
}

/// Build an HTTP response with a well-formed JSON body.
///
/// `code` is a fixed reason code from this crate, never request-derived, so the
/// response cannot reflect attacker input. `serde_json` does the encoding, so the
/// body is valid JSON for every code path.
#[must_use]
pub fn response(status: u16, code: &str) -> Vec<u8> {
    let body = serde_json::json!({ "status": code }).to_string();
    let reason = status_reason(status);
    let mut out = format!(
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n",
        body.len()
    );
    // RFC 9110 9.5: 405 MUST carry Allow. Only POST is accepted.
    if status == 405 {
        out.push_str("Allow: POST\r\n");
    }
    // 503 means the event pipeline is saturated; tell the sender to come back.
    if status == 503 {
        out.push_str("Retry-After: 5\r\n");
    }
    out.push_str("\r\n");
    out.push_str(&body);
    out.into_bytes()
}

/// Build an HTTP response carrying an arbitrary JSON document.
///
/// [`response`] is deliberately limited to a fixed reason code so a webhook
/// reply can never reflect attacker input. The verb surface needs to return a
/// verb's actual result, so this takes a pre-serialised body — the caller owns
/// making sure it is JSON, and every caller in-tree serialises with serde.
///
/// Same framing as [`response`], so the two cannot drift on headers.
#[must_use]
pub fn json_response(status: u16, body: &str) -> Vec<u8> {
    let reason = status_reason(status);
    let mut out = format!(
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n",
        body.len()
    );
    if status == 405 {
        out.push_str("Allow: POST\r\n");
    }
    out.push_str("\r\n");
    out.push_str(body);
    out.into_bytes()
}

fn status_reason(code: u16) -> &'static str {
    match code {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        413 => "Content Too Large",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
}
