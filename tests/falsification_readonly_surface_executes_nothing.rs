//! forjar#372: `forjar_plan` published `readOnlyHint: true` and executed
//! config-declared subprocesses.
//!
//! `src/verb/registry.rs` declares all nine verbs `Effects::ReadOnly`;
//! `src/verb/spec.rs` says that means "safe for an agent to call unattended",
//! and the registry says an agent "may call any forjar verb unattended without
//! risking a change to a machine". Three ordinary config keys made that false
//! for `plan`:
//!
//! ```text
//!   PlanHandler -> planner::plan -> core::task::probe_config
//!               -> task::ambient::hash_declared_inputs
//!               -> Command::new("bash").arg("-c")      src/core/task/ambient.rs
//!
//!               -> planner resolve_or_fallback
//!               -> resolver::template::resolve_secret_sops
//!               -> Command::new("sops")                src/core/resolver/template.rs
//!
//!               -> core::task::probe_resource -> hash_outputs_with
//!               -> Command::new("bash").arg("-c")      src/core/task/output_hash.rs
//! ```
//!
//! So an agent asked to INSPECT an untrusted repository executed whatever that
//! repository declared. No flag, nothing to opt into — just a config path.
//!
//! # Why this drives the real stdio server
//!
//! forjar already has MCP tests that call the library. They can prove the
//! handlers agree with the registry while saying nothing about what the shipped
//! `forjar mcp` actually runs. And `tests/e2e_mcp_stdio_t.rs` does spawn the
//! binary — but its fixture declares no `ambient_inputs` and no
//! `secrets.provider`, so it passed throughout the defect's life. **A fixture
//! that does not exercise the path proves nothing.** Everything below runs
//! against a HOSTILE config through `CARGO_BIN_EXE_forjar mcp`.
//!
//! `control_the_traps_fire_through_the_cli` is the guard on the guard: it runs
//! the same fixture through `forjar plan`, which still probes by design, and
//! requires every trap to fire. Without it, "no trap fired" could mean the
//! traps were never armed.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

/// The three traps, one per subprocess the plan path could reach.
const TRAPS: [&str; 3] = ["AMBIENT_FIRED", "SOPS_FIRED", "EQUIV_FIRED"];

/// A config that tries to make a read verb run code.
struct Hostile {
    dir: PathBuf,
}

impl Hostile {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("forjar-372-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("bin")).expect("sandbox");
        let me = Self { dir };
        me.write_fake_sops();
        me.write_config();
        // The `output_equivalence` normaliser only runs for an artifact that
        // EXISTS, so the fixture has to provide one.
        std::fs::write(me.dir.join("out.txt"), "artifact\n").expect("artifact");
        me
    }

    fn cfg(&self) -> PathBuf {
        self.dir.join("forjar.yaml")
    }

    fn trap(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    /// `sops` is resolved from PATH, so a stub proves the spawn without needing
    /// the real tool installed.
    fn write_fake_sops(&self) {
        let sops = self.dir.join("bin/sops");
        std::fs::write(
            &sops,
            format!(
                "#!/bin/sh\ntouch '{}'\necho decrypted\n",
                self.trap("SOPS_FIRED").display()
            ),
        )
        .expect("stub sops");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&sops, std::fs::Permissions::from_mode(0o755))
                .expect("chmod stub");
        }
    }

    fn write_config(&self) {
        let d = self.dir.display();
        let cfg = format!(
            r#"version: "1.0"
name: hostile
secrets:
  provider: sops
  file: secrets.enc.yaml
machines:
  sandbox:
    hostname: sandbox
    addr: 127.0.0.1
resources:
  build-thing:
    type: task
    machine: sandbox
    working_dir: {d}
    command: "echo hi"
    task_inputs: ["forjar.yaml"]
    ambient_inputs:
      - "touch {d}/AMBIENT_FIRED; echo v1"
    output_artifacts: ["out.txt"]
    output_equivalence:
      out.txt: !command "touch {d}/EQUIV_FIRED; cat $1"
  needs-secret:
    type: file
    machine: sandbox
    path: {d}/never-written.txt
    content: "{{{{secrets.API_KEY}}}}"
"#
        );
        std::fs::write(self.cfg(), cfg).expect("config");
    }

    fn disarm(&self) {
        for t in TRAPS {
            let _ = std::fs::remove_file(self.trap(t));
        }
    }

    fn fired(&self) -> Vec<&'static str> {
        TRAPS
            .into_iter()
            .filter(|t| self.trap(t).exists())
            .collect()
    }

    fn command(&self) -> Command {
        let mut c = Command::new(FORJAR);
        c.current_dir(&self.dir);
        let path = std::env::var("PATH").unwrap_or_default();
        c.env("PATH", format!("{}/bin:{path}", self.dir.display()));
        c
    }

    fn mcp(&self) -> McpServer {
        McpServer::spawn(self.command())
    }
}

impl Drop for Hostile {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A live `forjar mcp` server speaking JSON-RPC over stdio.
///
/// Shaped after `tests/e2e_mcp_stdio_t.rs`. It kills rather than waits on drop:
/// the stdio server does not exit on EOF (pforge#18, documented there), so a
/// polite shutdown would hang this suite.
struct McpServer {
    child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
}

impl McpServer {
    fn spawn(mut cmd: Command) -> Self {
        let mut child = cmd
            .arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn `forjar mcp`");
        let stdin = child.stdin.take().expect("stdin");
        let out = BufReader::new(child.stdout.take().expect("stdout"));
        let mut me = Self { child, stdin, out };
        me.request(
            1,
            "initialize",
            r#"{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"f372","version":"0"}}"#,
        );
        me.notify("notifications/initialized");
        me
    }

    fn request(&mut self, id: u64, method: &str, params: &str) -> serde_json::Value {
        let msg = format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"{method}\",\"params\":{params}}}\n"
        );
        self.stdin.write_all(msg.as_bytes()).expect("write");
        self.stdin.flush().expect("flush");
        for _ in 0..64 {
            let mut line = String::new();
            let n = self.out.read_line(&mut line).expect("read");
            assert!(n > 0, "server closed stdout awaiting id={id} ({method})");
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line.trim()) else {
                continue;
            };
            if v.get("id").and_then(serde_json::Value::as_u64) == Some(id) {
                return v;
            }
        }
        panic!("no reply with id={id} for {method}");
    }

    fn notify(&mut self, method: &str) {
        let msg = format!("{{\"jsonrpc\":\"2.0\",\"method\":\"{method}\"}}\n");
        self.stdin.write_all(msg.as_bytes()).expect("write");
        self.stdin.flush().expect("flush");
    }

    fn call(&mut self, id: u64, tool: &str, cfg: &Path) -> serde_json::Value {
        let args = format!("{{\"path\":\"{}\"}}", cfg.display());
        self.request(
            id,
            "tools/call",
            &format!("{{\"name\":\"{tool}\",\"arguments\":{args}}}"),
        )
    }

    fn tool_names(&mut self) -> Vec<String> {
        self.request(2, "tools/list", "{}")
            .pointer("/result/tools")
            .and_then(serde_json::Value::as_array)
            .expect("tools/list returned no array")
            .iter()
            .filter_map(|t| Some(t.get("name")?.as_str()?.to_string()))
            .collect()
    }
}

impl Drop for McpServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The JSON a `tools/call` result carries in its single text content block.
fn tool_json(reply: &serde_json::Value) -> serde_json::Value {
    let text = reply
        .pointer("/result/content/0/text")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("no text content in tool reply: {reply}"));
    serde_json::from_str(text).unwrap_or_else(|e| panic!("tool text is not JSON ({e}): {text}"))
}

// ── The control ─────────────────────────────────────────────────────────────

/// The traps ARE armed: the same fixture, through the CLI, fires all three.
///
/// `forjar plan` probes by design and that is deliberately unchanged — the
/// operator who typed it chose their own config. This test exists so that "no
/// trap fired" below cannot silently mean "the fixture was inert".
#[test]
fn control_the_traps_fire_through_the_cli() {
    let h = Hostile::new("control");
    h.disarm();
    let out = h
        .command()
        .args(["plan", "-f", h.cfg().to_str().unwrap()])
        .output()
        .expect("run forjar plan");
    let fired = h.fired();
    assert_eq!(
        fired.len(),
        TRAPS.len(),
        "the CLI must still execute all three (it is the surface a human drives); \
         fired {fired:?}, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ── The defect ──────────────────────────────────────────────────────────────

/// `forjar_plan` over real MCP stdio must execute nothing the config declares.
///
/// On 1.21.0 this fires AMBIENT_FIRED, SOPS_FIRED and EQUIV_FIRED.
#[test]
fn forjar_plan_over_mcp_stdio_executes_nothing_the_config_declares() {
    let h = Hostile::new("plan");
    h.disarm();
    let cfg = h.cfg();
    let reply = {
        let mut s = h.mcp();
        s.call(10, "forjar_plan", &cfg)
    };
    assert!(
        reply.get("result").is_some(),
        "forjar_plan must still answer over stdio, not error out: {reply}"
    );
    assert_eq!(
        h.fired(),
        Vec::<&str>::new(),
        "forjar_plan publishes readOnlyHint:true, and an agent trusts that before \
         calling it unattended — it must not run what an untrusted repository declares"
    );
}

/// Not just `plan`: no verb on the unattended surface may execute config-declared
/// code. This is the guard against the next handler that starts probing.
#[test]
fn no_advertised_verb_executes_what_the_config_declares() {
    let h = Hostile::new("allverbs");
    let cfg = h.cfg();
    let mut s = h.mcp();
    let tools = s.tool_names();
    assert!(
        tools.len() >= 9,
        "the surface advertised {} tools — an empty list makes this vacuous",
        tools.len()
    );
    for (i, tool) in tools.iter().enumerate() {
        h.disarm();
        // A verb may legitimately fail on this input; only the traps matter.
        let _ = s.call(200 + i as u64, tool, &cfg);
        assert_eq!(
            h.fired(),
            Vec::<&str>::new(),
            "{tool} executed something the config declared"
        );
    }
}

// ── The disclosure ──────────────────────────────────────────────────────────

/// Skipping silently would be its own defect: the plan is now lock-relative for
/// whatever it declined to run, and only the caller can decide what to do about
/// that. So it must SAY so, and name what it skipped.
#[test]
fn the_plan_reports_what_it_declined_to_execute() {
    let h = Hostile::new("disclose");
    let cfg = h.cfg();
    let out = {
        let mut s = h.mcp();
        tool_json(&s.call(11, "forjar_plan", &cfg))
    };

    let skipped = out["unattended_skipped"]
        .as_array()
        .unwrap_or_else(|| panic!("plan carries no `unattended_skipped`: {out}"));
    let joined = skipped
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>()
        .join(" | ");
    for expected in ["ambient_inputs", "sops", "output_equivalence"] {
        assert!(
            joined.contains(expected),
            "the disclosure must name {expected}, got: {joined}"
        );
    }

    let disclosure = out["disclosure"].as_str().unwrap_or_else(|| {
        panic!(
            "skipped {} things and disclosed nothing: {out}",
            skipped.len()
        )
    });
    assert!(
        disclosure.contains("forjar drift"),
        "the disclosure must name the command that CAN answer: {disclosure}"
    );
    assert_eq!(
        out["lock_relative"],
        serde_json::Value::Bool(true),
        "a plan that skipped probes is lock-relative: {out}"
    );
}

/// The guard against "fixed" meaning "always disclose", which trains an agent to
/// ignore the field. A config that declares nothing executable must skip
/// nothing — the unattended plan and `forjar plan` compute the same thing.
#[test]
fn a_harmless_config_skips_nothing_and_discloses_nothing() {
    let h = Hostile::new("harmless");
    let cfg = h.dir.join("clean.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: clean\nmachines:\n  sandbox:\n    hostname: sandbox\n\
             \x20   addr: 127.0.0.1\nresources:\n  a-file:\n    type: file\n\
             \x20   machine: sandbox\n    path: {}/clean.txt\n    content: \"declared\"\n",
            h.dir.display()
        ),
    )
    .expect("clean config");

    let out = {
        let mut s = h.mcp();
        tool_json(&s.call(12, "forjar_plan", &cfg))
    };
    assert_eq!(
        out["unattended_skipped"],
        serde_json::json!([]),
        "nothing executable was declared, so nothing may be reported skipped: {out}"
    );
    assert!(
        out.get("disclosure").is_none() || out["disclosure"].is_null(),
        "a plan with no blind spot must not manufacture one: {out}"
    );
}
