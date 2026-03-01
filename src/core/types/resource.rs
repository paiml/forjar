//! Resource type definitions: Resource, ResourceType, MachineTarget.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// ============================================================================
// Resources
// ============================================================================

/// A single infrastructure resource.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

    /// Source path (file: local path for copy; package/cargo: --path for local crate)
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

    /// FJ-224: General-purpose triggers — force re-apply when listed resources change.
    /// Unlike `depends_on` (execution order) and `restart_on` (service-specific),
    /// triggers work on any resource type.
    #[serde(default)]
    pub triggers: Vec<String>,

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

    /// FJ-281: Resource group for batch operations (e.g., `resource_group: network`)
    #[serde(default)]
    pub resource_group: Option<String>,

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

    // -- Model fields (FJ-240: ML model resource) --
    /// Model format: gguf, safetensors, apr
    #[serde(default)]
    pub format: Option<String>,

    /// Quantization level: q4_k_m, q5_k_m, q8_0, f16, none
    #[serde(default)]
    pub quantization: Option<String>,

    /// BLAKE3 checksum for model integrity verification (pin to exact version)
    #[serde(default)]
    pub checksum: Option<String>,

    /// Model cache directory (default: ~/.cache/apr/)
    #[serde(default)]
    pub cache_dir: Option<String>,

    // -- GPU fields (FJ-241: GPU hardware resource) --
    /// GPU backend: "nvidia" (default), "rocm", or "cpu"
    #[serde(default)]
    pub gpu_backend: Option<String>,

    /// NVIDIA driver version (e.g., "535")
    #[serde(default)]
    pub driver_version: Option<String>,

    /// CUDA toolkit version (e.g., "12.3")
    #[serde(default)]
    pub cuda_version: Option<String>,

    /// AMD ROCm version (e.g., "6.0") — used when gpu_backend = "rocm"
    #[serde(default)]
    pub rocm_version: Option<String>,

    /// GPU device indices (default: all)
    #[serde(default)]
    pub devices: Vec<u32>,

    /// Enable nvidia-persistenced (default: true)
    #[serde(default)]
    pub persistence_mode: Option<bool>,

    /// GPU compute mode: default, exclusive_process, prohibited
    #[serde(default)]
    pub compute_mode: Option<String>,

    /// GPU memory limit in MB (cgroup)
    #[serde(default)]
    pub gpu_memory_limit_mb: Option<u64>,

    // -- Task fields (ALB-027: pipeline orchestration) --
    // Note: `command` field is shared with cron (line 132)

    /// Output artifacts to hash for idempotency (glob paths relative to cwd)
    #[serde(default)]
    pub output_artifacts: Vec<String>,

    /// Shell command to check if task already completed (exit 0 = done, skip apply)
    #[serde(default)]
    pub completion_check: Option<String>,

    /// Timeout in seconds for command execution (default: no limit)
    #[serde(default)]
    pub timeout: Option<u64>,

    /// Working directory for the command
    #[serde(default)]
    pub working_dir: Option<String>,

    // -- Lifecycle hooks (FJ-265) --
    /// Shell command to run on the target before the resource's main script.
    /// If pre_apply exits non-zero, the resource is skipped (not applied).
    #[serde(default)]
    pub pre_apply: Option<String>,

    /// Shell command to run on the target after the resource's main script succeeds.
    #[serde(default)]
    pub post_apply: Option<String>,

    // -- Lifecycle protection rules (FJ-1220) --
    /// OpenTofu-style lifecycle rules: prevent_destroy, create_before_destroy, ignore_drift.
    #[serde(default)]
    pub lifecycle: Option<LifecycleRules>,
}

/// FJ-1220: Lifecycle protection rules for a resource.
///
/// Controls how a resource is handled during destroy, replacement, and drift detection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LifecycleRules {
    /// Prevent this resource from being destroyed (forjar destroy skips with warning)
    #[serde(default)]
    pub prevent_destroy: bool,

    /// Write new version before removing old (avoids config-absent window)
    #[serde(default)]
    pub create_before_destroy: bool,

    /// Fields whose drift is suppressed (reported as "suppressed" not "detected")
    #[serde(default)]
    pub ignore_drift: Vec<String>,
}

/// Resource type enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    #[default]
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
    /// FJ-240: ML model resource type
    Model,
    /// FJ-241: GPU hardware resource type
    Gpu,
    /// ALB-027: Pipeline task resource type
    Task,
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
            Self::Model => write!(f, "model"),
            Self::Gpu => write!(f, "gpu"),
            Self::Task => write!(f, "task"),
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
