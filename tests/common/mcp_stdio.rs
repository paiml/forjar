//! A live `forjar mcp` server speaking JSON-RPC over stdio, driven through the
//! SHIPPED BINARY.
//!
//! Shared by `e2e_mcp_stdio_t` and `falsification_read_only_verbs_do_not_write`
//! so the two cannot drift into asking a differently-behaved server the same
//! questions — and so neither has to carry a second copy of the framing.
//!
//! Everything here spawns `CARGO_BIN_EXE_forjar`. That is the point and not a
//! stylistic preference: forjar's other MCP tests call into the library, so
//! they can prove the handlers agree with the registry while saying nothing
//! about whether `forjar mcp` in a released binary reaches them at all.
//! rmedia's four-way transport-parity suite was GREEN for the entire period
//! `mcp::serve_stdio` had no caller from `main.rs`. Agreement cannot falsify
//! reachability.

// Each including test binary uses a different subset of this surface.
#![allow(unused)]

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

/// A spawned `forjar mcp` process plus its framed stdio.
pub struct McpServer {
    child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
}

impl McpServer {
    /// Spawn the shipped binary with the test's own working directory.
    pub fn spawn() -> Self {
        Self::spawn_impl(None)
    }

    /// Spawn it with `cwd` as its working directory.
    ///
    /// Worth having separately: a verb that writes to a RELATIVE path writes
    /// wherever the server happens to be standing, so a test that watches a
    /// fixture tree has to put the server inside it or the write lands
    /// somewhere the snapshot never looks.
    pub fn spawn_in(cwd: &Path) -> Self {
        Self::spawn_impl(Some(cwd))
    }

    fn spawn_impl(cwd: Option<&Path>) -> Self {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_forjar"));
        cmd.arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        let mut child = cmd
            .spawn()
            .expect("failed to spawn the release binary — `forjar mcp` is advertised in --help");
        let stdin = child.stdin.take().expect("stdin");
        let out = BufReader::new(child.stdout.take().expect("stdout"));
        Self { child, stdin, out }
    }

    /// Send a request and read replies until the matching id arrives.
    ///
    /// Reads until the id matches rather than assuming the next line is the
    /// answer: notifications and log lines may be interleaved, and a test that
    /// assumes ordering fails for a reason that has nothing to do with the
    /// property under test.
    pub fn request(&mut self, id: u64, method: &str, params: &str) -> serde_json::Value {
        let msg = format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"{method}\",\"params\":{params}}}\n"
        );
        self.stdin.write_all(msg.as_bytes()).expect("write");
        self.stdin.flush().expect("flush");

        for _ in 0..64 {
            let mut line = String::new();
            let n = self.out.read_line(&mut line).expect("read");
            assert!(
                n > 0,
                "server closed stdout while awaiting id={id} ({method})"
            );
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line.trim()) else {
                continue;
            };
            if v.get("id").and_then(|i| i.as_u64()) == Some(id) {
                return v;
            }
        }
        panic!("no reply with id={id} for {method}");
    }

    pub fn notify(&mut self, method: &str) {
        let msg = format!("{{\"jsonrpc\":\"2.0\",\"method\":\"{method}\"}}\n");
        self.stdin.write_all(msg.as_bytes()).expect("write");
        self.stdin.flush().expect("flush");
    }

    pub fn initialize(&mut self) -> serde_json::Value {
        let r = self.request(
            1,
            "initialize",
            r#"{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"e2e","version":"0"}}"#,
        );
        self.notify("notifications/initialized");
        r
    }

    /// The raw `tools/list` reply.
    pub fn tools_list(&mut self, id: u64) -> serde_json::Value {
        self.request(id, "tools/list", "{}")
    }

    /// One `tools/call`, with `arguments` supplied as JSON.
    pub fn call_tool(
        &mut self,
        id: u64,
        name: &str,
        arguments: &serde_json::Value,
    ) -> serde_json::Value {
        let params = serde_json::json!({ "name": name, "arguments": arguments });
        self.request(id, "tools/call", &params.to_string())
    }
}

impl Drop for McpServer {
    fn drop(&mut self) {
        // Close stdin first: a well-behaved stdio server exits on EOF, which is
        // the lifecycle property worth having. Kill only if it does not.
        // (It does not — see the closing note in `e2e_mcp_stdio_t`.)
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
