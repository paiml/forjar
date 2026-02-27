//! State, lock, plan, and provenance types.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use super::ResourceType;

// ============================================================================
// State / Lock file
// ============================================================================

/// Global lock file (state/forjar.lock.yaml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalLock {
    /// Schema version
    pub schema: String,

    /// Config name
    pub name: String,

    /// Last apply timestamp
    pub last_apply: String,

    /// Generator version
    pub generator: String,

    /// Per-machine summary
    pub machines: IndexMap<String, MachineSummary>,
}

/// Per-machine summary in the global lock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineSummary {
    /// Number of resources
    pub resources: usize,

    /// Number converged
    pub converged: usize,

    /// Number failed
    pub failed: usize,

    /// Last apply timestamp
    pub last_apply: String,
}

/// Per-machine state lock file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateLock {
    /// Schema version
    pub schema: String,

    /// Machine name
    pub machine: String,

    /// Machine hostname
    pub hostname: String,

    /// When the lock was generated
    pub generated_at: String,

    /// Generator version
    pub generator: String,

    /// BLAKE3 version
    pub blake3_version: String,

    /// Per-resource state
    pub resources: IndexMap<String, ResourceLock>,
}

/// Per-resource lock entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLock {
    /// Resource type
    #[serde(rename = "type")]
    pub resource_type: ResourceType,

    /// Convergence status
    pub status: ResourceStatus,

    /// When the resource was last applied
    #[serde(default)]
    pub applied_at: Option<String>,

    /// Duration of last apply in seconds
    #[serde(default)]
    pub duration_seconds: Option<f64>,

    /// BLAKE3 hash of the resource's observable state
    pub hash: String,

    /// Resource-specific details
    #[serde(default)]
    pub details: HashMap<String, serde_yaml_ng::Value>,
}

/// Resource convergence status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceStatus {
    Converged,
    Failed,
    Drifted,
    Unknown,
}

impl fmt::Display for ResourceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Converged => write!(f, "CONVERGED"),
            Self::Failed => write!(f, "FAILED"),
            Self::Drifted => write!(f, "DRIFTED"),
            Self::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

// ============================================================================
// Plan
// ============================================================================

/// Action to take on a resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanAction {
    Create,
    Update,
    Destroy,
    NoOp,
}

impl fmt::Display for PlanAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Create => write!(f, "CREATE"),
            Self::Update => write!(f, "UPDATE"),
            Self::Destroy => write!(f, "DESTROY"),
            Self::NoOp => write!(f, "NO-OP"),
        }
    }
}

/// A single planned change.
#[derive(Debug, Clone, Serialize)]
pub struct PlannedChange {
    /// Resource ID
    pub resource_id: String,

    /// Target machine
    pub machine: String,

    /// Resource type
    pub resource_type: ResourceType,

    /// Action to take
    pub action: PlanAction,

    /// Human-readable description
    pub description: String,
}

/// Full execution plan.
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionPlan {
    /// Config name
    pub name: String,

    /// Planned changes grouped by machine
    pub changes: Vec<PlannedChange>,

    /// Topological execution order (resource IDs)
    pub execution_order: Vec<String>,

    /// Summary counts
    pub to_create: u32,
    pub to_update: u32,
    pub to_destroy: u32,
    pub unchanged: u32,
}

// ============================================================================
// Provenance events
// ============================================================================

/// Provenance event for the JSONL event log.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ProvenanceEvent {
    ApplyStarted {
        machine: String,
        run_id: String,
        forjar_version: String,
    },
    ResourceStarted {
        machine: String,
        resource: String,
        action: String,
    },
    ResourceConverged {
        machine: String,
        resource: String,
        duration_seconds: f64,
        hash: String,
    },
    ResourceFailed {
        machine: String,
        resource: String,
        error: String,
    },
    ApplyCompleted {
        machine: String,
        run_id: String,
        resources_converged: u32,
        resources_unchanged: u32,
        resources_failed: u32,
        total_seconds: f64,
    },
    DriftDetected {
        machine: String,
        resource: String,
        expected_hash: String,
        actual_hash: String,
    },
    /// FJ-201: Secret decryption audit event.
    SecretAccessed {
        resource: String,
        marker_count: u32,
        identity_recipient: String,
    },
    /// FJ-201: Secret rotation audit event.
    SecretRotated {
        file: String,
        marker_count: u32,
        new_recipients: Vec<String>,
    },
}

/// Timestamped event wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampedEvent {
    pub ts: String,
    #[serde(flatten)]
    pub event: ProvenanceEvent,
}

// ============================================================================
// Apply result
// ============================================================================

/// FJ-262: Per-resource timing report entry.
#[derive(Debug, Clone, Serialize)]
pub struct ResourceReport {
    pub resource_id: String,
    pub resource_type: String,
    pub status: String,
    pub duration_seconds: f64,
    pub exit_code: Option<i32>,
    pub hash: Option<String>,
    pub error: Option<String>,
}

/// Result of applying to a single machine.
#[derive(Debug, Clone, Serialize)]
pub struct ApplyResult {
    pub machine: String,
    pub resources_converged: u32,
    pub resources_unchanged: u32,
    pub resources_failed: u32,
    #[serde(serialize_with = "serialize_duration_secs")]
    pub total_duration: std::time::Duration,
    /// FJ-262: Per-resource reports for timing + status
    pub resource_reports: Vec<ResourceReport>,
}

fn serialize_duration_secs<S: serde::Serializer>(
    d: &std::time::Duration,
    s: S,
) -> Result<S::Ok, S::Error> {
    s.serialize_f64(d.as_secs_f64())
}

// ============================================================================
// Template helper
// ============================================================================

/// Convert a serde_yaml_ng::Value to a string for template resolution.
pub fn yaml_value_to_string(val: &serde_yaml_ng::Value) -> String {
    match val {
        serde_yaml_ng::Value::String(s) => s.clone(),
        serde_yaml_ng::Value::Number(n) => n.to_string(),
        serde_yaml_ng::Value::Bool(b) => b.to_string(),
        serde_yaml_ng::Value::Null => String::new(),
        other => format!("{:?}", other),
    }
}
