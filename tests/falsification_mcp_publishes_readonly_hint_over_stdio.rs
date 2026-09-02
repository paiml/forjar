//! `readOnlyHint` reaches a CONNECTED AGENT, not just `--schema`
//! (paiml/forjar#375).
//!
//! `src/verb/spec.rs` defines `Effects::ReadOnly` as "safe for an agent to call
//! unattended", and `src/verb/registry.rs` rests its whole argument on an agent
//! consulting the hint before deciding it needs a human. The hint was injected
//! in `export_schema()` — the `--schema` path — and nowhere else. Measured on
//! 1.24.0, driving real stdio (`initialize` → `notifications/initialized` →
//! `tools/list`):
//!
//! ```text
//!   tools/list count: 12
//!   keys of every tool object: ['description', 'inputSchema', 'name']
//!   forjar_remediate -> readOnlyHint None,  outputSchema ABSENT   (all 12)
//! ```
//!
//! while `forjar mcp --schema` on the same binary carried
//! `annotations.readOnlyHint: true` for all twelve. The two paths disagreed and
//! the one that disagreed is the one clients use.
//!
//! SIX files assert `readOnlyHint` and every one of them reads `export_schema()`
//! or `mcp --schema`; the one suite that drives real `tools/list`
//! (`e2e_mcp_stdio_t`) asserted only `name` and `inputSchema`. Agreement between
//! two non-wire surfaces cannot falsify what goes over the wire — the same
//! reachability argument that suite's header makes about `serve_stdio` having no
//! caller.
//!
//! ORDER IS NONDETERMINISTIC. pmcp caches tool metadata in a `HashMap`, so
//! `tools/list` comes back scrambled (measured: `remediate, trace, plan, lint,
//! …`). Every assertion below is keyed by NAME; a positional zip against
//! `verbs()` would flake for a reason unrelated to the property.
//!
//! Usage: cargo test --test falsification_mcp_publishes_readonly_hint_over_stdio

#[path = "common/mcp_stdio.rs"]
mod mcp_stdio;

use mcp_stdio::McpServer;
use std::collections::BTreeMap;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_forjar");

/// The tool objects a connected client actually receives.
fn wire_tools() -> Vec<serde_json::Value> {
    let mut s = McpServer::spawn();
    s.initialize();
    let r = s.request(2, "tools/list", "{}");
    let tools = r
        .pointer("/result/tools")
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| panic!("tools/list returned no tools array: {r}"))
        .clone();
    // Non-vacuity: a server advertising nothing satisfies every per-tool
    // assertion below trivially.
    assert_eq!(
        tools.len(),
        forjar::verb::verbs().len(),
        "tools/list advertised {} tools for a {}-row verb table — every \
         per-tool assertion here would be vacuous",
        tools.len(),
        forjar::verb::verbs().len()
    );
    tools
}

fn name_of(t: &serde_json::Value) -> String {
    t.get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("a tool object has no name: {t}"))
        .to_string()
}

/// REJECTION CRITERION: a tool advertised over stdio with no read-only
/// annotation at all.
///
/// PRESENCE, not `Some(true)`. Asserting the literal would hard-code the very
/// thing the fix must never hard-code — the value has to come from
/// `Effects::read_only()`, so that the first `Mutating` row to join the table
/// publishes `false` rather than a comfortable lie. #356 established that a
/// wrong hint is worse than a missing one. The VALUE is checked in the next
/// test, against the declaration.
#[test]
fn every_tool_sends_a_read_only_hint_over_the_wire() {
    let tools = wire_tools();

    let silent: Vec<String> = tools
        .iter()
        .filter(|t| {
            t.pointer("/annotations/readOnlyHint")
                .and_then(serde_json::Value::as_bool)
                .is_none()
        })
        .map(name_of)
        .collect();

    assert!(
        silent.is_empty(),
        "these tools send no `annotations.readOnlyHint` over stdio: {silent:?}\n\
         An agent cannot consult a field it never receives, which is the whole \
         basis on which `Effects::ReadOnly` claims a verb is safe to call \
         unattended."
    );
}

/// REJECTION CRITERION, and the one a `--schema`-only fix cannot satisfy: the
/// wire and `forjar mcp --schema` disagreeing about any tool.
///
/// Name-keyed maps compared whole, so a tool present on one surface and absent
/// from the other fails too.
#[test]
fn the_wire_and_the_schema_agree_per_tool() {
    let out = Command::new(BIN)
        .args(["mcp", "--schema"])
        .output()
        .expect("run `forjar mcp --schema`");
    assert!(out.status.success(), "`forjar mcp --schema` failed");
    let schema: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("`mcp --schema` printed JSON");

    let from_schema: BTreeMap<String, Option<bool>> = schema["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("--schema has no tools array: {schema}"))
        .iter()
        .map(|t| {
            (
                name_of(t),
                t.pointer("/annotations/readOnlyHint")
                    .and_then(serde_json::Value::as_bool),
            )
        })
        .collect();

    let from_wire: BTreeMap<String, Option<bool>> = wire_tools()
        .iter()
        .map(|t| {
            (
                name_of(t),
                t.pointer("/annotations/readOnlyHint")
                    .and_then(serde_json::Value::as_bool),
            )
        })
        .collect();

    assert!(
        from_schema.values().all(Option::is_some),
        "the schema side lost its annotations, so the comparison below would \
         be two absences agreeing: {from_schema:?}"
    );
    assert_eq!(
        from_wire, from_schema,
        "`forjar mcp --schema` and a real `tools/list` disagree about \
         `readOnlyHint`. The one that disagrees is the one clients read."
    );
}

/// REJECTION CRITERION: no `outputSchema` on the wire.
///
/// The same line in the adapter dropped both fields, so shipping one and not
/// the other is arbitrary — and `output_schema` is already derived from the
/// verb's `$output` type, the type the handler's success value serialises
/// through, so publishing it costs nothing and lets a client type-check what it
/// gets back.
#[test]
fn the_wire_carries_an_output_schema() {
    let tools = wire_tools();

    let silent: Vec<String> = tools
        .iter()
        .filter(|t| {
            !t.get("outputSchema")
                .is_some_and(serde_json::Value::is_object)
        })
        .map(name_of)
        .collect();

    assert!(
        silent.is_empty(),
        "these tools publish no `outputSchema` over stdio: {silent:?}"
    );
}

/// REJECTION CRITERION: a server that advertises beautifully and dispatches
/// nothing.
///
/// This is here because the obvious simplification of the adapter — wiring the
/// verb table's own JSON-in/JSON-out `invoke` into it, since one already exists
/// — satisfies all three tests above perfectly and then panics on every
/// `tools/call` with "Cannot start a runtime from within a runtime":
/// `VerbSpec::invoke` builds a `tokio::runtime::Runtime` internally and the
/// adapter's `handle()` is already running on one. Dispatch must stay on
/// `HandlerRegistry::dispatch`, and that is load-bearing rather than incidental.
#[test]
fn every_advertised_tool_still_dispatches() {
    let names: Vec<String> = wire_tools().iter().map(name_of).collect();

    let mut s = McpServer::spawn();
    s.initialize();
    let mut unknown = Vec::new();
    for (i, name) in names.iter().enumerate() {
        let r = s.call_tool(100 + i as u64, name, &serde_json::json!({}));
        // Failing on empty arguments is an application error and fine. A server
        // that does not know a name it just advertised is not.
        let msg = r
            .pointer("/error/message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_lowercase();
        let code = r.pointer("/error/code").and_then(serde_json::Value::as_i64);
        if code == Some(-32601) || msg.contains("unknown tool") || msg.contains("not found") {
            unknown.push(name.clone());
        }
        assert!(
            !msg.contains("cannot start a runtime from within a runtime"),
            "`{name}` panicked inside the async adapter: the dispatch path was \
             rewired through `VerbSpec::invoke`, which builds its own tokio \
             runtime: {r}"
        );
    }

    assert!(
        unknown.is_empty(),
        "tools/list advertises {unknown:?} but tools/call does not know them"
    );
}
