//! The unified verb surface is read-only BY CONSTRUCTION (paiml/forjar#356).
//!
//! `src/verb/registry.rs` has always SAID this:
//!
//! > it means an MCP agent may call any forjar verb unattended without risking
//! > a change to a machine
//!
//! and until this file it was only a comment. `Effects` is published to MCP as
//! `readOnlyHint`, which is the field an agent reads before deciding whether it
//! may invoke a tool without asking a human — so the guarantee is load-bearing
//! for every unattended caller, and a comment is not where a load-bearing
//! guarantee lives. `every_tool_publishes_a_read_only_hint` in
//! `src/mcp/tests_registry.rs` checks only that the hint EXISTS; a verb row
//! reading `Effects::Mutating` would publish `readOnlyHint: false` and pass it.
//!
//! WHAT THIS FILE IS FOR. Ending the guarantee is a legitimate decision — the
//! epic that motivated #356 proposes `apply`, `undo` and `remediate` behind a
//! `FORJAR_ENABLE_MUTATING_MCP` flag. It is not a decision that should happen as
//! a side effect of adding a row to a table. This test is the interlock: adding
//! a mutating verb turns it red, and a human has to come here and say so.
//!
//! Usage: cargo test --test falsification_verb_readonly_surface

use forjar::mcp::export_schema;
use forjar::verb::{verbs, Effects};

/// REJECTION CRITERION: any verb declaring `Effects::Mutating`.
#[test]
fn every_verb_on_the_unified_surface_is_read_only() {
    let mutating: Vec<&str> = verbs()
        .iter()
        .filter(|v| !v.effects.read_only())
        .map(|v| v.name)
        .collect();
    assert!(
        mutating.is_empty(),
        "{:?} declare Effects::Mutating.\n\n\
         The unified verb surface is read-only by construction: MCP publishes \
         `readOnlyHint: true` for every tool, and an agent trusts that before \
         calling one unattended. Adding a mutating verb ENDS that standing \
         guarantee for the whole surface, not just for the new verb — a client \
         that could previously treat `forjar_*` as safe can no longer do so \
         without inspecting each tool.\n\n\
         If that is the intended change, it is a human's to make: say so here, \
         and say so in the surface's documentation and in \
         contracts/verb-surface-v1.yaml at the same time.",
        mutating
    );
}

/// The published annotation must be `true`, not merely present.
#[test]
fn every_published_tool_publishes_read_only_hint_true() {
    let schema = export_schema();
    let tools = schema["tools"].as_array().expect("tools array");
    for t in tools {
        assert_eq!(
            t["annotations"]["readOnlyHint"],
            serde_json::Value::Bool(true),
            "{} publishes readOnlyHint {:?} — an agent reads this field to \
             decide whether it may call the tool unattended",
            t["name"],
            t["annotations"]["readOnlyHint"]
        );
    }
}

/// Vacuity guard. Both assertions above are trivially true of an empty surface,
/// and a registry that silently collapses is exactly how a green suite ends up
/// certifying nothing.
#[test]
fn the_surface_is_not_empty() {
    assert!(
        verbs().len() >= 9,
        "the verb registry has {} entries — the read-only assertions above \
         become vacuous when it empties",
        verbs().len()
    );
    let n = export_schema()["tools"]
        .as_array()
        .map(Vec::len)
        .unwrap_or_default();
    assert_eq!(
        n,
        verbs().len(),
        "export_schema drifted from the verb table"
    );
}

/// The interlock only works if `Effects::Mutating` can actually be
/// distinguished. If `read_only()` returned `true` unconditionally, every
/// assertion in this file would pass over a surface that mutates machines.
#[test]
fn effects_actually_discriminates() {
    assert!(Effects::ReadOnly.read_only());
    assert!(
        !Effects::Mutating.read_only(),
        "readOnlyHint is derived from this; if it cannot say `false` then the \
         hint is a constant and the whole annotation is theatre"
    );
}
