//! GH-228: the OCI registry push carries its own HTTP transport.
//!
//! Two claims, and each has a way to be wrong.
//!
//! CLAIM 1 — no external binary. Every registry verb used to be a `curl`
//! subprocess, which made an undeclared runtime dependency out of
//! `forjar build --push`: on a host without curl the first HEAD died as
//! `No such file or directory (os error 2)`, naming neither curl nor a missing
//! binary (GH-224 fixed the *message*; this fixes the dependency).
//! [`push_transport_spawns_no_external_binary`] scans the four transport
//! sources and fails with `file:line` for every process spawn it finds.
//!
//! CLAIM 2 — the two load-bearing gates survived the port. The old guards were
//! argv-shaped: unit tests asserted `--fail-with-body` appeared in a `Vec<String>`.
//! A rewrite deletes its own guardrails that way, so they are restated here as
//! behaviour against a live registry on loopback:
//!
//! * #154 — a registry that refuses the manifest is a FAILED push. The bug was
//!   a failed PUT reported as a successful push while the registry stored nothing.
//! * #210 — only 202 opens an upload session. `docker.io` answers the upload
//!   POST with a 301 to its marketing site; the old code took that redirect
//!   target as the session URL, PUT the blob at a web page, got 200, and
//!   reported a push. ureq follows up to 10 redirects **by default**, so this
//!   hole reopens the moment `max_redirects(0)` is dropped from the agent —
//!   which is exactly what [`a_301_on_the_upload_post_is_not_an_upload_session`]
//!   detects.
//!
//! Nothing here is `#[ignore]`d. The curl implementation could not run tests
//! like these in CI at all, because curl honors an ambient `HTTP(S)_PROXY` even
//! for `127.0.0.1`; the transport configures the proxy per agent and disables
//! it for loopback.

use forjar::core::store::registry_push::{
    push_blob, push_image, push_manifest, BlobDescriptor, RegistryPushConfig,
};
use forjar::core::store::registry_push_http::verify_manifest_pushed;
use forjar::core::types::PushKind;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

// ── Claim 1: the partition ───────────────────────────────────────────────

/// The four sources that make up the registry transport.
const TRANSPORT_SOURCES: [&str; 4] = [
    "src/core/store/registry_push.rs",
    "src/core/store/registry_push_http.rs",
    "src/core/store/registry_push_chunked.rs",
    "src/core/store/registry_http.rs",
];

/// THE FALSIFIER. RED on 1.20.1: seven `Command::new("curl")` call sites.
///
/// Scans for *any* process spawn, not for the string "curl": an exclusion list
/// that is green by construction proves nothing, and swapping curl for wget
/// would leave the undeclared-dependency defect exactly where it was.
#[test]
fn push_transport_spawns_no_external_binary() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut offenders = Vec::new();

    for rel in TRANSPORT_SOURCES {
        let path = root.join(rel);
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("the transport source {rel} must be readable: {e}"));
        for (n, line) in source.lines().enumerate() {
            if line.contains("Command::new(") {
                offenders.push(format!("{rel}:{}: {}", n + 1, line.trim()));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "`forjar build --push` must not depend on a binary it does not ship. \
         Process spawns still on the registry transport:\n{}",
        offenders.join("\n")
    );
}

// ── The loopback registry ────────────────────────────────────────────────

/// Decides the raw HTTP response for one (method, path).
type Router = Arc<dyn Fn(&str, &str) -> String + Send + Sync>;

/// A minimal OCI Distribution v1.1 endpoint on `127.0.0.1:0`.
struct TestRegistry {
    addr: String,
    seen: Arc<Mutex<Vec<(String, String)>>>,
    shutdown: Option<Sender<()>>,
    handle: Option<JoinHandle<()>>,
}

impl TestRegistry {
    fn spawn(router: Router) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
        let addr = listener.local_addr().expect("local_addr").to_string();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let (stop_tx, stop_rx) = mpsc::channel();

        let recorder = Arc::clone(&seen);
        let handle = std::thread::spawn(move || serve(listener, &router, &recorder, &stop_rx));

        TestRegistry {
            addr,
            seen,
            shutdown: Some(stop_tx),
            handle: Some(handle),
        }
    }

    /// Every (method, path) the registry has been asked for so far.
    fn requests(&self) -> Vec<(String, String)> {
        self.seen.lock().expect("recorder lock").clone()
    }

    fn saw(&self, method: &str, path_fragment: &str) -> bool {
        self.requests()
            .iter()
            .any(|(m, p)| m == method && p.contains(path_fragment))
    }
}

impl Drop for TestRegistry {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        let _ = TcpStream::connect(&self.addr);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn serve(
    listener: TcpListener,
    router: &Router,
    seen: &Arc<Mutex<Vec<(String, String)>>>,
    stop_rx: &Receiver<()>,
) {
    for stream in listener.incoming() {
        if stop_rx.try_recv().is_ok() {
            break;
        }
        let Ok(stream) = stream else { break };
        answer(stream, router, seen);
    }
}

/// Read one request, RECORD it, drain its body, and write the routed reply.
///
/// The record goes in before the reply is written. Recording after it (the
/// original order) raced the client: a caller that had already received its
/// response could run `saw()` before the recorder's push, and
/// `a_registry_that_rejects_the_manifest_is_a_failed_push` failed on a hosted
/// coverage runner with "the manifest PUT must actually have been attempted"
/// although the 401 it asserted on had been served (forjar#451's lint lane).
fn answer(stream: TcpStream, router: &Router, seen: &Arc<Mutex<Vec<(String, String)>>>) {
    let mut reader = BufReader::new(stream);
    let Some((method, path, length)) = read_head(&mut reader) else {
        return;
    };
    seen.lock()
        .expect("recorder lock")
        .push((method.clone(), path.clone()));
    if length > 0 {
        let mut body = vec![0u8; length];
        let _ = reader.read_exact(&mut body);
    }
    let response = router(&method, &path);
    let _ = reader.get_mut().write_all(response.as_bytes());
    let _ = reader.get_mut().flush();
}

fn read_head(reader: &mut BufReader<TcpStream>) -> Option<(String, String, usize)> {
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).ok()? == 0 {
        return None;
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();

    let mut length = 0usize;
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
            length = v.trim().parse().unwrap_or(0);
        }
    }
    Some((method, path, length))
}

fn status_only(line: &str) -> String {
    format!("HTTP/1.1 {line}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
}

fn with_body(line: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n{body}",
        body.len()
    )
}

// ── The fixture under push ───────────────────────────────────────────────

const MANIFEST_HEX: &str = "3333333333333333333333333333333333333333333333333333333333333333";

/// A structurally valid OCI layout: layer + config + manifest + index, so
/// `discover_blobs` classifies all three kinds and `push_image` proceeds.
fn write_oci_layout(oci: &Path) {
    let blobs = oci.join("blobs").join("sha256");
    std::fs::create_dir_all(&blobs).expect("create blobs dir");

    let layer_hex = "1".repeat(64);
    let config_hex = "2".repeat(64);

    std::fs::write(blobs.join(&layer_hex), b"layer-bytes").expect("write layer");
    let config = br#"{"architecture":"amd64","os":"linux"}"#;
    std::fs::write(blobs.join(&config_hex), config).expect("write config");

    let manifest = format!(
        r#"{{"schemaVersion":2,"mediaType":"application/vnd.oci.image.manifest.v1+json",
"config":{{"mediaType":"application/vnd.oci.image.config.v1+json","digest":"sha256:{config_hex}","size":{}}},
"layers":[{{"mediaType":"application/vnd.oci.image.layer.v1.tar+gzip","digest":"sha256:{layer_hex}","size":11}}]}}"#,
        config.len()
    );
    std::fs::write(blobs.join(MANIFEST_HEX), manifest.as_bytes()).expect("write manifest");

    let index = format!(
        r#"{{"schemaVersion":2,"manifests":[{{"mediaType":"application/vnd.oci.image.manifest.v1+json","digest":"sha256:{MANIFEST_HEX}","size":{}}}]}}"#,
        manifest.len()
    );
    std::fs::write(oci.join("index.json"), index.as_bytes()).expect("write index");
}

fn config_for(registry: &TestRegistry, check_existing: bool) -> RegistryPushConfig {
    RegistryPushConfig {
        registry: registry.addr.clone(),
        name: "team/app".into(),
        tag: "v1".into(),
        check_existing,
    }
}

// ── Claim 2: the behaviour ───────────────────────────────────────────────

/// A whole push, end to end, against a registry that behaves.
///
/// This is the guard a naive deletion of the argv tests cannot fake: it asserts
/// the registry was actually asked for each step of the protocol, in a shape it
/// recognises, and that the tag reads back afterwards.
#[test]
fn push_completes_end_to_end_against_a_loopback_registry() {
    let digest = format!("sha256:{MANIFEST_HEX}");
    let served = digest.clone();
    let registry = TestRegistry::spawn(Arc::new(move |method: &str, path: &str| {
        match (method, path) {
            ("HEAD", p) if p.contains("/manifests/") => format!(
                "HTTP/1.1 200 OK\r\nDocker-Content-Digest: {served}\r\n\
                 Content-Length: 0\r\nConnection: close\r\n\r\n"
            ),
            // Nothing is in this registry yet, so every blob is absent.
            ("HEAD", _) => status_only("404 Not Found"),
            ("POST", _) => "HTTP/1.1 202 Accepted\r\n\
                 Location: /v2/team/app/blobs/uploads/session-1?_state=x\r\n\
                 Content-Length: 0\r\nConnection: close\r\n\r\n"
                .to_string(),
            ("PUT", _) => status_only("201 Created"),
            _ => status_only("400 Bad Request"),
        }
    }));

    let dir = tempfile::tempdir().expect("tempdir");
    let oci = dir.path().join("oci");
    write_oci_layout(&oci);
    let config = config_for(&registry, true);

    let results = push_image(&oci, &config).expect("a well-behaved registry must accept the push");
    assert_eq!(results.len(), 3, "layer + config + manifest: {results:?}");

    verify_manifest_pushed(&config, &digest).expect("the tag must read back");

    let seen = registry.requests();
    assert!(
        registry.saw("HEAD", "/v2/team/app/blobs/sha256:"),
        "the existence check must ask about the blob by digest: {seen:?}"
    );
    assert!(
        registry.saw("POST", "/v2/team/app/blobs/uploads/"),
        "an upload session must be opened: {seen:?}"
    );
    assert!(
        registry.saw("PUT", "digest=sha256:"),
        "the blob PUT must carry the digest query the session URL needs: {seen:?}"
    );
    assert!(
        registry.saw("PUT", "/v2/team/app/manifests/v1"),
        "the manifest is PUT to the TAG, not uploaded as a blob: {seen:?}"
    );
    assert!(
        registry.saw("HEAD", "/v2/team/app/manifests/v1"),
        "the push must be verified by reading the tag back: {seen:?}"
    );
}

/// #154, restated behaviourally: a refused manifest is a refused push.
///
/// Replaces `manifest_put_argv_uses_fail_with_body`. RED against any port that
/// forgets to gate on the status, and the assertion on the body is what
/// `--fail-with-body` bought — the registry's own words, not just a number.
#[test]
fn a_registry_that_rejects_the_manifest_is_a_failed_push() {
    let registry = TestRegistry::spawn(Arc::new(|_method: &str, _path: &str| {
        with_body(
            "401 Unauthorized",
            r#"{"errors":[{"code":"UNAUTHORIZED","message":"authentication required"}]}"#,
        )
    }));
    let config = config_for(&registry, false);

    let err = push_manifest(&config, r#"{"schemaVersion":2}"#, "sha256:abc")
        .expect_err("a 401 stored nothing; reporting that as a push is defect #154");

    assert!(err.contains("401"), "the status must be named: {err}");
    assert!(
        err.contains("authentication required"),
        "the registry's own diagnostic must survive to the operator: {err}"
    );
    assert!(
        registry.saw("PUT", "/v2/team/app/manifests/v1"),
        "the manifest PUT must actually have been attempted"
    );
}

/// #210, and the specific guard for ureq's default redirect-following.
///
/// The registry answers the upload POST with a 301 whose target answers
/// `202 Accepted` with a `Location`. A client that follows the redirect sees a
/// valid-looking upload session and pushes the blob at it; a client that does
/// not sees the 301 and refuses. Loopback rather than `docker.com` so the test
/// needs no outbound network and cannot pass merely because the network is down.
#[test]
fn a_301_on_the_upload_post_is_not_an_upload_session() {
    let registry = TestRegistry::spawn(Arc::new(|method: &str, path: &str| {
        if method == "POST" && path.ends_with("/blobs/uploads/") {
            return "HTTP/1.1 301 Moved Permanently\r\n\
                 Location: /v2/team/app/blobs/uploads/decoy\r\n\
                 Content-Length: 0\r\nConnection: close\r\n\r\n"
                .to_string();
        }
        // The redirect target looks exactly like a healthy upload session.
        "HTTP/1.1 202 Accepted\r\nLocation: /v2/team/app/blobs/uploads/decoy\r\n\
         Content-Length: 0\r\nConnection: close\r\n\r\n"
            .to_string()
    }));

    let dir = tempfile::tempdir().expect("tempdir");
    let blob_path = dir.path().join("layer.tar");
    std::fs::write(&blob_path, b"layer-bytes").expect("write blob");
    let blob = BlobDescriptor {
        digest: "sha256:abc".into(),
        size: 11,
        path: blob_path,
        kind: PushKind::Layer,
    };

    let err = push_blob(&config_for(&registry, false), &blob)
        .expect_err("a redirect is not an upload session — Refs #210");

    assert!(
        err.contains("registry-1.docker.io"),
        "a 3xx on the upload POST must be reported as the wrong host: {err}"
    );
}
