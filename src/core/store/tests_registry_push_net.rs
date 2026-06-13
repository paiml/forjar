//! COV-1 (PMAT-088): Network-path tests for OCI registry push.
//!
//! Exercises [`push_blob_chunked`] against a minimal in-process HTTP server
//! that speaks just enough of the OCI Distribution v1.1 push protocol. The
//! server is bound to an OS-assigned port (`127.0.0.1:0`) and runs on a
//! dedicated thread, so the tests are hermetic and parallel-safe — no fixed
//! ports, no shared global state, no real network.
//!
//! The production push functions shell out to `curl`, so these tests verify
//! the real subprocess + HTTP round-trip, not a mock of it.
//!
//! All four are `#[ignore]`d: they require an **unproxied loopback**. The
//! clean-room CI containers export `HTTP(S)_PROXY`, which curl honors even
//! for `127.0.0.1`, so it never reaches the in-process server and blocks
//! with no connect timeout (the suite then hangs to the 1h job limit). The
//! production `curl` calls intentionally omit `--noproxy` (real registries
//! may sit behind a proxy), and forcing `no_proxy` via process-global env is
//! unsafe under the parallel test runner — so these run locally / in
//! proxy-free environments via `cargo test -- --ignored`. (PMAT-088 / PR #153.)

use super::registry_push::*;
use crate::core::types::PushKind;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::JoinHandle;

/// A single HTTP request observed by the test server.
#[derive(Debug, Clone)]
struct RecordedRequest {
    method: String,
    path: String,
}

/// How the test server should respond to the next request.
#[derive(Clone)]
enum Reply {
    /// 202 Accepted with the given `Location` header (chunk accepted/resumption).
    Accepted { location: String },
    /// 202 Accepted with no `Location` header (client keeps the current URL).
    AcceptedNoLocation,
    /// 201 Created (upload finalized).
    Created,
    /// 500 Internal Server Error.
    ServerError,
}

/// Minimal in-process OCI push server.
///
/// Hands out one scripted [`Reply`] per incoming connection (curl opens a
/// fresh connection per invocation). Records every request for assertions.
struct OciTestServer {
    addr: String,
    requests: Receiver<RecordedRequest>,
    shutdown: Option<Sender<()>>,
    handle: Option<JoinHandle<()>>,
}

impl OciTestServer {
    /// Spawn a server that replies with `replies` in order, then 201 for any
    /// extra connections. Returns once it is bound and accepting.
    fn spawn(replies: Vec<Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
        let addr = listener.local_addr().expect("local_addr").to_string();
        Self::from_listener(listener, addr, replies)
    }

    /// Build a server around an already-bound listener. Lets a test learn the
    /// address first (to script absolute `Location` URLs) before serving.
    fn from_listener(listener: TcpListener, addr: String, replies: Vec<Reply>) -> Self {
        listener
            .set_nonblocking(false)
            .expect("set blocking accept");

        let (req_tx, req_rx) = mpsc::channel();
        let (stop_tx, stop_rx) = mpsc::channel();

        let handle = std::thread::spawn(move || serve_loop(listener, replies, &req_tx, &stop_rx));

        OciTestServer {
            addr,
            requests: req_rx,
            shutdown: Some(stop_tx),
            handle: Some(handle),
        }
    }

    /// Base URL for an upload session targeting this server.
    fn upload_url(&self) -> String {
        format!("http://{}/v2/test/repo/blobs/uploads/session-0", self.addr)
    }

    /// Drain all requests recorded so far.
    fn recorded(&self) -> Vec<RecordedRequest> {
        let mut out = Vec::new();
        while let Ok(r) = self
            .requests
            .recv_timeout(std::time::Duration::from_millis(200))
        {
            out.push(r);
        }
        out
    }
}

impl Drop for OciTestServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        // Nudge the accept loop out of its blocking accept().
        let _ = TcpStream::connect(&self.addr);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// Accept connections and reply with the scripted plan until shutdown.
fn serve_loop(
    listener: TcpListener,
    replies: Vec<Reply>,
    req_tx: &Sender<RecordedRequest>,
    stop_rx: &Receiver<()>,
) {
    for (idx, stream) in listener.incoming().enumerate() {
        if stop_rx.try_recv().is_ok() {
            break;
        }
        let Ok(stream) = stream else { break };
        let reply = replies.get(idx).cloned().unwrap_or(Reply::Created);
        if let Some(req) = handle_connection(stream, &reply) {
            let _ = req_tx.send(req);
        }
    }
}

/// Read one HTTP request, drain its body, and write the scripted reply.
fn handle_connection(stream: TcpStream, reply: &Reply) -> Option<RecordedRequest> {
    let mut reader = BufReader::new(stream);
    let (method, path, content_length) = read_request_head(&mut reader)?;
    drain_body(&mut reader, content_length);
    let response = render_reply(reply);
    let _ = reader.get_mut().write_all(response.as_bytes());
    let _ = reader.get_mut().flush();
    Some(RecordedRequest { method, path })
}

/// Parse the request line and `Content-Length` from the header block.
fn read_request_head(reader: &mut BufReader<TcpStream>) -> Option<(String, String, usize)> {
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).ok()? == 0 {
        return None;
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();

    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            break;
        }
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some(v) = trimmed.to_ascii_lowercase().strip_prefix("content-length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
    }
    Some((method, path, content_length))
}

/// Consume exactly `content_length` body bytes so curl's write completes.
fn drain_body(reader: &mut BufReader<TcpStream>, content_length: usize) {
    if content_length == 0 {
        return;
    }
    let mut body = vec![0u8; content_length];
    let _ = reader.read_exact(&mut body);
}

/// Serialize a [`Reply`] into an HTTP/1.1 response with `Connection: close`.
fn render_reply(reply: &Reply) -> String {
    match reply {
        Reply::Accepted { location } => format!(
            "HTTP/1.1 202 Accepted\r\nLocation: {location}\r\nRange: 0-0\r\n\
             Content-Length: 0\r\nConnection: close\r\n\r\n"
        ),
        Reply::AcceptedNoLocation => "HTTP/1.1 202 Accepted\r\nRange: 0-0\r\n\
             Content-Length: 0\r\nConnection: close\r\n\r\n"
            .to_string(),
        Reply::Created => {
            "HTTP/1.1 201 Created\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string()
        }
        Reply::ServerError => "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\
             Connection: close\r\n\r\n"
            .to_string(),
    }
}

/// Create a small temp blob file and a descriptor whose declared `size`
/// spans `chunks` PATCH iterations (the loop is driven by `size`, while curl
/// streams the real file). Returns the dir guard to keep the file alive.
fn make_blob(chunks: u64) -> (tempfile::TempDir, BlobDescriptor) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path: PathBuf = dir.path().join("layer.bin");
    std::fs::write(&path, b"oci-layer-bytes").expect("write blob");
    let blob = BlobDescriptor {
        digest: "sha256:deadbeef".into(),
        size: CHUNK_SIZE * chunks,
        path,
        kind: PushKind::Layer,
    };
    (dir, blob)
}

#[test]
#[ignore = "requires unproxied loopback — see module docs (PMAT-088)"]
fn chunked_push_single_chunk_happy_path() {
    // One PATCH (202, no Location => keep current URL) + finalize PUT (201).
    let server = OciTestServer::spawn(vec![Reply::AcceptedNoLocation, Reply::Created]);
    let url = server.upload_url();
    let (_dir, blob) = make_blob(1);

    let result = push_blob_chunked(&url, &blob);
    assert!(result.is_ok(), "happy path should succeed: {result:?}");

    let reqs = server.recorded();
    assert!(
        reqs.iter().any(|r| r.method == "PATCH"),
        "expected a PATCH chunk request, got {reqs:?}"
    );
    assert!(
        reqs.iter().any(|r| r.method == "PUT"),
        "expected a finalize PUT request, got {reqs:?}"
    );
}

#[test]
#[ignore = "requires unproxied loopback — see module docs (PMAT-088)"]
fn chunked_push_follows_location_for_resumption() {
    // Two-chunk blob: each PATCH hands back an absolute upload URL (as real
    // registries do) that the next request must follow — server-driven
    // resumption — then the finalize PUT lands on the last handed-back URL.
    // Bind first so the scripted Location URLs can point back at this server.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr").to_string();
    let loc_chunk2 = format!("http://{addr}/v2/test/repo/blobs/uploads/resumed-1");
    let loc_finalize = format!("http://{addr}/v2/test/repo/blobs/uploads/resumed-2");

    let server = OciTestServer::from_listener(
        listener,
        addr,
        vec![
            Reply::Accepted {
                location: loc_chunk2,
            },
            Reply::Accepted {
                location: loc_finalize,
            },
            Reply::Created,
        ],
    );
    let url = server.upload_url();
    let (_dir, blob) = make_blob(2);

    let result = push_blob_chunked(&url, &blob);
    assert!(result.is_ok(), "resumption path should succeed: {result:?}");

    let reqs = server.recorded();
    let patch_count = reqs.iter().filter(|r| r.method == "PATCH").count();
    assert_eq!(patch_count, 2, "two-chunk blob => two PATCHes: {reqs:?}");
    // The second PATCH must follow the Location handed back by the first.
    assert!(
        reqs.iter()
            .any(|r| r.method == "PATCH" && r.path.contains("resumed-1")),
        "second chunk must follow the first Location: {reqs:?}"
    );
    // The finalize PUT must follow the Location handed back by the last PATCH.
    assert!(
        reqs.iter()
            .any(|r| r.method == "PUT" && r.path.contains("resumed-2")),
        "finalize must follow the last Location: {reqs:?}"
    );
}

#[test]
#[ignore = "requires unproxied loopback — see module docs (PMAT-088)"]
fn chunked_push_errors_when_registry_unreachable() {
    // `.invalid` is RFC 6761 guaranteed non-resolvable, so curl fails DNS
    // resolution and exits non-zero, and the function must return Err. Unlike
    // a bind-then-drop port (which can race a concurrent test that reuses the
    // freed port), this is fully deterministic and hermetic.
    let url = "http://forjar-cov-registry.invalid/v2/test/repo/blobs/uploads/session-0";
    let (_dir, blob) = make_blob(1);

    let result = push_blob_chunked(url, &blob);
    assert!(
        result.is_err(),
        "unreachable registry must produce Err, got {result:?}"
    );
    let msg = result.unwrap_err();
    assert!(
        msg.contains("chunked upload"),
        "error should name the chunked-upload phase: {msg}"
    );
}

#[test]
#[ignore = "requires unproxied loopback — see module docs (PMAT-088)"]
fn chunked_push_succeeds_even_on_http_500_because_curl_silent() {
    // Documents the production behaviour: `curl -s` exits 0 on HTTP errors,
    // so a 500 does NOT surface as Err from push_blob_chunked. This locks in
    // the current contract (only transport failures error out).
    let server = OciTestServer::spawn(vec![Reply::ServerError, Reply::ServerError]);
    let url = server.upload_url();
    let (_dir, blob) = make_blob(1);

    let result = push_blob_chunked(&url, &blob);
    assert!(
        result.is_ok(),
        "curl -s masks HTTP 500; only transport errors fail: {result:?}"
    );
    let reqs = server.recorded();
    assert!(reqs.iter().any(|r| r.method == "PATCH"));
}
