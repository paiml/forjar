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
///
/// # There is deliberately no `policy_dir` here
///
/// `lint` is `Effects::ReadOnly`, so `forjar_lint` publishes
/// `readOnlyHint: true`, and an agent reads that hint to decide it may call the
/// tool unattended. A compliance pack rule of `type: script` is handed to
/// `sh -c` (`core::compliance_pack::check_script`), so a `policy_dir` field
/// here let a caller name a directory and have forjar execute whatever was in
/// it. Measured before the field was removed: a pack whose script was
/// `touch <path>` created that file through CLI, `verb call`, HTTP and a real
/// `tools/call` over stdio, and the reply was
/// `{"gate_passed":true,"error_count":0,"warnings":[]}` — nothing in the result
/// said a script had run.
///
/// A schema description would not have fixed it. `readOnlyHint` is machine-read
/// and the description is not, so the hint would still have been false.
///
/// `--policy-dir` survives on `forjar lint` and `forjar apply --policy-check`,
/// where an operator typed a flag whose help text says it runs shell. Opting in
/// and reading a schema are not the same act.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LintInput {
    /// Path to forjar.yaml
    pub path: String,
    /// Cyclomatic ceiling for generated shell. Omitted: the check is skipped
    /// entirely — no parse, no CFG. See `core::quality_gate` for why it is
    /// opt-in rather than defaulted.
    ///
    /// This one stays: it parses forjar's OWN generated shell and walks a CFG.
    /// It runs nothing and writes nothing, so it cannot falsify `readOnlyHint`
    /// the way `policy_dir` did.
    pub max_cyclomatic: Option<usize>,
}

/// One quality-gate finding, in the shape a SARIF consumer would recognise.
#[derive(Debug, Serialize, JsonSchema)]
pub struct GateFindingOutput {
    /// `FJQ-CPX-001` | `FJQ-SEC-001` | `FJQ-SEC-002` | `FJQ-SH-<code>` | a policy id.
    pub rule_id: String,
    /// "error", "warning" or "note". Only "error" blocks an apply.
    pub level: String,
    /// Resource this is about. Empty for a config-wide compliance rule.
    pub resource: String,
    /// What is wrong.
    pub message: String,
    /// 1-based line of the resource key, when it is in the addressed file.
    /// Absent for a resource that arrived through `includes:`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yaml_line: Option<usize>,
    /// "check" | "apply" | "state_query" when the finding is about generated shell.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_kind: Option<String>,
    /// 1-based line within that generated script.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_line: Option<usize>,
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
    /// Whether the quality gate passed — false when any finding is an error.
    ///
    /// A failing gate is a SUCCESSFUL result with this set to false, never an
    /// MCP error. `pforge_runtime::Error::Handler` carries a bare string, so a
    /// structured verdict cannot be expressed as an error, and an `Err`
    /// bypasses `output_schema` entirely — it would ship a payload no
    /// published schema describes.
    pub gate_passed: bool,
    /// `FORJAR_QUALITY_GATE_VIOLATION` when the gate did not pass.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    /// Every finding, including advisory ones the `warnings` lines only tally.
    pub findings: Vec<GateFindingOutput>,
    /// SARIF 2.1.0 log for CI ingestion (GitHub Code Scanning et al).
    pub sarif: serde_json::Value,
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

/// MCP remediate handler input.
///
/// There is deliberately no `dry_run`. The verb returns the corrected document
/// and writes nothing, so a caller that wants the file changed already has the
/// bytes; a flag whose `false` branch cannot be honoured is worse than a
/// missing one, and a `readOnlyHint` that depends on a parameter is a
/// `readOnlyHint` an agent cannot trust.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemediateInput {
    /// Path to forjar.yaml
    pub path: String,
    /// Restrict to these policy ids: the rule's `id`, or — when it declares
    /// none — its generated `RULE-<index>-<slug>`, where `index` is the rule's
    /// position in `policies:`. Omitted or empty means every rule.
    ///
    /// The index is part of the identity because a slug of the `message:` is
    /// not one: two un-id'd rules sharing a message generated the same name, so
    /// naming it applied BOTH and no string selected between them
    /// (paiml/forjar#369). The ids this tool REPORTS in `remediations_applied`
    /// and `remaining_violations` are the ids it accepts here, so round-tripping
    /// one back is exact.
    pub policy_ids: Option<Vec<String>>,
}

/// MCP remediate handler output.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RemediateOutput {
    /// Corrections applied, sorted by resource then field.
    pub remediations_applied: Vec<RemediationOutput>,
    /// The corrected document. Byte-identical to the input when nothing
    /// applied. Nothing on disk was changed — this is the write, and the
    /// caller performs it.
    pub updated_yaml_content: String,
    /// Violations still present, re-evaluated against the corrected config,
    /// each carrying why forjar did not fix it.
    pub remaining_violations: Vec<ViolationOutput>,
    /// Whether the document changed.
    pub changed: bool,
    /// Content hash of the config before.
    pub config_hash_before: String,
    /// Content hash of the config after.
    pub config_hash_after: String,
    /// What this verb did not look at, when that would otherwise read as "the
    /// config is clean".
    pub scope_note: Option<String>,
}

/// One applied correction.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RemediationOutput {
    /// The rule that determined the value.
    pub policy_id: String,
    /// The resource whose field was rewritten.
    pub resource_id: String,
    /// The field that was rewritten.
    pub field: String,
    /// The value before.
    pub from: Option<String>,
    /// The value written — read from the policy rule, never chosen by forjar.
    pub to: String,
    /// 1-based line of the edited value.
    pub line: usize,
}

/// One violation that is still present.
///
/// A typed projection of `PolicyViolation` rather than a `serde_json::Value`:
/// the untyped alternative publishes a schema that describes nothing, which is
/// the weakness `ShowOutput.config` already carries. It does duplicate the
/// shape `policy_check_to_json` renders, and that is a real drift surface.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ViolationOutput {
    /// The rule that flagged it.
    pub policy_id: String,
    /// The resource it was flagged on.
    pub resource_id: String,
    /// The rule's own message.
    pub message: String,
    /// `error`, `warning` or `info`.
    pub severity: String,
    /// `assert`, `deny`, `warn`, `require` or `limit`.
    pub rule_type: String,
    /// The rule's prose `remediation:` hint, if it carries one.
    pub remediation_hint: Option<String>,
    /// Why forjar did not fix it.
    pub reason: String,
}
