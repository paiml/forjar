//! MCP Input/Output type definitions for forjar handlers.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── Input / Output types ────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ValidateInput {
    /// Path to forjar.yaml
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ValidateOutput {
    pub valid: bool,
    pub resource_count: usize,
    pub machine_count: usize,
    pub errors: Vec<String>,
}

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

#[derive(Debug, Serialize, JsonSchema)]
pub struct PlanOutput {
    pub changes: Vec<PlannedChangeOutput>,
    pub to_create: u32,
    pub to_update: u32,
    pub to_destroy: u32,
    pub unchanged: u32,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct PlannedChangeOutput {
    pub resource_id: String,
    pub machine: String,
    pub action: String,
    pub description: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DriftInput {
    /// Path to forjar.yaml
    pub path: String,
    /// State directory (default: "state")
    pub state_dir: Option<String>,
    /// Filter to specific machine
    pub machine: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DriftOutput {
    pub drifted: bool,
    pub findings: Vec<DriftFindingOutput>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DriftFindingOutput {
    pub resource: String,
    pub expected_hash: String,
    pub actual_hash: String,
    pub detail: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LintInput {
    /// Path to forjar.yaml
    pub path: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct LintOutput {
    pub warnings: Vec<String>,
    pub warning_count: usize,
    pub error_count: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GraphInput {
    /// Path to forjar.yaml
    pub path: String,
    /// Output format: "mermaid" (default) or "dot"
    pub format: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GraphOutput {
    pub graph: String,
    pub format: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ShowInput {
    /// Path to forjar.yaml
    pub path: String,
    /// Show specific resource only
    pub resource: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ShowOutput {
    pub config: serde_json::Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StatusInput {
    /// State directory (default: "state")
    pub state_dir: Option<String>,
    /// Filter to specific machine
    pub machine: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct StatusOutput {
    pub machines: Vec<MachineStatusOutput>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct MachineStatusOutput {
    pub name: String,
    pub resource_count: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TraceInput {
    /// State directory (default: "state")
    pub state_dir: Option<String>,
    /// Filter to specific machine
    pub machine: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TraceOutput {
    pub trace_count: usize,
    pub spans: Vec<TraceSpanOutput>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TraceSpanOutput {
    pub machine: String,
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub start_time: String,
    pub duration_us: u64,
    pub exit_code: i32,
    pub resource_type: String,
    pub action: String,
    pub content_hash: Option<String>,
    pub logical_clock: u64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnomalyInput {
    /// State directory (default: "state")
    pub state_dir: Option<String>,
    /// Filter to specific machine
    pub machine: Option<String>,
    /// Minimum events to consider a resource (default: 3)
    pub min_events: Option<usize>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AnomalyOutput {
    pub anomaly_count: usize,
    pub findings: Vec<AnomalyFindingOutput>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AnomalyFindingOutput {
    pub resource: String,
    pub score: f64,
    pub status: String,
    pub reasons: Vec<String>,
}
