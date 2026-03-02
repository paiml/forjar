//! FJ-1315: Build sandbox configuration.
//!
//! Defines sandbox settings for store builds: isolation level, resource limits,
//! and preset profiles. Extends pepita namespace isolation for content-addressed
//! store builds with read-only bind mounts, seccomp BPF, and resource limits.

use serde::{Deserialize, Serialize};

/// Sandbox isolation level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SandboxLevel {
    /// Full isolation: no network, read-only inputs, seccomp, cgroups
    Full,
    /// Network access allowed, but filesystem isolation enforced
    NetworkOnly,
    /// Minimal isolation: PID/mount namespaces, no seccomp
    Minimal,
    /// No sandbox (legacy behavior)
    None,
}

/// Resource limits for sandboxed builds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Isolation level
    pub level: SandboxLevel,

    /// Memory limit in megabytes (default: 2048)
    #[serde(default = "default_memory_mb")]
    pub memory_mb: u64,

    /// CPU limit (fractional cores, default: 4.0)
    #[serde(default = "default_cpus")]
    pub cpus: f64,

    /// Build timeout in seconds (default: 600)
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// Additional read-only bind mounts (host_path → sandbox_path)
    #[serde(default)]
    pub bind_mounts: Vec<BindMount>,

    /// Environment variables to pass into the sandbox
    #[serde(default)]
    pub env: Vec<EnvVar>,
}

/// A read-only bind mount for sandbox inputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BindMount {
    /// Host path to bind
    pub source: String,
    /// Path inside the sandbox
    pub target: String,
    /// Read-only (default: true)
    #[serde(default = "default_readonly")]
    pub readonly: bool,
}

/// An environment variable for the sandbox.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

fn default_memory_mb() -> u64 {
    2048
}
fn default_cpus() -> f64 {
    4.0
}
fn default_timeout() -> u64 {
    600
}
fn default_readonly() -> bool {
    true
}

/// Validate a sandbox config, returning errors.
pub fn validate_config(config: &SandboxConfig) -> Vec<String> {
    let mut errors = Vec::new();

    if config.memory_mb == 0 {
        errors.push("memory_mb must be > 0".to_string());
    }
    if config.cpus <= 0.0 {
        errors.push("cpus must be > 0.0".to_string());
    }
    if config.timeout == 0 {
        errors.push("timeout must be > 0".to_string());
    }
    if config.memory_mb > 1_048_576 {
        errors.push("memory_mb exceeds 1 TiB maximum".to_string());
    }
    if config.cpus > 1024.0 {
        errors.push("cpus exceeds 1024 core maximum".to_string());
    }
    for mount in &config.bind_mounts {
        if mount.source.is_empty() {
            errors.push("bind mount source cannot be empty".to_string());
        }
        if mount.target.is_empty() {
            errors.push("bind mount target cannot be empty".to_string());
        }
    }

    errors
}

/// Create a preset sandbox profile by name.
pub fn preset_profile(name: &str) -> Option<SandboxConfig> {
    match name {
        "full" => Some(SandboxConfig {
            level: SandboxLevel::Full,
            memory_mb: 2048,
            cpus: 4.0,
            timeout: 600,
            bind_mounts: Vec::new(),
            env: Vec::new(),
        }),
        "network-only" => Some(SandboxConfig {
            level: SandboxLevel::NetworkOnly,
            memory_mb: 4096,
            cpus: 8.0,
            timeout: 1200,
            bind_mounts: Vec::new(),
            env: Vec::new(),
        }),
        "minimal" => Some(SandboxConfig {
            level: SandboxLevel::Minimal,
            memory_mb: 1024,
            cpus: 2.0,
            timeout: 300,
            bind_mounts: Vec::new(),
            env: Vec::new(),
        }),
        "gpu" => Some(SandboxConfig {
            level: SandboxLevel::NetworkOnly,
            memory_mb: 16384,
            cpus: 8.0,
            timeout: 3600,
            bind_mounts: vec![BindMount {
                source: "/dev/nvidia0".to_string(),
                target: "/dev/nvidia0".to_string(),
                readonly: false,
            }],
            env: vec![EnvVar {
                name: "NVIDIA_VISIBLE_DEVICES".to_string(),
                value: "all".to_string(),
            }],
        }),
        _ => None,
    }
}

/// Parse sandbox config from YAML string.
pub fn parse_sandbox_config(yaml: &str) -> Result<SandboxConfig, String> {
    serde_yaml_ng::from_str(yaml).map_err(|e| format!("invalid sandbox config: {e}"))
}

/// Check whether a sandbox level blocks network access.
pub fn blocks_network(level: SandboxLevel) -> bool {
    matches!(level, SandboxLevel::Full)
}

/// Check whether a sandbox level enforces filesystem isolation.
pub fn enforces_fs_isolation(level: SandboxLevel) -> bool {
    matches!(
        level,
        SandboxLevel::Full | SandboxLevel::NetworkOnly | SandboxLevel::Minimal
    )
}

/// Compute cgroup path for a sandbox build.
pub fn cgroup_path(store_hash: &str) -> String {
    let hash = store_hash.strip_prefix("blake3:").unwrap_or(store_hash);
    format!("/sys/fs/cgroup/forjar-build-{}", &hash[..16.min(hash.len())])
}
