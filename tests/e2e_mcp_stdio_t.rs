//! MCP transport, exercised through the SHIPPED BINARY over stdio.
//!
//! # Why this spawns `CARGO_BIN_EXE_forjar` and not the library
//!
//! forjar already had two MCP tests — `falsification_mcp_contract_coverage` and
//! `falsification_mcp_registry_image` — and neither references
//! `CARGO_BIN_EXE_`. They call into the library, so they can prove the handlers
//! agree with the registry while saying nothing about whether `forjar mcp` in a
//! released binary reaches them at all.
//!
//! That is not hypothetical. rmedia's four-way transport-parity suite was GREEN
//! for the entire period `mcp::serve_stdio` and `http::serve` had no caller from
//! `main.rs` — the transports agreed with each other perfectly and were
//! unreachable from the process entry point. Agreement cannot falsify
//! reachability.
//!
//! So every assertion below goes through the artifact a user installs.
//!
//! The stdio framing itself lives in `tests/common/mcp_stdio.rs`, shared with
//! `falsification_read_only_verbs_do_not_write` so the two suites cannot end up
//! interrogating differently-behaved servers.

#[path = "common/mcp_stdio.rs"]
mod mcp_stdio;

use mcp_stdio::McpServer;

/// The transport is REACHABLE from the process entry point.
#[test]
fn the_shipped_binary_answers_initialize() {
    let mut s = McpServer::spawn();
    let r = s.initialize();

    let info = r
        .pointer("/result/serverInfo/name")
        .and_then(|v| v.as_str())
        .expect("initialize returned no serverInfo.name — `forjar mcp` is not wired to a server");
    assert_eq!(info, "forjar-mcp");
    assert!(
        r.pointer("/result/capabilities/tools").is_some(),
        "server declares no tools capability, so tools/list is meaningless: {r}"
    );
}

/// `tools/list` advertises a non-empty surface.
///
/// Zero tools would make every assertion below vacuously true, which is the
/// "0 passed is not a pass" failure in miniature.
#[test]
fn tools_list_advertises_a_non_empty_surface() {
    let mut s = McpServer::spawn();
    s.initialize();
    let r = s.request(2, "tools/list", "{}");

    let tools = r
        .pointer("/result/tools")
        .and_then(|v| v.as_array())
        .expect("tools/list returned no tools array");
    assert!(
        !tools.is_empty(),
        "tools/list advertised zero tools — every downstream assertion would be vacuous"
    );

    for t in tools {
        let name = t.get("name").and_then(|v| v.as_str()).unwrap_or("");
        assert!(!name.is_empty(), "a tool has no name: {t}");
        assert!(
            t.get("inputSchema").is_some(),
            "tool `{name}` publishes no inputSchema — a client cannot know what to pass"
        );
    }
}

/// Every advertised tool must be DISPATCHABLE.
///
/// The failure this guards is a server whose `tools/list` and `tools/call`
/// disagree: the list is what an LLM reads and believes, so a name it
/// advertises that `tools/call` reports unknown is worse than not listing it.
/// (paiml/pforge#12 was exactly this, found the same way.)
#[test]
fn every_advertised_tool_is_dispatchable() {
    let mut s = McpServer::spawn();
    s.initialize();
    let listed = s.request(2, "tools/list", "{}");
    let tools: Vec<String> = listed
        .pointer("/result/tools")
        .and_then(|v| v.as_array())
        .expect("tools")
        .iter()
        .filter_map(|t| t.get("name")?.as_str().map(str::to_string))
        .collect();
    assert!(!tools.is_empty(), "no tools to dispatch");

    let mut unknown = Vec::new();
    for (i, name) in tools.iter().enumerate() {
        let r = s.request(
            100 + i as u64,
            "tools/call",
            &format!("{{\"name\":\"{name}\",\"arguments\":{{}}}}"),
        );
        // A tool may legitimately fail on empty arguments — that is an
        // application error and fine. What is NOT fine is the server not
        // knowing the name it just advertised.
        let msg = r
            .pointer("/error/message")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let code = r.pointer("/error/code").and_then(|v| v.as_i64());
        let unknown_method = code == Some(-32601)
            || msg.to_lowercase().contains("unknown tool")
            || msg.to_lowercase().contains("not found");
        if unknown_method {
            unknown.push(name.clone());
        }
    }
    assert!(
        unknown.is_empty(),
        "tools/list advertises {:?} but tools/call does not know them — a client reads the list and believes it",
        unknown
    );
}

/// A tool name that was never advertised must be refused, not silently accepted.
///
/// Guards the guard above: a server that answered everything would pass
/// `every_advertised_tool_is_dispatchable` while proving nothing.
#[test]
fn an_unadvertised_tool_is_refused() {
    let mut s = McpServer::spawn();
    s.initialize();
    let r = s.request(
        3,
        "tools/call",
        r#"{"name":"forjar_this_tool_does_not_exist","arguments":{}}"#,
    );
    assert!(
        r.get("error").is_some()
            || r.pointer("/result/isError").and_then(|v| v.as_bool()) == Some(true),
        "the server accepted a tool it never advertised: {r}"
    );
}

// ── NOT COVERED, named rather than implied ──────────────────────────────────
//
// LIFECYCLE: that the server exits when stdin closes.
//
// It does not. Written as a test first, and it FAILED: the process is still
// alive 10s after EOF, so every disconnected stdio client leaks a server. That
// is real and is filed as paiml/pforge#18 with the reproduction.
//
// It is not asserted here because forjar cannot fix it. `serve()` delegates to
// `pforge_runtime::McpServer::run()`, which delegates to pmcp's `run_stdio()` —
// two layers upstream, in crates forjar consumes as published versions. Gating
// forjar's release on a pmcp fix -> pforge release -> forjar bump chain would
// make a gate that cannot pass, and an unpassable gate trains people to bypass
// the whole protocol.
//
// Re-arm this the moment pforge#18 lands, by restoring the test that found it:
// spawn the binary, write one `initialize`, drop stdin, poll `try_wait` for 10s,
// and require exit 0.
