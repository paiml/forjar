//! Configuration types: ForjarConfig, Machine, DataSource, Policy rules, Outputs.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{default_true, Resource};

// ============================================================================
// Top-level forjar.yaml
// ============================================================================

/// Root configuration — the desired state of infrastructure.
///
/// # Examples
///
/// ```
/// use forjar::core::types::ForjarConfig;
///
/// let yaml = r#"
/// version: "1.0"
/// name: my-infra
/// machines:
///   web:
///     hostname: web-01
///     addr: 10.0.0.1
/// resources:
///   pkg-nginx:
///     type: package
///     machine: web
///     packages: [nginx]
/// "#;
/// let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
/// assert_eq!(config.name, "my-infra");
/// assert_eq!(config.machines.len(), 1);
/// assert_eq!(config.resources.len(), 1);
/// ```
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
    #[serde(default)]
    pub resources: IndexMap<String, Resource>,

    /// Execution policy
    #[serde(default)]
    pub policy: super::Policy,

    /// FJ-215: Output values — computed from params/templates, written to state/outputs.yaml
    #[serde(default)]
    pub outputs: IndexMap<String, OutputValue>,

    /// FJ-220: Policy rules for plan-time enforcement
    #[serde(default)]
    pub policies: Vec<PolicyRule>,

    /// FJ-223: External data sources resolved at plan time
    #[serde(default)]
    pub data: IndexMap<String, DataSource>,

    /// FJ-254: Config includes — merge multiple YAML files
    #[serde(default)]
    pub includes: Vec<String>,

    /// FJ-1200: Post-apply health check blocks (OpenTofu-style check blocks)
    #[serde(default)]
    pub checks: IndexMap<String, CheckBlock>,

    /// FJ-1210: Declarative resource renames processed before planning
    #[serde(default)]
    pub moved: Vec<MovedEntry>,
}

/// FJ-1200: A post-apply health check assertion.
///
/// Check blocks run AFTER all resources converge. Failures are warnings
/// by default (like OpenTofu) — they don't roll back the apply.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckBlock {
    /// Target machine to run the check on
    pub machine: String,

    /// Shell command to execute (exit 0 = pass)
    pub command: String,

    /// Expected exit code (default: 0)
    #[serde(default)]
    pub expect_exit: Option<i32>,

    /// Human-readable description of what the check validates
    #[serde(default)]
    pub description: Option<String>,
}

/// FJ-1210: A declarative resource rename entry.
///
/// Processed during planning before the diff — state is updated in-place.
/// After the first successful apply, the moved block can be removed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovedEntry {
    /// Old resource name (in state)
    pub from: String,

    /// New resource name (in config)
    pub to: String,
}

/// FJ-223: External data source definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSource {
    /// Source type: `file`, `command`, `dns`, or `forjar-state`
    #[serde(rename = "type")]
    pub source_type: DataSourceType,

    /// For `file`: path to read. For `command`: shell command. For `dns`: hostname.
    #[serde(default)]
    pub value: Option<String>,

    /// Optional default if the data source fails (prevents hard errors)
    #[serde(default)]
    pub default: Option<String>,

    /// FJ-1250: State directory for forjar-state data sources
    #[serde(default)]
    pub state_dir: Option<String>,

    /// FJ-1250: Config name to import outputs from
    #[serde(default)]
    pub config: Option<String>,

    /// FJ-1250: Output names to import
    #[serde(default)]
    pub outputs: Vec<String>,

    /// FJ-1270: Maximum age before outputs are considered stale (e.g., "1h", "24h", "7d")
    #[serde(default)]
    pub max_staleness: Option<String>,
}

/// Data source type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSourceType {
    /// Read content from a local file
    File,
    /// Run shell command and capture stdout
    Command,
    /// Resolve DNS hostname to IP
    Dns,
    /// FJ-1250: Read outputs from another forjar config's state
    #[serde(rename = "forjar-state")]
    ForjarState,
}

/// FJ-220: A policy rule for plan-time enforcement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Rule severity: `require`, `deny`, or `warn`
    #[serde(rename = "type")]
    pub rule_type: PolicyRuleType,

    /// Human-readable description of what this rule checks
    pub message: String,

    /// Resource type filter (e.g., "file", "package"). None = all types.
    #[serde(default)]
    pub resource_type: Option<String>,

    /// Tag filter — only check resources with this tag
    #[serde(default)]
    pub tag: Option<String>,

    /// For `require`: field that must be set (e.g., "owner", "tags", "mode")
    #[serde(default)]
    pub field: Option<String>,

    /// For `deny`/`warn`: field to check
    #[serde(default)]
    pub condition_field: Option<String>,

    /// For `deny`/`warn`: value that triggers the rule (equality check)
    #[serde(default)]
    pub condition_value: Option<String>,
}

/// Policy rule severity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyRuleType {
    /// Resource must have a field set
    Require,
    /// Block apply if condition matches
    Deny,
    /// Advisory warning (does not block)
    Warn,
}

/// Result of evaluating a policy rule against a resource.
#[derive(Debug, Clone)]
pub struct PolicyViolation {
    /// Rule that was violated
    pub rule_message: String,
    /// Resource that violated the rule
    pub resource_id: String,
    /// Severity
    pub severity: PolicyRuleType,
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
///
/// # Examples
///
/// ```
/// use forjar::core::types::Machine;
///
/// let yaml = r#"
/// hostname: web-01
/// addr: 10.0.0.1
/// roles: [web, app]
/// "#;
/// let machine: Machine = serde_yaml_ng::from_str(yaml).unwrap();
/// assert_eq!(machine.hostname, "web-01");
/// assert_eq!(machine.user, "root"); // default
/// assert_eq!(machine.arch, "x86_64"); // default
/// assert!(!machine.is_container_transport());
/// ```
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

    /// FJ-230: Pepita transport configuration (required when `transport: pepita`)
    #[serde(default)]
    pub pepita: Option<PepitaTransportConfig>,

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

    /// GPU device access: `"all"`, `"device=0"`, etc. Maps to `--gpus` flag (NVIDIA).
    #[serde(default)]
    pub gpus: Option<String>,

    /// Device passthrough via `--device` (e.g., `/dev/kfd`, `/dev/dri` for AMD ROCm).
    #[serde(default)]
    pub devices: Vec<String>,

    /// Additional groups via `--group-add` (e.g., `video`, `render` for GPU device access).
    #[serde(default)]
    pub group_add: Vec<String>,

    /// Environment variables via `--env` (e.g., `CUDA_VISIBLE_DEVICES`, `ROCR_VISIBLE_DEVICES`).
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,

    /// Volume mounts via `-v` (e.g., `/var/run/docker.sock:/var/run/docker.sock`).
    #[serde(default)]
    pub volumes: Vec<String>,
}

fn default_runtime() -> String {
    "docker".to_string()
}

/// FJ-230: Pepita kernel namespace transport configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PepitaTransportConfig {
    /// Root filesystem: path to base rootfs or `debootstrap:jammy`
    pub rootfs: String,

    /// cgroup v2 memory limit in MB (optional)
    #[serde(default)]
    pub memory_mb: Option<u64>,

    /// cgroup v2 CPU limit (optional)
    #[serde(default)]
    pub cpus: Option<f64>,

    /// Network mode: `isolated` (new netns) or `host` (share host netns)
    #[serde(default = "default_pepita_network")]
    pub network: String,

    /// Filesystem mode: `overlay` (overlayfs) or `bind` (bind mount)
    #[serde(default = "default_pepita_filesystem")]
    pub filesystem: String,

    /// Destroy namespace after apply (default: true)
    #[serde(default = "default_true")]
    pub ephemeral: bool,
}

fn default_pepita_network() -> String {
    "isolated".to_string()
}

fn default_pepita_filesystem() -> String {
    "overlay".to_string()
}

impl Machine {
    /// Returns true if this machine uses container transport.
    ///
    /// # Examples
    ///
    /// ```
    /// use forjar::core::types::Machine;
    ///
    /// let ssh: Machine = serde_yaml_ng::from_str("hostname: h\naddr: 10.0.0.1").unwrap();
    /// assert!(!ssh.is_container_transport());
    ///
    /// let ct: Machine = serde_yaml_ng::from_str("hostname: h\naddr: container").unwrap();
    /// assert!(ct.is_container_transport());
    /// ```
    pub fn is_container_transport(&self) -> bool {
        self.transport.as_deref() == Some("container") || self.addr == "container"
    }

    /// Returns the effective container name (explicit or derived from hostname).
    ///
    /// # Examples
    ///
    /// ```
    /// use forjar::core::types::Machine;
    ///
    /// let m: Machine = serde_yaml_ng::from_str("hostname: ci-01\naddr: container").unwrap();
    /// assert_eq!(m.container_name(), "forjar-ci-01");
    /// ```
    pub fn container_name(&self) -> String {
        self.container
            .as_ref()
            .and_then(|c| c.name.clone())
            .unwrap_or_else(|| format!("forjar-{}", self.hostname))
    }

    /// Returns true if this machine uses pepita (kernel namespace) transport.
    pub fn is_pepita_transport(&self) -> bool {
        self.transport.as_deref() == Some("pepita") || self.addr == "pepita"
    }

    /// Returns the effective pepita namespace name (derived from hostname).
    pub fn pepita_name(&self) -> String {
        format!("forjar-ns-{}", self.hostname)
    }
}

fn default_user() -> String {
    "root".to_string()
}

fn default_arch() -> String {
    "x86_64".to_string()
}
