//! MCP Input/Output type definitions for forjar handlers.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// FVS (#356): the types for the verbs discharged from `Bucket::Pending` live in
// `types_ops.rs` for the 500-line file cap. Re-exported here so there remains
// exactly ONE path a consumer imports.
pub use super::types_ops::*;

// ── Input / Output types ────────────────────────────────────────────

/// MCP validate handler input.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ValidateInput {
    /// Path to forjar.yaml
    pub path: String,
}

/// MCP validate handler output.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ValidateOutput {
    /// Whether the config is valid.
    pub valid: bool,
    /// Number of resources in the config.
    pub resource_count: usize,
    /// Number of machines in the config.
    pub machine_count: usize,
    /// Validation error messages.
    pub errors: Vec<String>,
}

/// MCP plan handler input.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PlanInput {
    /// Path to forjar.yaml
    pub path: String,
    /// State directory (default: "state")
    pub state_dir: Option<String>,
    /// Filter to a specific resource. The counts below describe the FILTERED
    /// set, and an id that is not in the config is an error (GH-214).
    pub resource: Option<String>,
    /// Filter by tag
    pub tag: Option<String>,
}

/// MCP plan handler output.
#[derive(Debug, Serialize, JsonSchema)]
pub struct PlanOutput {
    /// Planned resource changes.
    pub changes: Vec<PlannedChangeOutput>,
    /// Count of resources to create.
    pub to_create: u32,
    /// Count of resources to update.
    pub to_update: u32,
    /// Count of resources to destroy.
    pub to_destroy: u32,
    /// Count of unchanged resources.
    pub unchanged: u32,
    /// forjar#342: this report compares the config to the LOCK and contacts no
    /// machine. Always true — it is a statement of the quantifier, not a mode.
    pub lock_relative: bool,
    /// How many locked resources carry state observed on a target that this
    /// plan did not consult. Always present, including at zero, so a consumer
    /// can tell "nothing observed" from "an older binary".
    pub unconsulted_observations: usize,
    /// forjar#372: the config-declared work this plan refused to EXECUTE,
    /// one line each — `ambient_inputs` commands, subprocess secret providers,
    /// `output_equivalence` normalisers. This surface publishes
    /// `readOnlyHint: true`, so it runs none of them; the plan is
    /// lock-relative for whatever is listed here. Always present, including
    /// empty, so a consumer can tell "nothing skipped" from "an older binary".
    pub unattended_skipped: Vec<String>,
    /// The prose form of the disclosure. Present iff there is something to
    /// disclose — `unconsulted_observations` is non-zero, or
    /// `unattended_skipped` is non-empty, or both. Names `forjar drift` as the
    /// command that can answer the question this report cannot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disclosure: Option<String>,
}

/// A single planned resource change.
#[derive(Debug, Serialize, JsonSchema)]
pub struct PlannedChangeOutput {
    /// Resource identifier.
    pub resource_id: String,
    /// Target machine name.
    pub machine: String,
    /// Planned action (create, update, destroy).
    pub action: String,
    /// Human-readable change description.
    pub description: String,
}

/// MCP drift handler input.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DriftInput {
    /// Path to forjar.yaml
    pub path: String,
    /// State directory (default: "state")
    pub state_dir: Option<String>,
    /// Filter to specific machine
    pub machine: Option<String>,
}

/// MCP drift handler output.
#[derive(Debug, Serialize, JsonSchema)]
pub struct DriftOutput {
    /// Whether any drift was detected.
    pub drifted: bool,
    /// Individual drift findings.
    pub findings: Vec<DriftFindingOutput>,
    /// GH-208: machines that could NOT be compared (no state recorded yet), so a
    /// caller can distinguish "clean" from "not looked at". Previously an
    /// uncomparable machine simply contributed no findings and the tool answered
    /// `drifted: false`, which reads as a clean bill of health for a machine that
    /// was never inspected.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unchecked: Vec<String>,
}

/// A single drift finding for a resource.
#[derive(Debug, Serialize, JsonSchema)]
pub struct DriftFindingOutput {
    /// Resource that drifted.
    pub resource: String,
    /// Expected content hash.
    pub expected_hash: String,
    /// Actual content hash found.
    pub actual_hash: String,
    /// Drift detail message.
    pub detail: String,
}

/// MCP lint handler input.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct LintInput {
    /// Path to forjar.yaml
    pub path: String,
}

/// MCP lint handler output.
#[derive(Debug, Serialize, JsonSchema)]
pub struct LintOutput {
    /// Lint warning messages.
    pub warnings: Vec<String>,
    /// Total number of warnings.
    pub warning_count: usize,
    /// Total number of errors.
    pub error_count: usize,
}

/// MCP graph handler input.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GraphInput {
    /// Path to forjar.yaml
    pub path: String,
    /// Output format: "mermaid" (default) or "dot". Any other value is an
    /// error — "ascii" and "svg" exist on the CLI only, and an unrecognised
    /// value is rejected exactly as `forjar graph --format` rejects it (GH-212).
    pub format: Option<String>,
}

/// MCP graph handler output.
#[derive(Debug, Serialize, JsonSchema)]
pub struct GraphOutput {
    /// Rendered dependency graph.
    pub graph: String,
    /// The format ACTUALLY rendered — always one of "mermaid" or "dot", never
    /// an echo of an unsupported request (GH-212).
    pub format: String,
}

/// MCP show handler input.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ShowInput {
    /// Path to forjar.yaml
    pub path: String,
    /// Show specific resource only
    pub resource: Option<String>,
}

/// MCP show handler output.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ShowOutput {
    /// Parsed config as JSON value.
    pub config: serde_json::Value,
}

/// MCP status handler input.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct StatusInput {
    /// GH-208: path to the project's forjar.yaml. Optional for backward
    /// compatibility, but without it the tool cannot know WHICH project it
    /// is being asked about and falls back to `./state` relative to the
    /// server's cwd — which for an MCP stdio server is chosen by the client
    /// and is arbitrary.
    pub path: Option<String>,
    /// State directory (default: "state")
    pub state_dir: Option<String>,
    /// Filter to specific machine
    pub machine: Option<String>,
}

/// MCP status handler output.
#[derive(Debug, Serialize, JsonSchema)]
pub struct StatusOutput {
    /// Per-machine status entries.
    pub machines: Vec<MachineStatusOutput>,
}

/// Status summary for a single machine.
#[derive(Debug, Serialize, JsonSchema)]
pub struct MachineStatusOutput {
    /// Machine name.
    pub name: String,
    /// Number of managed resources.
    pub resource_count: usize,
}

/// MCP trace handler input.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TraceInput {
    /// GH-208: path to the project's forjar.yaml. Optional for backward
    /// compatibility, but without it the tool cannot know WHICH project it
    /// is being asked about and falls back to `./state` relative to the
    /// server's cwd — which for an MCP stdio server is chosen by the client
    /// and is arbitrary.
    pub path: Option<String>,
    /// State directory (default: "state")
    pub state_dir: Option<String>,
    /// Filter to specific machine
    pub machine: Option<String>,
}

/// MCP trace handler output.
#[derive(Debug, Serialize, JsonSchema)]
pub struct TraceOutput {
    /// Number of trace entries.
    pub trace_count: usize,
    /// Individual trace spans.
    pub spans: Vec<TraceSpanOutput>,
}

/// A single trace span.
#[derive(Debug, Serialize, JsonSchema)]
pub struct TraceSpanOutput {
    /// Machine the span ran on.
    pub machine: String,
    /// Unique trace identifier.
    pub trace_id: String,
    /// Unique span identifier.
    pub span_id: String,
    /// Parent span for nesting.
    pub parent_span_id: Option<String>,
    /// Span operation name.
    pub name: String,
    /// ISO 8601 start timestamp.
    pub start_time: String,
    /// Duration in microseconds.
    pub duration_us: u64,
    /// Process exit code.
    pub exit_code: i32,
    /// Resource type (package, file, service, etc.).
    pub resource_type: String,
    /// Action performed (create, update, destroy).
    pub action: String,
    /// Content hash after action.
    pub content_hash: Option<String>,
    /// Lamport logical clock value.
    pub logical_clock: u64,
}

/// MCP anomaly handler input.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnomalyInput {
    /// GH-208: path to the project's forjar.yaml. Optional for backward
    /// compatibility, but without it the tool cannot know WHICH project it
    /// is being asked about and falls back to `./state` relative to the
    /// server's cwd — which for an MCP stdio server is chosen by the client
    /// and is arbitrary.
    pub path: Option<String>,
    /// State directory (default: "state")
    pub state_dir: Option<String>,
    /// Filter to specific machine
    pub machine: Option<String>,
    /// Minimum events to consider a resource (default: 3)
    pub min_events: Option<usize>,
}

/// MCP anomaly handler output.
#[derive(Debug, Serialize, JsonSchema)]
pub struct AnomalyOutput {
    /// Number of anomalies detected.
    pub anomaly_count: usize,
    /// Individual anomaly findings.
    pub findings: Vec<AnomalyFindingOutput>,
}

/// A single anomaly finding.
#[derive(Debug, Serialize, JsonSchema)]
pub struct AnomalyFindingOutput {
    /// Resource with anomalous behavior.
    pub resource: String,
    /// Anomaly score (higher = more anomalous).
    pub score: f64,
    /// Anomaly status classification.
    pub status: String,
    /// Reasons for anomaly detection.
    pub reasons: Vec<String>,
}
