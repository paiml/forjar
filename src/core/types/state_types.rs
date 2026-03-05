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

    /// FJ-1260: Persisted output values for cross-stack data flow
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub outputs: IndexMap<String, String>,
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
///
/// # Examples
///
/// ```
/// use forjar::core::types::ResourceStatus;
///
/// assert_eq!(ResourceStatus::Converged.to_string(), "CONVERGED");
/// assert_eq!(ResourceStatus::Failed.to_string(), "FAILED");
/// assert_eq!(ResourceStatus::Drifted.to_string(), "DRIFTED");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceStatus {
    /// Resource matches desired state.
    Converged,
    /// Resource apply failed.
    Failed,
    /// Resource state drifted from lock.
    Drifted,
    /// Resource status not yet determined.
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
    /// Resource will be created.
    Create,
    /// Resource will be updated.
    Update,
    /// Resource will be destroyed.
    Destroy,
    /// Resource is already converged.
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

    /// Number of resources to create.
    pub to_create: u32,
    /// Number of resources to update.
    pub to_update: u32,
    /// Number of resources to destroy.
    pub to_destroy: u32,
    /// Number of unchanged resources.
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
        /// Target machine name.
        machine: String,
        /// Unique run identifier.
        run_id: String,
        /// Forjar CLI version.
        forjar_version: String,
        /// FJ-1391: Operator identity for drift forensics (e.g., "user@hostname")
        #[serde(default, skip_serializing_if = "Option::is_none")]
        operator: Option<String>,
        /// FJ-1391: BLAKE3 hash of the config file used for this apply
        #[serde(default, skip_serializing_if = "Option::is_none")]
        config_hash: Option<String>,
        /// FJ-1393: Param count for experiment tracking (number of params in this apply)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        param_count: Option<u32>,
    },
    ResourceStarted {
        /// Target machine name.
        machine: String,
        /// Resource identifier.
        resource: String,
        /// Action being performed.
        action: String,
    },
    ResourceConverged {
        /// Target machine name.
        machine: String,
        /// Resource identifier.
        resource: String,
        /// Time taken to converge in seconds.
        duration_seconds: f64,
        /// BLAKE3 hash of the converged state.
        hash: String,
    },
    ResourceFailed {
        /// Target machine name.
        machine: String,
        /// Resource identifier.
        resource: String,
        /// Error message describing the failure.
        error: String,
    },
    ApplyCompleted {
        /// Target machine name.
        machine: String,
        /// Unique run identifier.
        run_id: String,
        /// Number of resources that converged.
        resources_converged: u32,
        /// Number of resources unchanged.
        resources_unchanged: u32,
        /// Number of resources that failed.
        resources_failed: u32,
        /// Total apply duration in seconds.
        total_seconds: f64,
    },
    DriftDetected {
        /// Target machine name.
        machine: String,
        /// Resource identifier.
        resource: String,
        /// Expected content hash.
        expected_hash: String,
        /// Actual content hash found.
        actual_hash: String,
    },
    /// FJ-201: Secret decryption audit event.
    SecretAccessed {
        /// Resource that accessed the secret.
        resource: String,
        /// Number of encrypted markers decrypted.
        marker_count: u32,
        /// Age identity recipient used for decryption.
        identity_recipient: String,
    },
    /// FJ-201: Secret rotation audit event.
    SecretRotated {
        /// Config file containing rotated secrets.
        file: String,
        /// Number of markers re-encrypted.
        marker_count: u32,
        /// New age recipient public keys.
        new_recipients: Vec<String>,
    },
}

/// Timestamped event wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampedEvent {
    /// ISO 8601 timestamp.
    pub ts: String,
    /// The provenance event.
    #[serde(flatten)]
    pub event: ProvenanceEvent,
}

// ============================================================================
// Apply result
// ============================================================================

/// FJ-262: Per-resource timing report entry.
#[derive(Debug, Clone, Serialize)]
pub struct ResourceReport {
    /// Resource identifier.
    pub resource_id: String,
    /// Resource type string.
    pub resource_type: String,
    /// Apply status (e.g., "converged", "failed").
    pub status: String,
    /// Apply duration in seconds.
    pub duration_seconds: f64,
    /// Process exit code, if applicable.
    pub exit_code: Option<i32>,
    /// BLAKE3 hash of the converged state.
    pub hash: Option<String>,
    /// Error message on failure.
    pub error: Option<String>,
}

/// Result of applying to a single machine.
#[derive(Debug, Clone, Serialize)]
pub struct ApplyResult {
    /// Target machine name.
    pub machine: String,
    /// Number of resources that converged.
    pub resources_converged: u32,
    /// Number of resources unchanged.
    pub resources_unchanged: u32,
    /// Number of resources that failed.
    pub resources_failed: u32,
    /// Total apply duration.
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
///
/// # Examples
///
/// ```
/// use forjar::core::types::yaml_value_to_string;
/// use serde_yaml_ng::Value;
///
/// assert_eq!(yaml_value_to_string(&Value::String("hello".into())), "hello");
/// assert_eq!(yaml_value_to_string(&Value::Bool(true)), "true");
/// assert_eq!(yaml_value_to_string(&Value::Null), "");
/// ```
pub fn yaml_value_to_string(val: &serde_yaml_ng::Value) -> String {
    match val {
        serde_yaml_ng::Value::String(s) => s.clone(),
        serde_yaml_ng::Value::Number(n) => n.to_string(),
        serde_yaml_ng::Value::Bool(b) => b.to_string(),
        serde_yaml_ng::Value::Null => String::new(),
        other => format!("{other:?}"),
    }
}
