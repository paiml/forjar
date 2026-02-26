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

    /// FJ-215: Output values — computed from params/templates, written to state/outputs.yaml
    #[serde(default)]
    pub outputs: IndexMap<String, OutputValue>,
}

/// A declared output value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputValue {
    /// Template expression (e.g., `{{params.data_dir}}`, `{{machines.web.addr}}`)
    pub value: String,

    /// Optional description for documentation
    #[serde(default)]
    pub description: Option<String>,
}

// ============================================================================
// Machines
// ============================================================================

/// A managed machine (bare-metal, VM, container, or edge device).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Machine {
    /// Machine hostname
    pub hostname: String,

    /// Network address (IP, DNS, or `container` sentinel)
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

    /// Explicit transport override: `container`. If omitted, inferred from `addr`.
    #[serde(default)]
    pub transport: Option<String>,

    /// Container configuration (required when `transport: container`)
    #[serde(default)]
    pub container: Option<ContainerConfig>,

    /// Relative cost weight (lower = cheaper, preferred first). Default: 0.
    #[serde(default)]
    pub cost: u32,
}

/// Container execution target configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// Container runtime: `docker` or `podman` (default: `docker`)
    #[serde(default = "default_runtime")]
    pub runtime: String,

    /// OCI image (required for ephemeral containers)
    #[serde(default)]
    pub image: Option<String>,

    /// Container name (auto-generated from machine key if omitted)
    #[serde(default)]
    pub name: Option<String>,

    /// Destroy container after apply (default: true)
    #[serde(default = "default_true")]
    pub ephemeral: bool,

    /// Run with `--privileged` flag (default: false)
    #[serde(default)]
    pub privileged: bool,

    /// Run with `--init` for PID 1 reaping (default: true)
    #[serde(default = "default_true")]
    pub init: bool,
}

fn default_runtime() -> String {
    "docker".to_string()
}

impl Machine {
    /// Returns true if this machine uses container transport.
    pub fn is_container_transport(&self) -> bool {
        self.transport.as_deref() == Some("container") || self.addr == "container"
    }

    /// Returns the effective container name (explicit or derived from hostname).
    pub fn container_name(&self) -> String {
        self.container
            .as_ref()
            .and_then(|c| c.name.clone())
            .unwrap_or_else(|| format!("forjar-{}", self.hostname))
    }
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
    /// Package provider (apt, cargo, uv)
    #[serde(default)]
    pub provider: Option<String>,

    /// Package list
    #[serde(default)]
    pub packages: Vec<String>,

    /// Package version constraint (e.g., "1.2.3", ">=1.0")
    #[serde(default)]
    pub version: Option<String>,

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

    // -- User fields --
    /// User ID (UID)
    #[serde(default)]
    pub uid: Option<u32>,

    /// Login shell
    #[serde(default)]
    pub shell: Option<String>,

    /// Home directory
    #[serde(default)]
    pub home: Option<String>,

    /// Supplementary groups
    #[serde(default)]
    pub groups: Vec<String>,

    /// SSH authorized keys
    #[serde(default)]
    pub ssh_authorized_keys: Vec<String>,

    /// System user flag (--system)
    #[serde(default)]
    pub system_user: bool,

    // -- Cron fields --
    /// Cron schedule expression (e.g., "0 * * * *")
    #[serde(default)]
    pub schedule: Option<String>,

    /// Command to execute
    #[serde(default)]
    pub command: Option<String>,

    // -- Docker fields --
    /// Container image
    #[serde(default)]
    pub image: Option<String>,

    /// Port mappings (e.g., ["8080:80", "443:443"])
    #[serde(default)]
    pub ports: Vec<String>,

    /// Environment variables (e.g., ["KEY=value"])
    #[serde(default)]
    pub environment: Vec<String>,

    /// Volume mounts (e.g., ["/host:/container"])
    #[serde(default)]
    pub volumes: Vec<String>,

    /// Docker restart policy
    #[serde(default)]
    pub restart: Option<String>,

    // -- Network fields --
    /// Network protocol (tcp/udp)
    #[serde(default)]
    pub protocol: Option<String>,

    /// Port number or range
    #[serde(default)]
    pub port: Option<String>,

    /// Network action (allow/deny)
    #[serde(default)]
    pub action: Option<String>,

    /// Source IP/CIDR (for firewall rules)
    #[serde(rename = "from", default)]
    pub from_addr: Option<String>,

    // -- Recipe fields --
    /// Recipe name (for type: recipe)
    #[serde(default)]
    pub recipe: Option<String>,

    /// Recipe inputs (for type: recipe)
    #[serde(default)]
    pub inputs: HashMap<String, serde_yaml_ng::Value>,

    /// Architecture filter — only apply to machines with matching arch
    #[serde(default)]
    pub arch: Vec<String>,

    /// Tags for selective filtering (e.g., `tags: [web, critical]`)
    #[serde(default)]
    pub tags: Vec<String>,

    /// Conditional expression — resource only applies when this evaluates to true.
    /// Examples: `{{machine.arch}} == "x86_64"`, `{{params.env}} != "production"`
    #[serde(default)]
    pub when: Option<String>,

    /// FJ-204: Numeric multiplier — creates N copies with `{{index}}` template.
    /// `count: 3` expands `my-res` into `my-res-0`, `my-res-1`, `my-res-2`.
    #[serde(default)]
    pub count: Option<u32>,

    /// FJ-203: List iteration — creates one copy per item with `{{item}}` template.
    /// `for_each: [a, b, c]` expands `my-res` into `my-res-a`, `my-res-b`, `my-res-c`.
    #[serde(default)]
    pub for_each: Option<Vec<String>>,

    // -- Pepita fields (FJ-040: kernel namespace isolation) --
    /// Chroot directory for filesystem isolation
    #[serde(default)]
    pub chroot_dir: Option<String>,

    /// User ID for namespace (uid mapping)
    #[serde(default)]
    pub namespace_uid: Option<u32>,

    /// Group ID for namespace (gid mapping)
    #[serde(default)]
    pub namespace_gid: Option<u32>,

    /// Enable seccomp syscall filtering
    #[serde(default)]
    pub seccomp: bool,

    /// Enable network namespace isolation
    #[serde(default)]
    pub netns: bool,

    /// CPU set binding (e.g., "0-3" or "0,2,4")
    #[serde(default)]
    pub cpuset: Option<String>,

    /// Memory limit in bytes
    #[serde(default)]
    pub memory_limit: Option<u64>,

    /// Overlay filesystem lower directory
    #[serde(default)]
    pub overlay_lower: Option<String>,

    /// Overlay filesystem upper directory
    #[serde(default)]
    pub overlay_upper: Option<String>,

    /// Overlay filesystem work directory
    #[serde(default)]
    pub overlay_work: Option<String>,

    /// Overlay filesystem merged mount point
    #[serde(default)]
    pub overlay_merged: Option<String>,
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
    Recipe,
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
            Self::Recipe => write!(f, "recipe"),
        }
    }
}

/// Machine target — single machine or multiple.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

impl fmt::Display for MachineTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Single(s) => write!(f, "{}", s),
            Self::Multiple(v) => write!(f, "[{}]", v.join(", ")),
        }
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

    /// Command to run locally before apply (exit non-zero aborts)
    #[serde(default)]
    pub pre_apply: Option<String>,

    /// Command to run locally after successful apply
    #[serde(default)]
    pub post_apply: Option<String>,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            failure: FailurePolicy::default(),
            parallel_machines: false,
            tripwire: true,
            lock_file: true,
            pre_apply: None,
            post_apply: None,
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

impl fmt::Display for FailurePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StopOnFirst => write!(f, "stop_on_first"),
            Self::ContinueIndependent => write!(f, "continue_independent"),
        }
    }
}

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
#[derive(Debug, Clone, Serialize)]
pub struct ApplyResult {
    pub machine: String,
    pub resources_converged: u32,
    pub resources_unchanged: u32,
    pub resources_failed: u32,
    #[serde(serialize_with = "serialize_duration_secs")]
    pub total_duration: std::time::Duration,
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
        assert!(m.transport.is_none());
        assert!(m.container.is_none());
    }

    #[test]
    fn test_fj001_container_config_defaults() {
        let yaml = r#"
runtime: docker
image: ubuntu:22.04
"#;
        let c: ContainerConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(c.runtime, "docker");
        assert_eq!(c.image.as_deref(), Some("ubuntu:22.04"));
        assert!(c.name.is_none());
        assert!(c.ephemeral);
        assert!(!c.privileged);
        assert!(c.init);
    }

    #[test]
    fn test_fj001_container_machine_parse() {
        let yaml = r#"
hostname: test-box
addr: container
transport: container
container:
  runtime: docker
  image: ubuntu:22.04
  name: forjar-test
  ephemeral: true
  privileged: false
  init: true
"#;
        let m: Machine = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(m.transport.as_deref(), Some("container"));
        assert!(m.is_container_transport());
        let c = m.container.unwrap();
        assert_eq!(c.runtime, "docker");
        assert_eq!(c.image.as_deref(), Some("ubuntu:22.04"));
        assert_eq!(c.name.as_deref(), Some("forjar-test"));
    }

    #[test]
    fn test_fj001_machine_container_name_derived() {
        let m = Machine {
            hostname: "test-box".to_string(),
            addr: "container".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("container".to_string()),
            container: Some(ContainerConfig {
                runtime: "docker".to_string(),
                image: Some("ubuntu:22.04".to_string()),
                name: None,
                ephemeral: true,
                privileged: false,
                init: true,
            }),
            cost: 0,
        };
        assert_eq!(m.container_name(), "forjar-test-box");
    }

    #[test]
    fn test_fj001_is_container_transport() {
        // Explicit transport field
        let m1 = Machine {
            hostname: "t".to_string(),
            addr: "1.2.3.4".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("container".to_string()),
            container: None,
            cost: 0,
        };
        assert!(m1.is_container_transport());

        // Sentinel addr
        let m2 = Machine {
            hostname: "t".to_string(),
            addr: "container".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };
        assert!(m2.is_container_transport());

        // Normal machine
        let m3 = Machine {
            hostname: "t".to_string(),
            addr: "10.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };
        assert!(!m3.is_container_transport());
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
        assert_eq!(ResourceStatus::Failed.to_string(), "FAILED");
        assert_eq!(ResourceStatus::Drifted.to_string(), "DRIFTED");
        assert_eq!(ResourceStatus::Unknown.to_string(), "UNKNOWN");
    }

    #[test]
    fn test_fj001_plan_action_display() {
        assert_eq!(PlanAction::Create.to_string(), "CREATE");
        assert_eq!(PlanAction::Update.to_string(), "UPDATE");
        assert_eq!(PlanAction::Destroy.to_string(), "DESTROY");
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
        assert_eq!(
            yaml_value_to_string(&serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(
                42
            ))),
            "42"
        );
        assert_eq!(
            yaml_value_to_string(&serde_yaml_ng::Value::Bool(true)),
            "true"
        );
        assert_eq!(yaml_value_to_string(&serde_yaml_ng::Value::Null), "");
        // Sequence/Mapping falls through to Debug format
        let seq = serde_yaml_ng::Value::Sequence(vec![serde_yaml_ng::Value::Null]);
        assert!(!yaml_value_to_string(&seq).is_empty());
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

    #[test]
    fn test_fj001_resource_type_display_all() {
        assert_eq!(ResourceType::File.to_string(), "file");
        assert_eq!(ResourceType::User.to_string(), "user");
        assert_eq!(ResourceType::Docker.to_string(), "docker");
        assert_eq!(ResourceType::Pepita.to_string(), "pepita");
        assert_eq!(ResourceType::Network.to_string(), "network");
        assert_eq!(ResourceType::Cron.to_string(), "cron");
    }

    // ── FJ-131: types.rs edge case tests ─────────────────────

    #[test]
    fn test_fj131_machine_target_default() {
        let t = MachineTarget::default();
        assert_eq!(t.to_vec(), vec!["localhost"]);
    }

    #[test]
    fn test_fj131_machine_target_multiple_empty() {
        let t = MachineTarget::Multiple(vec![]);
        assert!(t.to_vec().is_empty());
    }

    #[test]
    fn test_fj131_resource_type_recipe_display() {
        assert_eq!(ResourceType::Recipe.to_string(), "recipe");
    }

    #[test]
    fn test_fj131_resource_type_serde_roundtrip() {
        let types = [
            ResourceType::Package,
            ResourceType::File,
            ResourceType::Service,
            ResourceType::Mount,
            ResourceType::User,
            ResourceType::Docker,
            ResourceType::Pepita,
            ResourceType::Network,
            ResourceType::Cron,
            ResourceType::Recipe,
        ];
        for rt in &types {
            let json = serde_json::to_string(rt).unwrap();
            let back: ResourceType = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, rt, "roundtrip failed for {:?}", rt);
        }
    }

    #[test]
    fn test_fj131_failure_policy_serde() {
        let yaml_stop = "\"stop_on_first\"";
        let fp: FailurePolicy = serde_yaml_ng::from_str(yaml_stop).unwrap();
        assert_eq!(fp, FailurePolicy::StopOnFirst);

        let yaml_cont = "\"continue_independent\"";
        let fp2: FailurePolicy = serde_yaml_ng::from_str(yaml_cont).unwrap();
        assert_eq!(fp2, FailurePolicy::ContinueIndependent);
    }

    #[test]
    fn test_fj131_failure_policy_default() {
        let fp = FailurePolicy::default();
        assert_eq!(fp, FailurePolicy::StopOnFirst);
    }

    #[test]
    fn test_fj131_policy_with_hooks() {
        let yaml = r#"
failure: continue_independent
tripwire: false
lock_file: false
parallel_machines: true
pre_apply: "echo pre"
post_apply: "echo post"
"#;
        let p: Policy = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(p.failure, FailurePolicy::ContinueIndependent);
        assert!(!p.tripwire);
        assert!(!p.lock_file);
        assert!(p.parallel_machines);
        assert_eq!(p.pre_apply.as_deref(), Some("echo pre"));
        assert_eq!(p.post_apply.as_deref(), Some("echo post"));
    }

    #[test]
    fn test_fj131_container_config_default_runtime() {
        // When runtime is omitted, should default to "docker"
        let yaml = r#"
image: alpine:3.19
"#;
        let c: ContainerConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(c.runtime, "docker");
        assert!(c.ephemeral);
        assert!(c.init);
    }

    #[test]
    fn test_fj131_container_config_podman_non_ephemeral() {
        let yaml = r#"
runtime: podman
name: my-container
ephemeral: false
privileged: true
init: false
"#;
        let c: ContainerConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(c.runtime, "podman");
        assert!(c.image.is_none());
        assert_eq!(c.name.as_deref(), Some("my-container"));
        assert!(!c.ephemeral);
        assert!(c.privileged);
        assert!(!c.init);
    }

    #[test]
    fn test_fj131_machine_cost_default_zero() {
        let yaml = r#"
hostname: m
addr: 1.2.3.4
"#;
        let m: Machine = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(m.cost, 0);
    }

    #[test]
    fn test_fj131_machine_cost_explicit() {
        let yaml = r#"
hostname: gpu
addr: 10.0.0.1
cost: 100
"#;
        let m: Machine = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(m.cost, 100);
    }

    #[test]
    fn test_fj131_machine_ssh_key() {
        let yaml = r#"
hostname: remote
addr: 10.0.0.5
ssh_key: ~/.ssh/deploy_ed25519
"#;
        let m: Machine = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(m.ssh_key.as_deref(), Some("~/.ssh/deploy_ed25519"));
    }

    #[test]
    fn test_fj131_machine_container_name_no_container_block() {
        // container_name() on machine without container block falls back to hostname
        let m = Machine {
            hostname: "bare-metal".to_string(),
            addr: "10.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };
        assert_eq!(m.container_name(), "forjar-bare-metal");
    }

    #[test]
    fn test_fj131_global_lock_roundtrip() {
        let lock = GlobalLock {
            schema: "1.0".to_string(),
            name: "prod".to_string(),
            last_apply: "2026-02-25T12:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            machines: IndexMap::from([(
                "web".to_string(),
                MachineSummary {
                    resources: 5,
                    converged: 4,
                    failed: 1,
                    last_apply: "2026-02-25T12:00:00Z".to_string(),
                },
            )]),
        };
        let yaml = serde_yaml_ng::to_string(&lock).unwrap();
        let lock2: GlobalLock = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(lock2.name, "prod");
        assert_eq!(lock2.machines["web"].resources, 5);
        assert_eq!(lock2.machines["web"].converged, 4);
        assert_eq!(lock2.machines["web"].failed, 1);
    }

    #[test]
    fn test_fj131_resource_lock_optional_fields() {
        let yaml = r#"
type: file
status: converged
hash: "blake3:abc"
"#;
        let rl: ResourceLock = serde_yaml_ng::from_str(yaml).unwrap();
        assert!(rl.applied_at.is_none());
        assert!(rl.duration_seconds.is_none());
        assert!(rl.details.is_empty());
    }

    #[test]
    fn test_fj131_resource_status_serde_roundtrip() {
        let statuses = [
            ResourceStatus::Converged,
            ResourceStatus::Failed,
            ResourceStatus::Drifted,
            ResourceStatus::Unknown,
        ];
        for s in &statuses {
            let yaml = serde_yaml_ng::to_string(s).unwrap();
            let back: ResourceStatus = serde_yaml_ng::from_str(&yaml).unwrap();
            assert_eq!(&back, s, "roundtrip failed for {:?}", s);
        }
    }

    #[test]
    fn test_fj131_provenance_event_all_variants_serde() {
        let events = vec![
            ProvenanceEvent::ApplyStarted {
                machine: "m".to_string(),
                run_id: "r".to_string(),
                forjar_version: "0.1".to_string(),
            },
            ProvenanceEvent::ResourceStarted {
                machine: "m".to_string(),
                resource: "pkg".to_string(),
                action: "create".to_string(),
            },
            ProvenanceEvent::ResourceConverged {
                machine: "m".to_string(),
                resource: "pkg".to_string(),
                duration_seconds: 1.5,
                hash: "blake3:h".to_string(),
            },
            ProvenanceEvent::ResourceFailed {
                machine: "m".to_string(),
                resource: "pkg".to_string(),
                error: "oops".to_string(),
            },
            ProvenanceEvent::ApplyCompleted {
                machine: "m".to_string(),
                run_id: "r".to_string(),
                resources_converged: 3,
                resources_unchanged: 1,
                resources_failed: 0,
                total_seconds: 5.0,
            },
            ProvenanceEvent::DriftDetected {
                machine: "m".to_string(),
                resource: "cfg".to_string(),
                expected_hash: "a".to_string(),
                actual_hash: "b".to_string(),
            },
        ];
        for event in &events {
            let json = serde_json::to_string(event).unwrap();
            let back: ProvenanceEvent = serde_json::from_str(&json).unwrap();
            // Verify roundtrip doesn't panic and produces valid JSON
            let json2 = serde_json::to_string(&back).unwrap();
            assert_eq!(json, json2);
        }
    }

    #[test]
    fn test_fj131_timestamped_event_flatten() {
        let te = TimestampedEvent {
            ts: "2026-02-25T12:00:00Z".to_string(),
            event: ProvenanceEvent::DriftDetected {
                machine: "web".to_string(),
                resource: "cfg".to_string(),
                expected_hash: "aaa".to_string(),
                actual_hash: "bbb".to_string(),
            },
        };
        let json = serde_json::to_string(&te).unwrap();
        // Flattened: ts appears at top level alongside event fields
        assert!(json.contains("\"ts\":\"2026-02-25T12:00:00Z\""));
        assert!(json.contains("\"event\":\"drift_detected\""));
        assert!(json.contains("\"expected_hash\":\"aaa\""));
        // Verify roundtrip
        let back: TimestampedEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.ts, "2026-02-25T12:00:00Z");
    }

    #[test]
    fn test_fj131_planned_change_serialize() {
        let pc = PlannedChange {
            resource_id: "web-config".to_string(),
            machine: "web".to_string(),
            resource_type: ResourceType::File,
            action: PlanAction::Create,
            description: "Create file /etc/app.conf".to_string(),
        };
        let json = serde_json::to_string(&pc).unwrap();
        assert!(json.contains("\"resource_id\":\"web-config\""));
        assert!(json.contains("\"action\":\"create\""));
    }

    #[test]
    fn test_fj131_execution_plan_serialize() {
        let ep = ExecutionPlan {
            name: "prod".to_string(),
            changes: vec![],
            execution_order: vec!["a".to_string(), "b".to_string()],
            to_create: 1,
            to_update: 2,
            to_destroy: 0,
            unchanged: 3,
        };
        let json = serde_json::to_string(&ep).unwrap();
        assert!(json.contains("\"to_create\":1"));
        assert!(json.contains("\"unchanged\":3"));
    }

    #[test]
    fn test_fj131_yaml_value_to_string_mapping() {
        let mut map = serde_yaml_ng::Mapping::new();
        map.insert(
            serde_yaml_ng::Value::String("key".into()),
            serde_yaml_ng::Value::String("val".into()),
        );
        let val = serde_yaml_ng::Value::Mapping(map);
        let s = yaml_value_to_string(&val);
        assert!(
            !s.is_empty(),
            "Mapping should produce non-empty debug string"
        );
    }

    #[test]
    fn test_fj131_yaml_value_to_string_float() {
        let n = serde_yaml_ng::Number::from(9.81_f64);
        let val = serde_yaml_ng::Value::Number(n);
        assert_eq!(yaml_value_to_string(&val), "9.81");
    }

    #[test]
    fn test_fj131_config_minimal_defaults() {
        // Minimal config with all defaults
        let yaml = r#"
version: "1.0"
name: minimal
resources:
  f:
    type: file
    path: /tmp/test
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert!(config.description.is_none());
        assert!(config.params.is_empty());
        assert!(config.machines.is_empty());
        assert!(config.policy.tripwire); // default true
        assert!(config.policy.lock_file); // default true
    }

    #[test]
    fn test_fj131_resource_all_fields_roundtrip() {
        let yaml = r#"
version: "1.0"
name: all-fields
machines:
  m:
    hostname: m
    addr: 1.1.1.1
resources:
  full-file:
    type: file
    machine: m
    state: file
    path: /etc/app.conf
    content: "key=val"
    owner: www-data
    group: www-data
    mode: "0600"
    depends_on: []
    arch: [x86_64, aarch64]
    tags: [web, critical]
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let r = &config.resources["full-file"];
        assert_eq!(r.resource_type, ResourceType::File);
        assert_eq!(r.state.as_deref(), Some("file"));
        assert_eq!(r.owner.as_deref(), Some("www-data"));
        assert_eq!(r.mode.as_deref(), Some("0600"));
        assert_eq!(r.arch, vec!["x86_64", "aarch64"]);
        assert_eq!(r.tags, vec!["web", "critical"]);
    }

    #[test]
    fn test_fj131_apply_result_debug() {
        let ar = ApplyResult {
            machine: "web".to_string(),
            resources_converged: 5,
            resources_unchanged: 2,
            resources_failed: 0,
            total_duration: std::time::Duration::from_secs(3),
        };
        let debug = format!("{:?}", ar);
        assert!(debug.contains("web"));
        assert!(debug.contains("5"));
    }

    #[test]
    fn test_fj131_machine_roles_parse() {
        let yaml = r#"
hostname: gpu-01
addr: 10.0.0.5
roles: [gpu-compute, training, inference]
"#;
        let m: Machine = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(m.roles.len(), 3);
        assert_eq!(m.roles[0], "gpu-compute");
    }

    // --- FJ-132: Types edge case tests ---

    #[test]
    fn test_fj132_machine_target_to_vec_single() {
        let t = MachineTarget::Single("web".to_string());
        assert_eq!(t.to_vec(), vec!["web".to_string()]);
    }

    #[test]
    fn test_fj132_machine_target_to_vec_multiple() {
        let t = MachineTarget::Multiple(vec!["web".into(), "db".into(), "cache".into()]);
        assert_eq!(t.to_vec(), vec!["web", "db", "cache"]);
    }

    #[test]
    fn test_fj132_resource_type_clone() {
        let rt = ResourceType::Docker;
        let cloned = rt.clone();
        assert_eq!(format!("{}", rt), format!("{}", cloned));
    }

    #[test]
    fn test_fj132_resource_status_all_variants_display() {
        // Verify all four variants have non-empty Display output
        for status in &[
            ResourceStatus::Converged,
            ResourceStatus::Failed,
            ResourceStatus::Drifted,
            ResourceStatus::Unknown,
        ] {
            let s = format!("{}", status);
            assert!(!s.is_empty(), "ResourceStatus display should not be empty");
        }
    }

    #[test]
    fn test_fj132_plan_action_all_variants() {
        // Verify all four variants have non-empty Display output
        for action in &[
            PlanAction::Create,
            PlanAction::Update,
            PlanAction::Destroy,
            PlanAction::NoOp,
        ] {
            let s = format!("{}", action);
            assert!(!s.is_empty(), "PlanAction display should not be empty");
        }
    }

    #[test]
    fn test_fj132_resource_defaults() {
        // A resource with minimal fields should have sensible defaults
        let yaml = r#"
type: file
machine: m1
path: /tmp/test
"#;
        let r: Resource = serde_yaml_ng::from_str(yaml).unwrap();
        assert!(r.packages.is_empty());
        assert!(r.depends_on.is_empty());
        assert!(r.restart_on.is_empty());
        assert!(r.tags.is_empty());
        assert!(r.arch.is_empty());
        assert!(r.ports.is_empty());
        assert!(r.volumes.is_empty());
        assert!(r.environment.is_empty());
        assert!(r.ssh_authorized_keys.is_empty());
        assert!(r.groups.is_empty());
    }

    #[test]
    fn test_fj132_container_config_ephemeral_default_true() {
        let yaml = r#"
runtime: docker
image: ubuntu:22.04
"#;
        let c: ContainerConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert!(c.ephemeral, "ephemeral should default to true");
        assert!(c.init, "init should default to true");
        assert!(!c.privileged, "privileged should default to false");
    }

    #[test]
    fn test_fj132_yaml_value_to_string_null() {
        let val = serde_yaml_ng::Value::Null;
        assert_eq!(yaml_value_to_string(&val), "");
    }

    #[test]
    fn test_fj132_yaml_value_to_string_bool() {
        let val = serde_yaml_ng::Value::Bool(true);
        assert_eq!(yaml_value_to_string(&val), "true");
        let val = serde_yaml_ng::Value::Bool(false);
        assert_eq!(yaml_value_to_string(&val), "false");
    }

    #[test]
    fn test_fj132_machine_is_container_transport() {
        let m = Machine {
            hostname: "box".to_string(),
            addr: "container".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("container".to_string()),
            container: None,
            cost: 0,
        };
        assert!(m.is_container_transport());
    }

    #[test]
    fn test_fj132_machine_is_not_container_transport() {
        let m = Machine {
            hostname: "web".to_string(),
            addr: "10.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };
        assert!(!m.is_container_transport());
    }

    #[test]
    fn test_fj132_machine_container_name_explicit() {
        let m = Machine {
            hostname: "box".to_string(),
            addr: "container".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("container".to_string()),
            container: Some(ContainerConfig {
                runtime: "docker".to_string(),
                image: Some("ubuntu:22.04".to_string()),
                name: Some("my-custom-name".to_string()),
                ephemeral: true,
                privileged: false,
                init: true,
            }),
            cost: 0,
        };
        assert_eq!(m.container_name(), "my-custom-name");
    }

    #[test]
    fn test_fj132_machine_container_name_derived() {
        let m = Machine {
            hostname: "test-box".to_string(),
            addr: "container".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("container".to_string()),
            container: Some(ContainerConfig {
                runtime: "docker".to_string(),
                image: Some("ubuntu:22.04".to_string()),
                name: None,
                ephemeral: true,
                privileged: false,
                init: true,
            }),
            cost: 0,
        };
        assert_eq!(m.container_name(), "forjar-test-box");
    }

    #[test]
    fn test_fj132_resource_type_display_all() {
        let types = [
            (ResourceType::Package, "package"),
            (ResourceType::File, "file"),
            (ResourceType::Service, "service"),
            (ResourceType::Mount, "mount"),
            (ResourceType::User, "user"),
            (ResourceType::Docker, "docker"),
            (ResourceType::Cron, "cron"),
            (ResourceType::Network, "network"),
        ];
        for (rt, expected) in &types {
            assert_eq!(format!("{}", rt), *expected);
        }
    }

    #[test]
    fn test_fj132_policy_defaults() {
        let policy = Policy::default();
        assert!(matches!(policy.failure, FailurePolicy::StopOnFirst));
        assert!(policy.tripwire);
        assert!(policy.lock_file);
        assert!(!policy.parallel_machines);
    }

    #[test]
    fn test_fj132_machine_target_single_deserialization() {
        let yaml = "machine: web";
        let r: Resource =
            serde_yaml_ng::from_str(&format!("type: file\n{}\npath: /tmp/x", yaml)).unwrap();
        match &r.machine {
            MachineTarget::Single(name) => assert_eq!(name, "web"),
            MachineTarget::Multiple(_) => panic!("expected Single"),
        }
    }

    #[test]
    fn test_fj132_machine_target_multiple_deserialization() {
        let yaml = "type: file\nmachine: [web, db]\npath: /tmp/x";
        let r: Resource = serde_yaml_ng::from_str(yaml).unwrap();
        match &r.machine {
            MachineTarget::Multiple(names) => {
                assert_eq!(names.len(), 2);
                assert_eq!(names[0], "web");
                assert_eq!(names[1], "db");
            }
            MachineTarget::Single(_) => panic!("expected Multiple"),
        }
    }

    // ── FJ-142: Display + PartialEq for MachineTarget/FailurePolicy ──

    #[test]
    fn test_fj142_machine_target_display_single() {
        let t = MachineTarget::Single("web1".to_string());
        assert_eq!(format!("{}", t), "web1");
    }

    #[test]
    fn test_fj142_machine_target_display_multiple() {
        let t = MachineTarget::Multiple(vec!["web1".to_string(), "web2".to_string()]);
        assert_eq!(format!("{}", t), "[web1, web2]");
    }

    #[test]
    fn test_fj142_machine_target_display_empty_multiple() {
        let t = MachineTarget::Multiple(vec![]);
        assert_eq!(format!("{}", t), "[]");
    }

    #[test]
    fn test_fj142_machine_target_partial_eq() {
        let a = MachineTarget::Single("web".to_string());
        let b = MachineTarget::Single("web".to_string());
        let c = MachineTarget::Single("db".to_string());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_fj142_machine_target_eq_multiple() {
        let a = MachineTarget::Multiple(vec!["a".to_string(), "b".to_string()]);
        let b = MachineTarget::Multiple(vec!["a".to_string(), "b".to_string()]);
        let c = MachineTarget::Multiple(vec!["b".to_string(), "a".to_string()]);
        assert_eq!(a, b);
        assert_ne!(a, c); // order matters
    }

    #[test]
    fn test_fj142_failure_policy_display_stop() {
        assert_eq!(format!("{}", FailurePolicy::StopOnFirst), "stop_on_first");
    }

    #[test]
    fn test_fj142_failure_policy_display_continue() {
        assert_eq!(
            format!("{}", FailurePolicy::ContinueIndependent),
            "continue_independent"
        );
    }
}
