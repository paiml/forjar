//! `Effects::ReadOnly` is a PROMISE ABOUT BEHAVIOUR. This file drives it.
//!
//! # Why the existing tests could not catch #356
//!
//! `read_only_hint_is_derived_from_effects` asserts that `Effects::ReadOnly`
//! maps to `true`, and `every_verb_on_the_unified_surface_is_read_only` asserts
//! that every row in the table carries that variant. Both are assertions about
//! the DECLARATION. Neither invokes anything, so both stayed green while
//! `forjar_lint` grew a `policy_dir` parameter that fed compliance packs to
//! `sh -c`: a pack rule of `script: "touch <path>"` created that file through a
//! real `tools/call` over stdio, and the reply was
//! `{"gate_passed":true,"error_count":0,"warnings":[]}` with nothing in it
//! saying a script had run — while `readOnlyHint: true` went on being
//! published. A schema description could not have fixed that, because the hint
//! is machine-read and the description is not.
//!
//! So the assertion here is not "the table says ReadOnly". It is: spawn the
//! SHIPPED binary inside a booby-trapped directory, call every verb it
//! advertises, and fail if a single byte of that directory moves.
//!
//! # The trap has to be proved to work first
//!
//! A fixture whose pack is malformed, or whose script bashrs rejects before
//! execution, produces no file for the happiest of reasons and makes every
//! assertion below vacuous. `the_trap_fires_through_the_cli` runs the same pack
//! through `forjar lint --policy-dir` — the surface where an operator has typed
//! a flag and opted in — and REQUIRES the sentinel to appear. If that test ever
//! goes green-by-inertia, this whole file is worthless and says so.

#[path = "common/mcp_stdio.rs"]
mod mcp_stdio;

use mcp_stdio::McpServer;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The file a compliance pack's `type: script` rule creates when it executes.
const SENTINEL: &str = "PACK_SCRIPT_FIRED";

/// A config with something for every verb to chew on, plus an `ambient_inputs`
/// command — the other declarable way a forjar run shells out.
const CONFIG: &str = r#"version: '1.0'
name: read-only-probe
machines:
  local:
    hostname: localhost
    addr: local
    user: root
    arch: x86_64
    transport: local
resources:
  motd:
    type: file
    machine: local
    path: /etc/motd
    content: "welcome"
    tags: [probe]
  build:
    type: task
    machine: local
    command: "echo build"
    ambient_inputs:
      - "touch AMBIENT_FIRED; echo v1"
"#;

/// A compliance pack whose rule RUNS SHELL. `touch` is deliberately the whole
/// payload: it is the minimum observable side effect, and anything bigger would
/// make a failure of this test harder to read, not more convincing.
const EVIL_PACK: &str = r#"name: evil-pack
version: "1.0"
framework: TEST
rules:
  - id: EVIL-1
    title: This rule executes
    severity: warning
    type: script
    script: "touch PACK_SCRIPT_FIRED"
"#;

/// A fresh booby-trapped tree. Returned by value so the tempdir outlives use.
fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("forjar.yaml"), CONFIG).unwrap();
    std::fs::create_dir(dir.path().join("policies")).unwrap();
    std::fs::write(dir.path().join("policies/evil.yaml"), EVIL_PACK).unwrap();
    dir
}

/// Every path under `root`, with the bytes of every file.
///
/// Content and not just names: a verb that rewrote a lock file in place would
/// leave the tree the same shape, and "the shape is unchanged" is the weaker
/// claim of the two.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut acc = BTreeMap::new();
    walk(root, root, &mut acc);
    acc
}

fn walk(root: &Path, dir: &Path, acc: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read {dir:?}: {e}"));
    for entry in entries.flatten() {
        let path = entry.path();
        let key = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
        if path.is_dir() {
            acc.insert(key, None);
            walk(root, &path, acc);
        } else {
            acc.insert(key, Some(std::fs::read(&path).unwrap_or_default()));
        }
    }
}

/// Report the difference between two snapshots in a form a human can act on.
fn diff(
    before: &BTreeMap<PathBuf, Option<Vec<u8>>>,
    after: &BTreeMap<PathBuf, Option<Vec<u8>>>,
) -> Vec<String> {
    let mut out = Vec::new();
    for (path, content) in after {
        match before.get(path) {
            None => out.push(format!("CREATED {}", path.display())),
            Some(old) if old != content => out.push(format!("MODIFIED {}", path.display())),
            Some(_) => {}
        }
    }
    for path in before.keys() {
        if !after.contains_key(path) {
            out.push(format!("DELETED {}", path.display()));
        }
    }
    out
}

// ── The trap, proved to work ────────────────────────────────────────────────

/// The pack executes when a surface hands it to the gate.
///
/// This is the falsifiability check for everything below. `forjar lint
/// --policy-dir` is the OPERATOR surface: the flag's own help text says a pack
/// rule of `type: script` runs its shell, so an operator who types it has opted
/// in and this is not a defect — it is the control that proves the trap is
/// armed. It is also the reason `--policy-dir` was left on the CLI when the
/// field was removed from `LintInput`.
#[test]
fn the_trap_fires_through_the_cli() {
    let dir = fixture();
    let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .current_dir(dir.path())
        .args(["lint", "-f", "forjar.yaml", "--policy-dir", "policies"])
        .output()
        .expect("spawn forjar lint");

    assert!(
        dir.path().join(SENTINEL).exists(),
        "the booby-trapped pack did NOT execute through `forjar lint --policy-dir`, \
         so every assertion in this file is vacuous — the pack is probably malformed, \
         or bashrs rejected the script before it ran.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

// ── The property ────────────────────────────────────────────────────────────

/// No verb the MCP server advertises writes anything, however it is called.
///
/// The server is spawned INSIDE the fixture, so a verb writing to a relative
/// path writes somewhere the snapshot is looking.
#[test]
fn no_advertised_verb_writes_to_the_filesystem() {
    let dir = fixture();
    let cfg = dir.path().join("forjar.yaml").display().to_string();
    let mut s = McpServer::spawn_in(dir.path());
    s.initialize();

    let listed = s.tools_list(2);
    let tools: Vec<String> = listed
        .pointer("/result/tools")
        .and_then(|v| v.as_array())
        .expect("tools/list returned no tools array")
        .iter()
        .filter_map(|t| t.get("name")?.as_str().map(str::to_string))
        .collect();
    assert!(
        tools.len() >= 9,
        "the server advertised {} tools; the loop below would be near-vacuous",
        tools.len()
    );

    let before = snapshot(dir.path());

    // Pass A, hostile: every parameter name that has ever pointed the gate at
    // something executable, whether or not this verb declares it. A verb that
    // ignores them is fine; a verb that acts on them is the bug.
    let hostile = serde_json::json!({
        "path": cfg,
        "policy_dir": dir.path().join("policies").display().to_string(),
        "max_cyclomatic": 1,
        "state_dir": dir.path().join("state").display().to_string(),
    });
    // Pass B, minimal: guaranteed to deserialise, so the verb actually RUNS its
    // real work rather than being refused at the parameter check. Without this,
    // a server that rejected everything would pass.
    let minimal = serde_json::json!({ "path": cfg });

    for (i, name) in tools.iter().enumerate() {
        s.call_tool(200 + i as u64, name, &hostile);
        s.call_tool(400 + i as u64, name, &minimal);
    }

    let changes = diff(&before, &snapshot(dir.path()));
    assert!(
        changes.is_empty(),
        "a verb published with `readOnlyHint: true` changed the filesystem — an \
         agent calls these unattended on the strength of that hint:\n  {}",
        changes.join("\n  ")
    );
    assert!(
        !dir.path().join(SENTINEL).exists(),
        "the compliance pack executed through the MCP surface — this is #356 exactly"
    );
    assert!(
        !dir.path().join("AMBIENT_FIRED").exists(),
        "an ambient_inputs command ran from the verb surface"
    );
}

/// The #356 regression on its own, named so a failure reads as itself.
///
/// Separate from the sweep above because a sweep failure could be any verb and
/// any file; this one says which parameter, on which tool, over which transport.
#[test]
fn lint_does_not_run_compliance_packs_over_stdio() {
    let dir = fixture();
    let mut s = McpServer::spawn_in(dir.path());
    s.initialize();

    let reply = s.call_tool(
        3,
        "forjar_lint",
        &serde_json::json!({
            "path": dir.path().join("forjar.yaml").display().to_string(),
            "policy_dir": dir.path().join("policies").display().to_string(),
        }),
    );

    assert!(
        !dir.path().join(SENTINEL).exists(),
        "`forjar_lint` executed a pack script named by its own arguments, and \
         answered {reply}"
    );
}

// ── The declaration, as a cheap tripwire ────────────────────────────────────

/// No advertised input schema offers a parameter that names a policy directory.
///
/// Strictly weaker than the tests above — it reads a declaration, and the whole
/// point of #356 is that declarations were what everything already checked. It
/// earns its place only as the tripwire that names the exact mistake if someone
/// re-adds the field, so the failure message beats "some file appeared".
#[test]
fn no_advertised_input_schema_offers_a_policy_dir() {
    let mut s = McpServer::spawn();
    s.initialize();
    let listed = s.tools_list(2);
    let tools = listed
        .pointer("/result/tools")
        .and_then(|v| v.as_array())
        .expect("tools")
        .clone();
    assert!(!tools.is_empty(), "no tools advertised");

    let offenders: Vec<String> = tools
        .iter()
        .filter(|t| {
            t.pointer("/inputSchema/properties/policy_dir").is_some()
                || t.pointer("/input_schema/properties/policy_dir").is_some()
        })
        .filter_map(|t| t.get("name")?.as_str().map(str::to_string))
        .collect();

    assert!(
        offenders.is_empty(),
        "{offenders:?} publish a `policy_dir` parameter. A compliance pack rule of \
         `type: script` is handed to `sh -c`, so this makes the tool's \
         `readOnlyHint: true` false. Keep it a CLI flag."
    );
}
