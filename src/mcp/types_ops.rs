//! Input/output types for the verbs discharged from `Bucket::Pending`.
//!
//! Split out of `types.rs` only for the 500-line file cap — the surface has no
//! two halves. `types.rs` re-exports everything here, so every consumer still
//! writes `use crate::mcp::types::*` and the verb table still names one path.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── policy-coverage: NOT HERE ───────────────────────────────────────
//
// `PolicyCoverageInput` and `PolicyCoverageOutput` lived here. The verb was
// withdrawn (paiml/forjar#369) and the types went with it: the output was a
// `pub type` alias for `core::policy_coverage::PolicyCoverage`, so nothing was
// lost that the CLI does not still print, and an input type with no handler is
// a schema for a tool that is not published.
//
// `core::policy_coverage` is unchanged and still has exactly one calculation.
// Re-shipping the verb once #369 is fixed is these two declarations, one
// handler, one `register_all` line and one `verb_table!` row.

// ── audit (FJ-341) ──────────────────────────────────────────────────

/// MCP audit handler input.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuditInput {
    /// Path to the project's forjar.yaml. Optional, but without it the tool
    /// cannot know WHICH project it is being asked about and falls back to
    /// `./state` relative to the server's cwd — which for an MCP stdio server
    /// is chosen by the client and is arbitrary (GH-208).
    pub path: Option<String>,
    /// State directory (default: `<config dir>/state`)
    pub state_dir: Option<String>,
    /// Filter to a specific machine
    pub machine: Option<String>,
    /// Return at most this many entries, newest first (default: 20)
    pub limit: Option<usize>,
}

/// MCP audit handler output.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AuditOutput {
    /// Number of entries returned — which is `min(limit, events on disk)`, not
    /// the size of the trail.
    pub event_count: usize,
    /// The entries, newest first.
    pub events: Vec<AuditEventOutput>,
}

/// One entry of the append-only provenance log.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AuditEventOutput {
    /// Machine whose log the entry came from.
    pub machine: String,
    /// ISO 8601 timestamp.
    pub timestamp: String,
    /// The event, SERIALISED — not `Debug`-printed into a string. `forjar audit
    /// --json` shipped `"event": "ApplyStarted { machine: \\\"local\\\", ... }"`
    /// for three releases, so run_id, operator, config_hash, resource and
    /// action were unreadable without re-parsing Rust Debug syntax out of a
    /// JSON string, in a document that exists to be machine-read.
    pub event: serde_json::Value,
}

// ── workspace (FJ-210) ──────────────────────────────────────────────

/// MCP workspace handler input.
///
/// There is no `op` field, and that is the point: `new`, `select` and `delete`
/// mutate, and this surface is read-only by construction (see
/// `src/verb/registry.rs`). What this verb reports is the selection recorded in
/// `.forjar/workspace` and the directories under the state base — two facts
/// about the project, in one call.
///
/// It does NOT report where the other verbs read state. See
/// [`WorkspaceOutput::workspace_state_dir`] for what was measured.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct WorkspaceInput {
    /// Path to the project's forjar.yaml. The active workspace is recorded in
    /// `.forjar/workspace` BESIDE it; without this the tool reads the server's
    /// cwd, which the client chose (GH-208).
    pub path: Option<String>,
    /// State directory holding the per-workspace subdirectories
    /// (default: `<config dir>/state`)
    pub state_dir: Option<String>,
}

/// MCP workspace handler output.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceOutput {
    /// The workspace selected in `.forjar/workspace`, or `null` when none is
    /// selected and state lives directly under the state base. `null` is not
    /// "unknown" — it is the default workspace, and it is the case `forjar
    /// workspace current` prints as `(default — no workspace selected)`.
    ///
    /// This is a REPORT of a selection, not a setting that binds anything on
    /// this surface — see [`Self::workspace_state_dir`].
    pub active: Option<String>,
    /// Every workspace directory under the state base, sorted by name.
    pub workspaces: Vec<WorkspaceEntryOutput>,
    /// The state base actually inspected — `<config dir>/state` unless
    /// `state_dir` said otherwise.
    ///
    /// This is derived from the arguments alone and is the same string whether
    /// the directory exists or not, which is why it cannot on its own tell an
    /// empty list from a wrong path. [`Self::state_base_exists`] is the half
    /// that can.
    pub state_base: String,
    /// Whether `state_base` exists on disk.
    ///
    /// This is what separates "no workspaces yet" (`true`, and `workspaces` is
    /// empty because nothing has been applied) from "that is not the project"
    /// (`false`). `list_workspaces_in` returns an empty list for both, so
    /// without this field the two are indistinguishable in the report.
    pub state_base_exists: bool,
    /// The directory the workspace selection designates: `state_base` joined
    /// with `active`, or `state_base` itself when nothing is selected.
    ///
    /// Since paiml/forjar#367 this is also the directory the OTHER verbs read
    /// when they are given no explicit `state_dir` — `mcp::paths::resolve_state_dir`
    /// joins the same marker file the CLI's `--workspace` resolution does, so
    /// the two surfaces no longer answer differently about one project. Before
    /// that fix they did, silently:
    ///
    /// ```text
    ///   $ forjar plan -f forjar.yaml                            1 unchanged
    ///   $ forjar verb call plan --json '{"path":"<root>/forjar.yaml"}'
    ///     { "to_create": 1, "unchanged": 0 }     # CREATE for a converged file
    /// ```
    ///
    /// The field is still worth reporting, for two reasons that survive the
    /// fix. An agent that has to NAME the directory the CLI is working in has
    /// no other way to ask. And a caller that wants to pin the read — to a
    /// workspace that is not the selected one, or against a selection that
    /// might change under it — passes this path back as the next verb's
    /// `state_dir`, which is honoured verbatim and never joined onto again.
    ///
    /// `state_base` is deliberately NOT this path: it is the directory
    /// workspaces live under, which is what `workspaces` above enumerates.
    pub workspace_state_dir: String,
}

/// One workspace directory.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceEntryOutput {
    /// Directory name under the state base.
    pub name: String,
    /// Whether this is the selected workspace.
    pub active: bool,
}
