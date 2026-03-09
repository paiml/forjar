//! Configuration types: ForjarConfig, Machine, DataSource, Policy rules, Outputs.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{default_true, PolicyRule, Resource};

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
/// let config: ForjarConfig = serde_yaml_ng::from_str(yaml).expect("valid YAML");
/// assert_eq!(config.name, "my-infra");
/// assert_eq!(config.machines.len(), 1);
/// assert_eq!(config.resources.len(), 1);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

    /// FJ-2502: Include provenance — maps "resource:id" / "machine:id" / "param:id"
    /// to the include file that contributed it. Not serialized to YAML.
    #[serde(skip)]
    pub include_provenance: HashMap<String, String>,

    /// FJ-1200: Post-apply health check blocks (OpenTofu-style check blocks)
    #[serde(default)]
    pub checks: IndexMap<String, CheckBlock>,

    /// FJ-1210: Declarative resource renames processed before planning
    #[serde(default)]
    pub moved: Vec<MovedEntry>,

    /// FJ-2300: Secret provider configuration
    #[serde(default)]
    pub secrets: SecretsConfig,

    /// FJ-3500: Environment definitions (dev, staging, prod).
    /// Each environment overrides params and machine addresses.
    #[serde(default)]
    pub environments: IndexMap<String, super::environment::Environment>,
}

/// FJ-2300 + FJ-3300: Secret provider configuration.
///
/// Controls how `{{secrets.*}}` template variables are resolved.
/// Providers: "env" (default), "file", "sops", "op" (1Password).
///
/// When `ephemeral: true`, resolved secret values are never written to state.
/// Instead, a BLAKE3 hash is stored for drift detection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecretsConfig {
    /// Provider type: "env" (default), "file", "sops", "op"
    #[serde(default)]
    pub provider: Option<String>,

    /// Path prefix for file-based secrets (used with `provider: file`)
    #[serde(default)]
    pub path: Option<String>,

    /// Encrypted file path for SOPS provider (used with `provider: sops`)
    #[serde(default)]
    pub file: Option<String>,

    /// FJ-3300: When true, secret values are never persisted to state files.
    /// A BLAKE3 hash is stored instead, enabling drift detection without
    /// exposing cleartext secrets at rest.
    #[serde(default)]
    pub ephemeral: bool,
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

// PolicyRule, PolicyRuleType, PolicyViolation moved to policy_rule_types.rs (FJ-3200)

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
/// let machine: Machine = serde_yaml_ng::from_str(yaml).expect("valid YAML");
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

    /// FJ-2300: Operators allowed to apply to this machine.
    /// Empty = no restriction (backward compatible).
    #[serde(default)]
    pub allowed_operators: Vec<String>,
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
    /// Construct an SSH machine with hostname, address, and user.
    /// All other fields use sensible defaults.
    pub fn ssh(hostname: &str, addr: &str, user: &str) -> Self {
        Self {
            hostname: hostname.to_string(),
            addr: addr.to_string(),
            user: user.to_string(),
            arch: default_arch(),
            ssh_key: None,
            roles: Vec::new(),
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
            allowed_operators: Vec::new(),
        }
    }

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

    /// FJ-2300: Check if an operator is authorized for this machine.
    ///
    /// Returns true if `allowed_operators` is empty (no restriction)
    /// or the operator is in the allowed list.
    pub fn is_operator_allowed(&self, operator: &str) -> bool {
        self.allowed_operators.is_empty() || self.allowed_operators.iter().any(|o| o == operator)
    }
}

fn default_user() -> String {
    "root".to_string()
}

fn default_arch() -> String {
    "x86_64".to_string()
}
