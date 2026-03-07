//! FJ-2700: Service mode types — health check loops, restart policies, dispatch.

use serde::{Deserialize, Serialize};
use std::fmt;

/// FJ-2700: Restart policy for service-mode tasks.
///
/// # Examples
///
/// ```
/// use forjar::core::types::RestartPolicy;
///
/// let policy = RestartPolicy::default();
/// assert_eq!(policy.max_restarts, 5);
/// assert!(policy.should_restart(3));
/// assert!(!policy.should_restart(5));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartPolicy {
    /// Maximum number of restarts before giving up (default: 5).
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,
    /// Backoff base delay in seconds (default: 1.0).
    #[serde(default = "default_backoff_base")]
    pub backoff_base_secs: f64,
    /// Backoff maximum delay in seconds (default: 60.0).
    #[serde(default = "default_backoff_max")]
    pub backoff_max_secs: f64,
    /// Reset restart count after healthy for this many seconds (default: 300).
    #[serde(default = "default_reset_after")]
    pub reset_after_secs: u32,
}

fn default_max_restarts() -> u32 {
    5
}
fn default_backoff_base() -> f64 {
    1.0
}
fn default_backoff_max() -> f64 {
    60.0
}
fn default_reset_after() -> u32 {
    300
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self {
            max_restarts: default_max_restarts(),
            backoff_base_secs: default_backoff_base(),
            backoff_max_secs: default_backoff_max(),
            reset_after_secs: default_reset_after(),
        }
    }
}

impl RestartPolicy {
    /// Whether to restart given the current failure count.
    pub fn should_restart(&self, failures: u32) -> bool {
        failures < self.max_restarts
    }

    /// Calculate backoff delay for the Nth restart (exponential with cap).
    pub fn backoff_secs(&self, restart_num: u32) -> f64 {
        let delay = self.backoff_base_secs * 2.0_f64.powi(restart_num as i32);
        delay.min(self.backoff_max_secs)
    }
}

/// FJ-2700: Health check result from a single check execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Whether the check passed (exit 0).
    pub healthy: bool,
    /// Exit code of the health check command.
    pub exit_code: i32,
    /// Duration of the check in seconds.
    pub duration_secs: f64,
    /// ISO 8601 timestamp.
    pub checked_at: String,
    /// Stdout from the check (may be empty).
    #[serde(default)]
    pub stdout: String,
}

impl fmt::Display for HealthCheckResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.healthy { "healthy" } else { "unhealthy" };
        write!(
            f,
            "{status} (exit={}, {:.2}s) at {}",
            self.exit_code, self.duration_secs, self.checked_at,
        )
    }
}

/// FJ-2700: Service lifecycle event for logging.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ServiceEvent {
    /// Service started.
    Started { pid: u32, at: String },
    /// Health check passed.
    HealthOk { at: String },
    /// Health check failed.
    HealthFail {
        exit_code: i32,
        consecutive: u32,
        at: String,
    },
    /// Service restarted due to health check failures.
    Restarted {
        old_pid: u32,
        new_pid: u32,
        restart_count: u32,
        at: String,
    },
    /// Service stopped (manual or max restarts exceeded).
    Stopped { reason: String, at: String },
}

impl fmt::Display for ServiceEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Started { pid, at } => write!(f, "[{at}] started (pid {pid})"),
            Self::HealthOk { at } => write!(f, "[{at}] health: ok"),
            Self::HealthFail {
                exit_code,
                consecutive,
                at,
            } => {
                write!(
                    f,
                    "[{at}] health: fail (exit={exit_code}, consecutive={consecutive})"
                )
            }
            Self::Restarted {
                restart_count, at, ..
            } => {
                write!(f, "[{at}] restarted (#{restart_count})")
            }
            Self::Stopped { reason, at } => write!(f, "[{at}] stopped: {reason}"),
        }
    }
}

/// FJ-2700: Dispatch configuration for on-demand tasks.
///
/// # Examples
///
/// ```
/// use forjar::core::types::DispatchConfig;
///
/// let config = DispatchConfig {
///     name: "deploy".into(),
///     command: "make deploy".into(),
///     params: vec![("env".into(), "production".into())],
///     timeout_secs: Some(300),
/// };
/// assert_eq!(config.name, "deploy");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchConfig {
    /// Dispatch task name (used in `forjar run <name>`).
    pub name: String,
    /// Command to execute.
    pub command: String,
    /// Parameters passed to the command.
    #[serde(default)]
    pub params: Vec<(String, String)>,
    /// Execution timeout in seconds (None = no timeout).
    #[serde(default)]
    pub timeout_secs: Option<u32>,
}

/// FJ-2701: Input/output hash tracking for state.lock.yaml.
///
/// # Examples
///
/// ```
/// use forjar::core::types::IoHashes;
///
/// let hashes = IoHashes {
///     input_hash: Some("blake3:abc123".into()),
///     output_hash: Some("blake3:def456".into()),
///     cached: true,
/// };
/// assert!(hashes.is_cached());
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IoHashes {
    /// BLAKE3 hash of inputs (source files, dependencies).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_hash: Option<String>,
    /// BLAKE3 hash of outputs (artifacts, build results).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_hash: Option<String>,
    /// Whether this resource was served from cache.
    #[serde(default)]
    pub cached: bool,
}

impl IoHashes {
    /// Whether the resource was served from cache.
    pub fn is_cached(&self) -> bool {
        self.cached
    }

    /// Whether both input and output hashes are present.
    pub fn is_complete(&self) -> bool {
        self.input_hash.is_some() && self.output_hash.is_some()
    }
}

/// FJ-2701: GPU memory info field for state tracking.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GpuMemoryInfo {
    /// Total GPU memory in bytes.
    #[serde(default)]
    pub total_bytes: Option<u64>,
    /// Used GPU memory in bytes.
    #[serde(default)]
    pub used_bytes: Option<u64>,
    /// GPU device name.
    #[serde(default)]
    pub device: Option<String>,
}

impl GpuMemoryInfo {
    /// Used memory in MB.
    pub fn used_mb(&self) -> Option<f64> {
        self.used_bytes.map(|b| b as f64 / (1024.0 * 1024.0))
    }

    /// Utilization percentage.
    pub fn utilization_pct(&self) -> Option<f64> {
        match (self.used_bytes, self.total_bytes) {
            (Some(used), Some(total)) if total > 0 => Some(used as f64 / total as f64 * 100.0),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restart_policy_default() {
        let p = RestartPolicy::default();
        assert_eq!(p.max_restarts, 5);
        assert!((p.backoff_base_secs - 1.0).abs() < 0.001);
        assert!((p.backoff_max_secs - 60.0).abs() < 0.001);
    }

    #[test]
    fn restart_policy_should_restart() {
        let p = RestartPolicy::default();
        assert!(p.should_restart(0));
        assert!(p.should_restart(4));
        assert!(!p.should_restart(5));
        assert!(!p.should_restart(10));
    }

    #[test]
    fn restart_policy_backoff() {
        let p = RestartPolicy::default();
        assert!((p.backoff_secs(0) - 1.0).abs() < 0.001);
        assert!((p.backoff_secs(1) - 2.0).abs() < 0.001);
        assert!((p.backoff_secs(2) - 4.0).abs() < 0.001);
        assert!((p.backoff_secs(6) - 60.0).abs() < 0.001); // capped
        assert!((p.backoff_secs(10) - 60.0).abs() < 0.001); // still capped
    }

    #[test]
    fn health_check_result_display() {
        let ok = HealthCheckResult {
            healthy: true,
            exit_code: 0,
            duration_secs: 0.05,
            checked_at: "2026-03-05T14:30:00Z".into(),
            stdout: String::new(),
        };
        assert!(ok.to_string().contains("healthy"));

        let fail = HealthCheckResult {
            healthy: false,
            exit_code: 1,
            duration_secs: 5.0,
            checked_at: "2026-03-05T14:30:30Z".into(),
            stdout: "timeout".into(),
        };
        assert!(fail.to_string().contains("unhealthy"));
    }

    #[test]
    fn service_event_display() {
        let started = ServiceEvent::Started {
            pid: 1234,
            at: "t0".into(),
        };
        assert!(started.to_string().contains("pid 1234"));

        let fail = ServiceEvent::HealthFail {
            exit_code: 1,
            consecutive: 3,
            at: "t1".into(),
        };
        assert!(fail.to_string().contains("consecutive=3"));

        let stopped = ServiceEvent::Stopped {
            reason: "max restarts".into(),
            at: "t2".into(),
        };
        assert!(stopped.to_string().contains("max restarts"));
    }

    #[test]
    fn service_event_serde() {
        let event = ServiceEvent::Restarted {
            old_pid: 100,
            new_pid: 200,
            restart_count: 2,
            at: "ts".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("restarted"));
        let parsed: ServiceEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, ServiceEvent::Restarted { .. }));
    }

    #[test]
    fn dispatch_config_serde() {
        let cfg = DispatchConfig {
            name: "build".into(),
            command: "cargo build".into(),
            params: vec![("profile".into(), "release".into())],
            timeout_secs: Some(600),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: DispatchConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "build");
        assert_eq!(parsed.timeout_secs, Some(600));
    }

    #[test]
    fn io_hashes_cached() {
        let h = IoHashes {
            input_hash: Some("blake3:abc".into()),
            output_hash: Some("blake3:def".into()),
            cached: true,
        };
        assert!(h.is_cached());
        assert!(h.is_complete());
    }

    #[test]
    fn io_hashes_incomplete() {
        let h = IoHashes {
            input_hash: Some("blake3:abc".into()),
            output_hash: None,
            cached: false,
        };
        assert!(!h.is_complete());
    }

    #[test]
    fn gpu_memory_info_utilization() {
        let info = GpuMemoryInfo {
            total_bytes: Some(8 * 1024 * 1024 * 1024),
            used_bytes: Some(4 * 1024 * 1024 * 1024),
            device: Some("NVIDIA RTX 4090".into()),
        };
        let pct = info.utilization_pct().unwrap();
        assert!((pct - 50.0).abs() < 0.01);
        let mb = info.used_mb().unwrap();
        assert!((mb - 4096.0).abs() < 0.01);
    }

    #[test]
    fn gpu_memory_info_empty() {
        let info = GpuMemoryInfo::default();
        assert!(info.utilization_pct().is_none());
        assert!(info.used_mb().is_none());
    }

    #[test]
    fn restart_policy_serde_roundtrip() {
        let p = RestartPolicy {
            max_restarts: 3,
            backoff_base_secs: 2.0,
            backoff_max_secs: 30.0,
            reset_after_secs: 120,
        };
        let yaml = serde_yaml_ng::to_string(&p).unwrap();
        let parsed: RestartPolicy = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(parsed.max_restarts, 3);
    }
}
