//! The HTTP leg of the unified verb surface, proven from OUTSIDE the process.
//!
//! Every test here spawns `CARGO_BIN_EXE_forjar` and talks to it over a real
//! socket. Nothing calls into `forjar::verb::*` in-process, for the same reason
//! the CLI/MCP e2e does not: a parity suite compares transports to each other,
//! so it cannot notice that none of them is reachable. rmedia's four-way suite
//! was green for its entire life while `main.rs` routed only two of its four.
//!
//! The assertion that makes this more than three lists agreeing is
//! `http_and_cli_return_identical_bytes` — the renderer-fidelity gate. Parity of
//! NAMES is cheap; two transports can expose the same verb and disagree about
//! what it returns. So the same verb is invoked over both surfaces and the
//! results are byte-compared.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

/// `free_port` asks the OS for an ephemeral port and DROPS the listener before
/// returning, so two tests racing here can be handed the same port. Serialise
/// the whole pick-then-bind window. (Same defect, same fix, as the webhook
/// harness — see src/core/tests_webhook_server.rs.)
static PORT_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral")
        .local_addr()
        .expect("local_addr")
        .port()
}

struct Server {
    child: Child,
    port: u16,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn start_server() -> Server {
    let _guard = PORT_GUARD.lock().unwrap_or_else(|e| e.into_inner());
    let port = free_port();
    let child = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .args(["verb", "serve", "--port", &port.to_string()])
        .spawn()
        .expect("spawn forjar verb serve");

    // Take ownership BEFORE probing. If the probe times out and this function
    // panics, `Server::drop` still kills and reaps the child; leaving the
    // original code's `panic!` on the timeout path would leak a live server
    // holding a port, and the next test to be handed that port would fail for
    // a reason with no relation to its own subject.
    let srv = Server { child, port };

    // Probe by CONNECTING, never by binding: a probe that binds competes with
    // the server it is waiting for and can starve it.
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return srv;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("forjar verb serve did not accept a connection on port {port} within 20s");
}

fn request(port: u16, method: &str, path: &str, body: Option<&str>) -> (u16, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    s.set_read_timeout(Some(Duration::from_secs(20))).ok();
    let body = body.unwrap_or("");
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    s.write_all(req.as_bytes()).expect("write request");
    s.flush().ok();
    let mut raw = String::new();
    s.read_to_string(&mut raw).expect("read response");
    let status: u16 = raw
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or_else(|| panic!("no status line in response: {raw:?}"));
    let body = raw.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (status, body)
}

fn cli(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .args(args)
        .output()
        .expect("spawn forjar");
    assert!(
        out.status.success(),
        "forjar {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn the_http_surface_is_reachable() {
    let srv = start_server();
    let (status, _) = request(srv.port, "GET", "/healthz", None);
    assert_eq!(status, 200, "GET /healthz should be 200");
}

/// FVS-1, the HTTP leg: names as a CLIENT sees them.
#[test]
fn http_lists_the_same_verbs_as_the_cli() {
    let srv = start_server();
    let (status, body) = request(srv.port, "GET", "/v1/verbs", None);
    assert_eq!(status, 200, "GET /v1/verbs body={body}");

    let http: serde_json::Value = serde_json::from_str(&body).expect("http json");
    let cli_json: serde_json::Value =
        serde_json::from_str(&cli(&["verb", "list", "--json"])).expect("cli json");

    let names = |v: &serde_json::Value| -> Vec<String> {
        v["verbs"]
            .as_array()
            .expect("verbs array")
            .iter()
            .map(|r| r["name"].as_str().unwrap_or_default().to_string())
            .collect()
    };
    let h = names(&http);
    assert!(
        !h.is_empty(),
        "HTTP lists no verbs — parity would be vacuous"
    );
    assert_eq!(h, names(&cli_json), "HTTP and CLI expose different verbs");
}

/// THE renderer-fidelity gate. Two transports agreeing on NAMES is cheap; this
/// asserts they agree on the RESULT, byte for byte, for the same invocation.
#[test]
fn http_and_cli_return_identical_bytes() {
    let srv = start_server();
    let cfg = concat!(env!("CARGO_MANIFEST_DIR"), "/forjar.yaml");
    let params = format!("{{\"path\":{}}}", serde_json::Value::from(cfg));

    let (status, http_body) = request(srv.port, "POST", "/v1/verbs/validate", Some(&params));
    assert_eq!(status, 200, "POST /v1/verbs/validate body={http_body}");

    let cli_body = cli(&["verb", "call", "validate", "--json", &params]);

    // A genuine BYTE comparison, which the test's name claims. Comparing
    // parsed `serde_json::Value`s would pass even if one transport emitted
    // compact JSON and the other pretty — a real difference to any client
    // diffing output, and exactly the kind of drift this gate exists to catch.
    // The only permitted difference is the trailing newline `println!` adds.
    assert_eq!(
        http_body.trim_end(),
        cli_body.trim_end(),
        "the same verb rendered differently over HTTP and CLI:\n  http={http_body:?}\n  cli={cli_body:?}"
    );
    // ...and the bytes must still be JSON, not two identically-wrong strings.
    serde_json::from_str::<serde_json::Value>(http_body.trim_end()).expect("http body is JSON");
}

/// An unknown verb is 404, and does not echo the request back.
#[test]
fn unknown_verb_is_404() {
    let srv = start_server();
    let (status, body) = request(
        srv.port,
        "POST",
        "/v1/verbs/definitely-not-a-verb",
        Some("{}"),
    );
    assert_eq!(status, 404, "body={body}");
    assert!(
        !body.contains("definitely-not-a-verb"),
        "the response echoes request input back: {body}"
    );
}

/// FVS-2 over HTTP: params are validated before the handler runs.
#[test]
fn bad_params_are_400() {
    let srv = start_server();
    let (status, _) = request(
        srv.port,
        "POST",
        "/v1/verbs/validate",
        Some("{\"wrong\":1}"),
    );
    assert_eq!(status, 400, "invalid params must be 400, not 500");
}

/// GET on an invoke endpoint is 405 with Allow, per RFC 9110 9.5.
#[test]
fn get_on_invoke_is_405() {
    let srv = start_server();
    let (status, _) = request(srv.port, "GET", "/v1/verbs/validate", None);
    assert_eq!(status, 405);
}
