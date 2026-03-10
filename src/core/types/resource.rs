//! Resource type definitions: Resource, ResourceType, MachineTarget.

use super::resource_enums::{MachineTarget, ResourceType};
use super::service_mode_types::RestartPolicy;
use super::task_types::{HealthCheck, PipelineStage, QualityGate, TaskMode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

    // -- Task fields (FJ-2700: task framework) --
    /// Task execution mode (batch/pipeline/service/dispatch).
    #[serde(default)]
    pub task_mode: Option<TaskMode>,
    /// Input file patterns for content-addressed caching.
    #[serde(default)]
    pub task_inputs: Vec<String>,
    /// Output artifacts to hash for idempotency.
    #[serde(default)]
    pub output_artifacts: Vec<String>,
    /// Completion check command (exit 0 = done).
    #[serde(default)]
    pub completion_check: Option<String>,
    /// Timeout in seconds.
    #[serde(default)]
    pub timeout: Option<u64>,
    /// Working directory for the command.
    #[serde(default)]
    pub working_dir: Option<String>,
    /// Pipeline stages (mode: pipeline).
    #[serde(default)]
    pub stages: Vec<PipelineStage>,
    /// Enable content-addressed stage caching.
    #[serde(default)]
    pub cache: bool,
    /// GPU device index for CUDA_VISIBLE_DEVICES.
    #[serde(default)]
    pub gpu_device: Option<u32>,
    /// Restart delay in seconds (mode: service).
    #[serde(default)]
    pub restart_delay: Option<u64>,
    /// Quality gate (mode: batch/pipeline) — fail-fast on exit code or parsed output.
    #[serde(default)]
    pub quality_gate: Option<QualityGate>,
    /// Health check (mode: service) — periodic liveness probe with retry/backoff.
    #[serde(default)]
    pub health_check: Option<HealthCheck>,
    /// Restart policy (mode: service) — max restarts, exponential backoff.
    #[serde(default)]
    pub restart_policy: Option<RestartPolicy>,

    // -- FJ-2704: Distributed coordination --
    /// Gather artifacts from remote machines after execution.
    /// Maps remote path → local destination.
    #[serde(default)]
    pub gather: Vec<String>,
    /// Scatter local artifacts to remote machines before execution.
    /// Maps local path → remote destination.
    #[serde(default)]
    pub scatter: Vec<String>,

    // -- Lifecycle hooks + protection --
    /// Pre-apply hook (exit non-zero skips resource).
    #[serde(default)]
    pub pre_apply: Option<String>,
    /// Post-apply hook.
    #[serde(default)]
    pub post_apply: Option<String>,
    /// Lifecycle protection rules.
    #[serde(default)]
    pub lifecycle: Option<LifecycleRules>,
    /// Run apply with sudo.
    #[serde(default)]
    pub sudo: bool,
    /// Enable content-addressed store.
    #[serde(default)]
    pub store: bool,
    /// Build script for derivation resources.
    #[serde(default)]
    pub script: Option<String>,

    // -- Build fields (FJ-33: cross-compile build→deploy) --
    /// Machine that performs the build (distinct from deploy target in `machine`).
    #[serde(default)]
    pub build_machine: Option<String>,

    // -- GitHub Release fields (FJ-34: nightly binary installation) --
    /// GitHub owner/repo (e.g., "paiml/forjar").
    #[serde(default)]
    pub repo: Option<String>,

    /// Release tag (e.g., "nightly", "v1.0.0").
    #[serde(default)]
    pub tag: Option<String>,

    /// Glob pattern to match release asset (e.g., "*aarch64-unknown-linux-gnu*").
    #[serde(default)]
    pub asset_pattern: Option<String>,

    /// Binary name to extract from the asset (e.g., "apr", "forjar").
    #[serde(default)]
    pub binary: Option<String>,

    /// Directory to install binary into (default: /usr/local/bin).
    #[serde(default)]
    pub install_dir: Option<String>,
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
