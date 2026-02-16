//! FJ-001: All types from the forjar specification.
//!
//! Defines the YAML schema types for machines, resources, policy, state locks,
//! and provenance events. All types derive Serialize/Deserialize for YAML roundtripping.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// ============================================================================
// Top-level forjar.yaml
// ============================================================================

/// Root configuration — the desired state of infrastructure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForjarConfig {
    /// Schema version (must be "1.0")
    pub version: String,

    /// Human-readable infrastructure name
    pub name: String,

    /// Optional description
    #[serde(default)]
    pub description: Option<String>,

    /// Global parameters (templatable)
    #[serde(default)]
    pub params: HashMap<String, serde_yaml_ng::Value>,

    /// Machine inventory
    #[serde(default)]
    pub machines: IndexMap<String, Machine>,

    /// Resource declarations (order-preserving)
    pub resources: IndexMap<String, Resource>,

    /// Execution policy
    #[serde(default)]
    pub policy: Policy,
}

// ============================================================================
// Machines
// ============================================================================

/// A managed machine (bare-metal, VM, or edge device).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Machine {
    /// Machine hostname
    pub hostname: String,

    /// Network address (IP or DNS)
    pub addr: String,

    /// SSH user
    #[serde(default = "default_user")]
    pub user: String,

    /// CPU architecture
    #[serde(default = "default_arch")]
    pub arch: String,

    /// Path to SSH private key
    #[serde(default)]
    pub ssh_key: Option<String>,

    /// Roles for this machine (informational)
    #[serde(default)]
    pub roles: Vec<String>,
}

fn default_user() -> String {
    "root".to_string()
}

fn default_arch() -> String {
    "x86_64".to_string()
}

// ============================================================================
// Resources
// ============================================================================

/// A single infrastructure resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// Resource type
    #[serde(rename = "type")]
    pub resource_type: ResourceType,

    /// Target machine(s) — single name or list
    #[serde(default)]
    pub machine: MachineTarget,

    /// Resource state (present/absent/running/etc.)
    #[serde(default)]
    pub state: Option<String>,

    /// Dependencies (other resource IDs that must be applied first)
    #[serde(default)]
    pub depends_on: Vec<String>,

    // -- Package fields --
    /// Package provider (apt, cargo, pip)
    #[serde(default)]
    pub provider: Option<String>,

    /// Package list
    #[serde(default)]
    pub packages: Vec<String>,

    // -- File fields --
    /// File/mount path
    #[serde(default)]
    pub path: Option<String>,

    /// Inline file content
    #[serde(default)]
    pub content: Option<String>,

    /// Source file (local path for copia sync)
    #[serde(default)]
    pub source: Option<String>,

    /// Symlink target
    #[serde(default)]
    pub target: Option<String>,

    /// File owner
    #[serde(default)]
    pub owner: Option<String>,

    /// File group
    #[serde(default)]
    pub group: Option<String>,

    /// File mode (e.g., "0644")
    #[serde(default)]
    pub mode: Option<String>,

    // -- Service fields --
    /// Service/unit name
    #[serde(default)]
    pub name: Option<String>,

    /// Enable on boot
    #[serde(default)]
    pub enabled: Option<bool>,

    /// Restart when these resources change
    #[serde(default)]
    pub restart_on: Vec<String>,

    // -- Mount fields --
    /// Mount source (device or NFS path)
    #[serde(rename = "fstype", default)]
    pub fs_type: Option<String>,

    /// Mount options string
    #[serde(default)]
    pub options: Option<String>,
}

/// Resource type enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Package,
    File,
    Service,
    Mount,
    User,
    Docker,
    Pepita,
    Network,
    Cron,
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Package => write!(f, "package"),
            Self::File => write!(f, "file"),
            Self::Service => write!(f, "service"),
            Self::Mount => write!(f, "mount"),
            Self::User => write!(f, "user"),
            Self::Docker => write!(f, "docker"),
            Self::Pepita => write!(f, "pepita"),
            Self::Network => write!(f, "network"),
            Self::Cron => write!(f, "cron"),
        }
    }
}

/// Machine target — single machine or multiple.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MachineTarget {
    Single(String),
    Multiple(Vec<String>),
}

impl Default for MachineTarget {
    fn default() -> Self {
        Self::Single("localhost".to_string())
    }
}

impl MachineTarget {
    /// Expand to a list of machine names.
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            Self::Single(s) => vec![s.clone()],
            Self::Multiple(v) => v.clone(),
        }
    }
}

// ============================================================================
// Policy
// ============================================================================

/// Execution policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Failure handling
    #[serde(default)]
    pub failure: FailurePolicy,

    /// Apply to independent machines concurrently
    #[serde(default)]
    pub parallel_machines: bool,

    /// Enable provenance tracing on every apply
    #[serde(default = "default_true")]
    pub tripwire: bool,

    /// Persist BLAKE3 state after apply
    #[serde(default = "default_true")]
    pub lock_file: bool,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            failure: FailurePolicy::default(),
            parallel_machines: false,
            tripwire: true,
            lock_file: true,
        }
    }
}

fn default_true() -> bool {
    true
}

/// Failure handling strategy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailurePolicy {
    #[default]
    StopOnFirst,
    ContinueIndependent,
}

// ============================================================================
// State / Lock file
// ============================================================================

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
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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

/// Result of applying to a single machine.
#[derive(Debug, Clone)]
pub struct ApplyResult {
    pub machine: String,
    pub resources_converged: u32,
    pub resources_unchanged: u32,
    pub resources_failed: u32,
    pub total_duration: std::time::Duration,
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj001_config_parse() {
        let yaml = r#"
version: "1.0"
name: test-infra
params:
  raid_path: /mnt/raid
machines:
  lambda:
    hostname: lambda-box
    addr: 192.168.1.1
    user: noah
    arch: x86_64
    roles: [gpu-compute]
resources:
  test-pkg:
    type: package
    machine: lambda
    provider: apt
    packages: [curl, wget]
policy:
  failure: stop_on_first
  tripwire: true
  lock_file: true
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.version, "1.0");
        assert_eq!(config.name, "test-infra");
        assert_eq!(config.machines.len(), 1);
        assert_eq!(config.machines["lambda"].hostname, "lambda-box");
        assert_eq!(config.resources.len(), 1);
        assert_eq!(
            config.resources["test-pkg"].resource_type,
            ResourceType::Package
        );
    }

    #[test]
    fn test_fj001_machine_defaults() {
        let yaml = r#"
hostname: test
addr: 1.2.3.4
"#;
        let m: Machine = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(m.user, "root");
        assert_eq!(m.arch, "x86_64");
        assert!(m.roles.is_empty());
    }

    #[test]
    fn test_fj001_machine_target_single() {
        let t = MachineTarget::Single("lambda".to_string());
        assert_eq!(t.to_vec(), vec!["lambda"]);
    }

    #[test]
    fn test_fj001_machine_target_multiple() {
        let yaml = r#"[intel, jetson]"#;
        let t: MachineTarget = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(t.to_vec(), vec!["intel", "jetson"]);
    }

    #[test]
    fn test_fj001_resource_type_display() {
        assert_eq!(ResourceType::Package.to_string(), "package");
        assert_eq!(ResourceType::Service.to_string(), "service");
        assert_eq!(ResourceType::Mount.to_string(), "mount");
    }

    #[test]
    fn test_fj001_policy_defaults() {
        let p = Policy::default();
        assert_eq!(p.failure, FailurePolicy::StopOnFirst);
        assert!(p.tripwire);
        assert!(p.lock_file);
        assert!(!p.parallel_machines);
    }

    #[test]
    fn test_fj001_resource_status_display() {
        assert_eq!(ResourceStatus::Converged.to_string(), "CONVERGED");
        assert_eq!(ResourceStatus::Drifted.to_string(), "DRIFTED");
    }

    #[test]
    fn test_fj001_plan_action_display() {
        assert_eq!(PlanAction::Create.to_string(), "CREATE");
        assert_eq!(PlanAction::NoOp.to_string(), "NO-OP");
    }

    #[test]
    fn test_fj001_state_lock_roundtrip() {
        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "lambda".to_string(),
            hostname: "test-box".to_string(),
            generated_at: "2026-02-16T14:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources: IndexMap::from([(
                "test-pkg".to_string(),
                ResourceLock {
                    resource_type: ResourceType::Package,
                    status: ResourceStatus::Converged,
                    applied_at: Some("2026-02-16T14:00:01Z".to_string()),
                    duration_seconds: Some(1.5),
                    hash: "blake3:abc123".to_string(),
                    details: HashMap::new(),
                },
            )]),
        };
        let yaml = serde_yaml_ng::to_string(&lock).unwrap();
        let lock2: StateLock = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(lock2.machine, "lambda");
        assert_eq!(
            lock2.resources["test-pkg"].status,
            ResourceStatus::Converged
        );
    }

    #[test]
    fn test_fj001_provenance_event_serde() {
        let event = ProvenanceEvent::ApplyStarted {
            machine: "lambda".to_string(),
            run_id: "r-abc".to_string(),
            forjar_version: "0.1.0".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"event\":\"apply_started\""));
        assert!(json.contains("\"run_id\":\"r-abc\""));
    }

    #[test]
    fn test_fj001_yaml_value_to_string() {
        assert_eq!(
            yaml_value_to_string(&serde_yaml_ng::Value::String("hello".into())),
            "hello"
        );
        assert_eq!(yaml_value_to_string(&serde_yaml_ng::Value::Bool(true)), "true");
        assert_eq!(yaml_value_to_string(&serde_yaml_ng::Value::Null), "");
    }

    #[test]
    fn test_fj001_multi_machine_resource() {
        let yaml = r#"
version: "1.0"
name: multi
machines:
  a:
    hostname: a
    addr: 1.1.1.1
  b:
    hostname: b
    addr: 2.2.2.2
resources:
  tools:
    type: package
    machine: [a, b]
    provider: cargo
    packages: [batuta]
policy:
  failure: stop_on_first
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let targets = config.resources["tools"].machine.to_vec();
        assert_eq!(targets, vec!["a", "b"]);
    }
}
