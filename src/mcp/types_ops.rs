//! Input/output types for the verbs discharged from `Bucket::Pending`.
//!
//! Split out of `types.rs` only for the 500-line file cap — the surface has no
//! two halves. `types.rs` re-exports everything here, so every consumer still
//! writes `use crate::mcp::types::*` and the verb table still names one path.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── policy-coverage (FJ-3208) ───────────────────────────────────────

/// MCP policy-coverage handler input.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PolicyCoverageInput {
    /// Path to forjar.yaml
    pub path: String,
}

/// MCP policy-coverage handler output.
///
/// NOT a struct of its own: this is `core::policy_coverage::PolicyCoverage`,
/// the exact value `forjar policy-coverage --json` prints. A copy of its fields
/// would be a second place for the answer to live, and that is precisely the
/// defect #356 found here — the verb was wired to a calculation with no
/// production caller while its documentation named the CLI's, and the two
/// disagreed about which resources were covered.
///
/// Two renderers over one calculation. There is no second type to drift.
pub type PolicyCoverageOutput = crate::core::policy_coverage::PolicyCoverage;

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
    /// It is reported because NOTHING ON THIS SURFACE RESOLVES IT. The CLI
    /// commands that take `--workspace` (`apply`, `plan`, `drift`, `lock`)
    /// join the active workspace onto their state dir in
    /// `cli::workspace::resolve_state_dir`; the verbs go through
    /// `mcp::paths::resolve_state_dir*`, which never does. A caller that wants
    /// the view those CLI commands have under this selection must pass this
    /// path back as the next verb's `state_dir`.
    ///
    /// MEASURED against this binary, one project, `.forjar/workspace = prod`
    /// and the lock under `state/prod/`:
    ///
    /// ```text
    ///   $ forjar plan -f forjar.yaml
    ///   state: <root>/state/prod
    ///   Plan: 0 to add, 0 to change, 0 to destroy, 1 unchanged.
    ///
    ///   $ forjar verb call plan --json '{"path":"<root>/forjar.yaml"}'
    ///   { "to_create": 1, "unchanged": 0, ... }
    /// ```
    ///
    /// and `verb call audit` on the same config reports `event_count: 0` over
    /// a four-event trail in `state/prod/local/events.jsonl`. That divergence
    /// is pre-existing and is filed as paiml/forjar#367; this field is not the
    /// fix, it is the fact stated where a caller reading the report will see
    /// it. (`forjar audit` takes no `--workspace` and does not join it either,
    /// so it prints "No audit events found." on the same project.)
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
