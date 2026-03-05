//! MCP Input/Output type definitions for forjar handlers.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    /// Filter to specific resource
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
    /// Output format: "mermaid" (default) or "dot"
    pub format: Option<String>,
}

/// MCP graph handler output.
#[derive(Debug, Serialize, JsonSchema)]
pub struct GraphOutput {
    /// Rendered dependency graph.
    pub graph: String,
    /// Output format (mermaid or dot).
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
