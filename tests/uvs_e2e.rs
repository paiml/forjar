//! GH-267 — the Unified Verb Surface, exercised through the SHIPPED BINARY.
//!
//! # Why every test here spawns a process
//!
//! paiml/rmedia#247 shipped a four-way transport-parity harness that was green
//! for the entire period `mcp::serve_stdio` and `http::serve` had no caller from
//! `main.rs`. The transports agreed with each other perfectly, and none of them
//! could be reached by running the program. An in-process harness cannot see
//! that, because it calls the functions directly — it proves the code is
//! correct, never that it is *wired*.
//!
//! So every test in this file starts `CARGO_BIN_EXE_forjar` and talks to it the
//! way a user or an agent would: argv and exit codes for the CLI, a TCP socket
//! for HTTP, stdin/stdout pipes for MCP. If a transport loses its entry point
//! in `main`, these go red.
//!
//! Exit codes are read from `Output::status`, never through a shell pipeline.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Output, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_forjar");

fn run(args: &[&str]) -> Output {
    Command::new(BIN).args(args).output().expect("spawn forjar")
}

fn stdout_of(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

// ── The registry is reachable from the entry point ──────────────────

#[test]
fn the_binary_exposes_the_serve_verb() {
    let o = run(&["--help"]);
    assert!(o.status.success(), "forjar --help must succeed");
    assert!(
        stdout_of(&o).contains("serve"),
        "`serve` is missing from the shipped help; the verb is not wired"
    );
}

#[test]
fn mcp_schema_emits_the_derived_catalogue_not_a_hand_written_one() {
    let o = run(&["mcp", "--schema"]);
    assert!(o.status.success(), "mcp --schema must succeed");
    let v: serde_json::Value = serde_json::from_str(&stdout_of(&o)).expect("schema is JSON");
    let n = v["verb_count"].as_u64().expect("verb_count") as usize;
    assert!(
        n > 150,
        "the derived catalogue must carry the whole surface, got {n}"
    );
    assert_eq!(v["verbs"].as_array().unwrap().len(), n);
}

#[test]
fn the_legacy_mcp_schema_is_still_reachable_and_is_the_smaller_one() {
    // The 1.x surface stays available for one release. It must remain wired —
    // a flag that silently does nothing is worse than a removed flag.
    let o = run(&["mcp", "--schema", "--legacy"]);
    assert!(o.status.success());
    let v: serde_json::Value = serde_json::from_str(&stdout_of(&o)).expect("legacy schema is JSON");
    assert_eq!(v["tool_count"], 9, "the legacy server had nine tools");
}

// ── CLI ──────────────────────────────────────────────────────────────

#[test]
fn every_verb_in_the_manifest_is_a_real_subcommand_of_the_shipped_binary() {
    // The manifest is generated from the clap tree in-process. This checks the
    // other direction, against the artifact that actually ships: each name is
    // accepted by the real argv parser.
    let manifest = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/surface-manifest.txt"
    ))
    .expect("surface manifest must be committed");

    let names: Vec<String> = manifest
        .lines()
        .filter_map(|l| l.strip_prefix('['))
        .filter_map(|l| l.split(']').next())
        .map(str::to_string)
        .collect();
    assert!(names.len() > 150, "manifest looks empty: {}", names.len());

    for name in &names {
        let o = run(&[name, "--help"]);
        assert!(
            o.status.success(),
            "`forjar {name} --help` failed — the manifest names a verb the binary does not have"
        );
    }
}

#[test]
fn an_unknown_verb_is_rejected_by_the_shipped_binary() {
    // Proves the sweep above can fail: the same check, on a name that is not a
    // verb, does not succeed.
    let o = run(&["definitely-not-a-verb", "--help"]);
    assert!(!o.status.success());
}

// ── HTTP ─────────────────────────────────────────────────────────────

/// A `forjar serve` child, killed on drop.
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

fn start_server(extra: &[&str]) -> Server {
    // Port 0 is not supported by the verb, so pick a free one by binding and
    // releasing — a race window exists but is tiny and retried below.
    let port = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").expect("probe port");
        l.local_addr().unwrap().port()
    };
    let mut args = vec!["serve", "--port"];
    let port_s = port.to_string();
    args.push(&port_s);
    args.extend_from_slice(extra);

    let child = Command::new(BIN)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn forjar serve");

    let server = Server { child, port };
    // Wait for the listener rather than sleeping a fixed time.
    for _ in 0..100 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return server;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("forjar serve did not start listening on port {port}");
}

fn http(port: u16, method: &str, path: &str, body: Option<&str>) -> (u16, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let b = body.unwrap_or("");
    let req = format!(
        "{method} {path} HTTP/1.1\r\nhost: localhost\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{b}",
        b.len()
    );
    s.write_all(req.as_bytes()).expect("write request");
    s.flush().unwrap();
    let mut raw = String::new();
    s.read_to_string(&mut raw).expect("read response");
    let (head, body) = raw.split_once("\r\n\r\n").expect("well-formed response");
    let status: u16 = head
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    (status, body.to_string())
}

fn http_json(port: u16, method: &str, path: &str, body: Option<&str>) -> (u16, serde_json::Value) {
    let (s, b) = http(port, method, path, body);
    (
        s,
        serde_json::from_str(&b).unwrap_or(serde_json::Value::Null),
    )
}

#[test]
fn http_health_answers_from_the_spawned_server() {
    let srv = start_server(&[]);
    let (status, body) = http_json(srv.port, "GET", "/health", None);
    assert_eq!(status, 200);
    assert_eq!(body["status"], "ok");
    assert!(body["verb_count"].as_u64().unwrap() > 150);
}

#[test]
fn http_lists_and_describes_verbs() {
    let srv = start_server(&[]);
    let (status, body) = http_json(srv.port, "GET", "/v1/verbs", None);
    assert_eq!(status, 200);
    assert!(body["verbs"].as_array().unwrap().len() > 150);

    let (status, body) = http_json(srv.port, "GET", "/v1/verbs/plan", None);
    assert_eq!(status, 200);
    assert_eq!(body["name"], "plan");
    assert_eq!(body["effects"], "read-only");

    let (status, _) = http_json(srv.port, "GET", "/v1/verbs/nope", None);
    assert_eq!(status, 404);
}

#[test]
fn http_invokes_a_verb_and_the_output_equals_the_cli() {
    // The parity claim, measured end to end: the HTTP body and the CLI stdout
    // come from two separate process invocations of the shipped binary.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("forjar.yaml"), FIXTURE).unwrap();

    let srv = start_server(&[]);
    let (status, body) = http_json(
        srv.port,
        "POST",
        "/v1/validate",
        Some(&format!(
            r#"{{"file": "{}"}}"#,
            dir.path().join("forjar.yaml").display()
        )),
    );
    assert_eq!(status, 200);

    let cli = Command::new(BIN)
        .args(["validate", "--file"])
        .arg(dir.path().join("forjar.yaml"))
        .output()
        .expect("cli validate");

    assert_eq!(
        body["stdout"].as_str().unwrap(),
        stdout_of(&cli),
        "HTTP and CLI must produce identical output for the same verb"
    );
    assert_eq!(
        body["exit_code"].as_i64().unwrap() as i32,
        cli.status.code().unwrap(),
        "HTTP must report the CLI's exit code"
    );
}

#[test]
fn http_rejects_bad_params_with_400_and_a_transport_verb_with_403() {
    let srv = start_server(&[]);
    let (status, body) = http_json(srv.port, "POST", "/v1/plan", Some(r#"{"bogus": 1}"#));
    assert_eq!(status, 400, "unknown parameter must be a client error");
    assert_eq!(body["error"]["kind"], "invalid_params");

    let (status, _) = http_json(srv.port, "POST", "/v1/serve", Some("{}"));
    assert_eq!(status, 403, "serving `serve` would recurse without bound");

    let (status, _) = http_json(srv.port, "POST", "/v1/nope", Some("{}"));
    assert_eq!(status, 404);

    let (status, _) = http_json(srv.port, "DELETE", "/v1/plan", None);
    assert_eq!(status, 405);
}

#[test]
fn http_read_only_mode_refuses_mutating_verbs() {
    let srv = start_server(&["--read-only"]);
    let (status, body) = http_json(srv.port, "POST", "/v1/apply", Some("{}"));
    assert_eq!(status, 403);
    assert_eq!(body["error"]["kind"], "not_invocable");

    // and still admits a read-only one
    let (status, _) = http_json(srv.port, "POST", "/v1/schema", Some("{}"));
    assert_eq!(status, 200);
}

// ── MCP ──────────────────────────────────────────────────────────────

/// Drive `forjar mcp` over real pipes and collect one response per request.
fn mcp_exchange(requests: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut child = Command::new(BIN)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn forjar mcp");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for r in requests {
            writeln!(stdin, "{r}").expect("write request");
        }
        stdin.flush().unwrap();
    }
    // Dropping stdin closes it, ending the server's loop.
    drop(child.stdin.take());

    let stdout = child.stdout.take().expect("stdout");
    let responses: Vec<serde_json::Value> = BufReader::new(stdout)
        .lines()
        .map_while(Result::ok)
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(&l).expect("each response line is JSON"))
        .collect();
    let _ = child.wait();
    responses
}

fn rpc(id: u32, method: &str, params: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
}

#[test]
fn mcp_initialize_and_tools_list_over_real_pipes() {
    let r = mcp_exchange(&[
        rpc(1, "initialize", serde_json::json!({})),
        rpc(2, "tools/list", serde_json::json!({})),
    ]);
    assert_eq!(r.len(), 2, "expected one response per request: {r:?}");
    assert_eq!(r[0]["id"], 1);
    assert_eq!(r[0]["result"]["serverInfo"]["name"], "forjar");

    let tools = r[1]["result"]["tools"].as_array().unwrap();
    assert!(
        tools.len() > 140,
        "the MCP surface must be the whole registry, got {}",
        tools.len()
    );
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"plan"));
    assert!(names.contains(&"lock-verify"));
    assert!(!names.contains(&"serve"), "transports must not be listed");
}

#[test]
fn mcp_tools_call_output_equals_the_cli() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("forjar.yaml");
    std::fs::write(&cfg, FIXTURE).unwrap();

    let r = mcp_exchange(&[rpc(
        1,
        "tools/call",
        serde_json::json!({
            "name": "validate",
            "arguments": { "file": cfg.to_str().unwrap() }
        }),
    )]);
    assert_eq!(r.len(), 1);
    let env = &r[0]["result"]["structuredContent"];

    let cli = Command::new(BIN)
        .args(["validate", "--file"])
        .arg(&cfg)
        .output()
        .unwrap();

    assert_eq!(
        env["stdout"].as_str().unwrap(),
        stdout_of(&cli),
        "MCP and CLI must produce identical output"
    );
    assert_eq!(
        env["exit_code"].as_i64().unwrap() as i32,
        cli.status.code().unwrap()
    );
    assert_eq!(r[0]["result"]["isError"], !cli.status.success());
}

#[test]
fn mcp_reports_protocol_errors_with_jsonrpc_codes() {
    let r = mcp_exchange(&[
        rpc(1, "no/such/method", serde_json::json!({})),
        rpc(2, "tools/call", serde_json::json!({"name": "no-such-verb"})),
        rpc(
            3,
            "tools/call",
            serde_json::json!({"name": "plan", "arguments": {"bogus": 1}}),
        ),
        rpc(4, "tools/call", serde_json::json!({"name": "serve"})),
    ]);
    assert_eq!(r.len(), 4);
    assert_eq!(r[0]["error"]["code"], -32601, "unknown method");
    assert_eq!(r[1]["error"]["code"], -32601, "unknown tool");
    assert_eq!(r[2]["error"]["code"], -32602, "invalid params");
    assert_eq!(r[3]["error"]["code"], -32601, "transport verb refused");
}

// ── Three-way agreement, measured on the shipped binary ─────────────

#[test]
fn cli_http_and_mcp_agree_on_every_read_only_verb_they_share() {
    // The conformance claim. Each of the three numbers below comes from a
    // separate process invocation, so agreement here cannot be an artifact of
    // sharing an in-process code path.
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("forjar.yaml");
    std::fs::write(&cfg, FIXTURE).unwrap();
    let file = cfg.to_str().unwrap().to_string();

    let srv = start_server(&[]);

    for verb in ["validate", "show", "graph", "lint", "score"] {
        let cli = Command::new(BIN)
            .args([verb, "--file", &file])
            .output()
            .unwrap_or_else(|e| panic!("cli {verb}: {e}"));

        let (status, http_body) = http_json(
            srv.port,
            "POST",
            &format!("/v1/{verb}"),
            Some(&serde_json::json!({ "file": file }).to_string()),
        );
        assert_eq!(status, 200, "{verb} over HTTP");

        let mcp = mcp_exchange(&[rpc(
            1,
            "tools/call",
            serde_json::json!({"name": verb, "arguments": {"file": file}}),
        )]);
        let mcp_env = &mcp[0]["result"]["structuredContent"];

        let expected = stdout_of(&cli);
        assert_eq!(
            http_body["stdout"].as_str().unwrap(),
            expected,
            "HTTP disagrees with CLI for `{verb}`"
        );
        assert_eq!(
            mcp_env["stdout"].as_str().unwrap(),
            expected,
            "MCP disagrees with CLI for `{verb}`"
        );
        let code = cli.status.code().unwrap();
        assert_eq!(
            http_body["exit_code"].as_i64().unwrap() as i32,
            code,
            "{verb}"
        );
        assert_eq!(
            mcp_env["exit_code"].as_i64().unwrap() as i32,
            code,
            "{verb}"
        );
    }
}

const FIXTURE: &str = r#"version: '1.0'
name: uvs-e2e
params: {}
machines:
  local:
    hostname: sandbox-local
    addr: 127.0.0.1
    user: nobody
    arch: x86_64
resources:
  hello:
    type: file
    machine: local
    path: /tmp/uvs-e2e-hello.txt
    content: hi
"#;
