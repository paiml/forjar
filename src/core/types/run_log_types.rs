//! FJ-2301: Run log types — persistent transport output capture.
//!
//! Every `forjar apply`, `forjar destroy`, or `forjar undo` invocation creates
//! a run log that captures the full output of every script executed.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// FJ-2301: Run log metadata (stored as `meta.yaml` per run).
///
/// # Examples
///
/// ```
/// use forjar::core::types::RunMeta;
///
/// let meta = RunMeta::new("r-abc123".into(), "intel".into(), "apply".into());
/// assert_eq!(meta.run_id, "r-abc123");
/// assert_eq!(meta.command, "apply");
/// assert!(meta.resources.is_empty());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMeta {
    /// Unique run identifier (e.g., "r-c7d16accaf62").
    pub run_id: String,
    /// Target machine name.
    pub machine: String,
    /// Command that initiated this run (apply, destroy, undo).
    pub command: String,
    /// Generation number at time of run.
    #[serde(default)]
    pub generation: Option<u64>,
    /// Operator identity (user@hostname).
    #[serde(default)]
    pub operator: Option<String>,
    /// ISO 8601 start timestamp.
    #[serde(default)]
    pub started_at: Option<String>,
    /// ISO 8601 finish timestamp.
    #[serde(default)]
    pub finished_at: Option<String>,
    /// Total duration in seconds.
    #[serde(default)]
    pub duration_secs: Option<f64>,
    /// Per-resource status.
    #[serde(default)]
    pub resources: HashMap<String, ResourceRunStatus>,
    /// Summary counts.
    #[serde(default)]
    pub summary: RunSummary,
}

impl RunMeta {
    /// Create a new run metadata with the given identifiers.
    pub fn new(run_id: String, machine: String, command: String) -> Self {
        Self {
            run_id,
            machine,
            command,
            generation: None,
            operator: None,
            started_at: None,
            finished_at: None,
            duration_secs: None,
            resources: HashMap::new(),
            summary: RunSummary::default(),
        }
    }

    /// Record a resource action in this run.
    pub fn record_resource(&mut self, resource_id: &str, status: ResourceRunStatus) {
        match &status {
            ResourceRunStatus::Noop => self.summary.noop += 1,
            ResourceRunStatus::Converged { failed: true, .. } => self.summary.failed += 1,
            ResourceRunStatus::Converged { .. } => self.summary.converged += 1,
            ResourceRunStatus::Skipped { .. } => self.summary.skipped += 1,
        }
        self.summary.total += 1;
        self.resources.insert(resource_id.to_string(), status);
    }
}

/// Per-resource status within a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum ResourceRunStatus {
    /// Resource was already converged.
    Noop,
    /// Resource was created or updated.
    Converged {
        /// Process exit code.
        #[serde(default)]
        exit_code: Option<i32>,
        /// Duration in seconds.
        #[serde(default)]
        duration_secs: Option<f64>,
        /// Whether the action failed.
        #[serde(default)]
        failed: bool,
    },
    /// Resource was skipped (dependency failed).
    Skipped {
        /// Reason for skipping.
        #[serde(default)]
        reason: Option<String>,
    },
}

/// Summary counts for a run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunSummary {
    /// Total resources processed.
    pub total: u32,
    /// Resources that converged (created or updated successfully).
    pub converged: u32,
    /// Resources already at desired state.
    pub noop: u32,
    /// Resources that failed.
    pub failed: u32,
    /// Resources skipped due to dependency failures.
    pub skipped: u32,
}

/// FJ-2301: Structured log entry for a single script execution.
///
/// Each `.log` file contains a header (resource, type, action, timestamps)
/// plus delimited sections for SCRIPT, STDOUT, STDERR, and RESULT.
///
/// # Examples
///
/// ```
/// use forjar::core::types::RunLogEntry;
///
/// let entry = RunLogEntry {
///     resource_id: "nginx-pkg".into(),
///     resource_type: "package".into(),
///     action: "apply".into(),
///     machine: "web-1".into(),
///     transport: "ssh".into(),
///     script: "apt-get install -y nginx".into(),
///     script_hash: "blake3:abc123".into(),
///     stdout: "Reading package lists...".into(),
///     stderr: String::new(),
///     exit_code: 0,
///     duration_secs: 1.2,
///     started_at: "2026-03-05T14:30:00Z".into(),
///     finished_at: "2026-03-05T14:30:01Z".into(),
/// };
/// let formatted = entry.format_log();
/// assert!(formatted.contains("=== FORJAR TRANSPORT LOG ==="));
/// assert!(formatted.contains("=== STDOUT ==="));
/// assert!(formatted.contains("exit_code: 0"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunLogEntry {
    /// Resource identifier.
    pub resource_id: String,
    /// Resource type (package, file, service, etc.).
    pub resource_type: String,
    /// Action performed (check, apply, destroy).
    pub action: String,
    /// Target machine name.
    pub machine: String,
    /// Transport used (ssh, local, pepita, container).
    pub transport: String,
    /// The script that was executed.
    pub script: String,
    /// BLAKE3 hash of the script.
    pub script_hash: String,
    /// Full stdout output.
    pub stdout: String,
    /// Full stderr output.
    pub stderr: String,
    /// Exit code.
    pub exit_code: i32,
    /// Duration in seconds.
    pub duration_secs: f64,
    /// ISO 8601 start timestamp.
    pub started_at: String,
    /// ISO 8601 finish timestamp.
    pub finished_at: String,
}

impl RunLogEntry {
    /// Format as a structured log file with delimited sections.
    pub fn format_log(&self) -> String {
        let mut out = String::with_capacity(
            self.script.len() + self.stdout.len() + self.stderr.len() + 512,
        );

        out.push_str("=== FORJAR TRANSPORT LOG ===\n");
        out.push_str(&format!("resource: {}\n", self.resource_id));
        out.push_str(&format!("type: {}\n", self.resource_type));
        out.push_str(&format!("action: {}\n", self.action));
        out.push_str(&format!("machine: {}\n", self.machine));
        out.push_str(&format!("transport: {}\n", self.transport));
        out.push_str(&format!("started: {}\n", self.started_at));
        out.push_str(&format!("script_hash: {}\n", self.script_hash));

        out.push_str("\n=== SCRIPT ===\n");
        out.push_str(&self.script);
        if !self.script.ends_with('\n') {
            out.push('\n');
        }

        out.push_str("\n=== STDOUT ===\n");
        out.push_str(&self.stdout);
        if !self.stdout.is_empty() && !self.stdout.ends_with('\n') {
            out.push('\n');
        }

        out.push_str("\n=== STDERR ===\n");
        out.push_str(&self.stderr);
        if !self.stderr.is_empty() && !self.stderr.ends_with('\n') {
            out.push('\n');
        }

        out.push_str("\n=== RESULT ===\n");
        out.push_str(&format!("exit_code: {}\n", self.exit_code));
        out.push_str(&format!("duration_secs: {:.3}\n", self.duration_secs));
        out.push_str(&format!("finished: {}\n", self.finished_at));

        out
    }
}

impl fmt::Display for RunLogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_log())
    }
}

/// FJ-2301: Log retention policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRetention {
    /// Keep last N runs per machine (default: 10).
    #[serde(default = "default_keep_runs")]
    pub keep_runs: u32,
    /// Keep last N failed runs regardless (default: 50).
    #[serde(default = "default_keep_failed")]
    pub keep_failed: u32,
    /// Maximum single log file size in bytes (default: 10MB).
    #[serde(default = "default_max_log_size")]
    pub max_log_size: u64,
    /// Total log storage budget per machine in bytes (default: 500MB).
    #[serde(default = "default_max_total_size")]
    pub max_total_size: u64,
}

impl Default for LogRetention {
    fn default() -> Self {
        Self {
            keep_runs: default_keep_runs(),
            keep_failed: default_keep_failed(),
            max_log_size: default_max_log_size(),
            max_total_size: default_max_total_size(),
        }
    }
}

fn default_keep_runs() -> u32 {
    10
}
fn default_keep_failed() -> u32 {
    50
}
fn default_max_log_size() -> u64 {
    10 * 1024 * 1024
}
fn default_max_total_size() -> u64 {
    500 * 1024 * 1024
}

/// FJ-2301: Generate a run ID from the current timestamp.
pub fn generate_run_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("r-{:012x}", nanos & 0xFFFF_FFFF_FFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_meta_new() {
        let meta = RunMeta::new("r-123".into(), "web".into(), "apply".into());
        assert_eq!(meta.run_id, "r-123");
        assert_eq!(meta.machine, "web");
        assert_eq!(meta.command, "apply");
        assert!(meta.resources.is_empty());
        assert_eq!(meta.summary.total, 0);
    }

    #[test]
    fn run_meta_record_resources() {
        let mut meta = RunMeta::new("r-1".into(), "m".into(), "apply".into());
        meta.record_resource("a", ResourceRunStatus::Noop);
        meta.record_resource("b", ResourceRunStatus::Converged {
            exit_code: Some(0),
            duration_secs: Some(1.5),
            failed: false,
        });
        meta.record_resource("c", ResourceRunStatus::Converged {
            exit_code: Some(1),
            duration_secs: Some(0.3),
            failed: true,
        });
        meta.record_resource("d", ResourceRunStatus::Skipped {
            reason: Some("dep failed".into()),
        });

        assert_eq!(meta.summary.total, 4);
        assert_eq!(meta.summary.noop, 1);
        assert_eq!(meta.summary.converged, 1);
        assert_eq!(meta.summary.failed, 1);
        assert_eq!(meta.summary.skipped, 1);
        assert_eq!(meta.resources.len(), 4);
    }

    #[test]
    fn run_meta_serde_roundtrip() {
        let mut meta = RunMeta::new("r-abc".into(), "intel".into(), "destroy".into());
        meta.generation = Some(5);
        meta.operator = Some("noah@host".into());
        meta.record_resource("pkg", ResourceRunStatus::Noop);
        let yaml = serde_yaml_ng::to_string(&meta).unwrap();
        let parsed: RunMeta = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(parsed.run_id, "r-abc");
        assert_eq!(parsed.generation, Some(5));
        assert_eq!(parsed.summary.noop, 1);
    }

    #[test]
    fn run_log_entry_format() {
        let entry = RunLogEntry {
            resource_id: "nginx".into(),
            resource_type: "package".into(),
            action: "apply".into(),
            machine: "web-1".into(),
            transport: "ssh".into(),
            script: "apt-get install -y nginx".into(),
            script_hash: "blake3:deadbeef".into(),
            stdout: "installed ok".into(),
            stderr: String::new(),
            exit_code: 0,
            duration_secs: 1.234,
            started_at: "2026-03-05T14:30:00Z".into(),
            finished_at: "2026-03-05T14:30:01Z".into(),
        };
        let log = entry.format_log();
        assert!(log.contains("=== FORJAR TRANSPORT LOG ==="));
        assert!(log.contains("resource: nginx"));
        assert!(log.contains("=== SCRIPT ==="));
        assert!(log.contains("apt-get install -y nginx"));
        assert!(log.contains("=== STDOUT ==="));
        assert!(log.contains("installed ok"));
        assert!(log.contains("=== STDERR ==="));
        assert!(log.contains("=== RESULT ==="));
        assert!(log.contains("exit_code: 0"));
        assert!(log.contains("duration_secs: 1.234"));
    }

    #[test]
    fn run_log_entry_display() {
        let entry = RunLogEntry {
            resource_id: "pkg".into(),
            resource_type: "package".into(),
            action: "check".into(),
            machine: "m".into(),
            transport: "local".into(),
            script: "echo test".into(),
            script_hash: "blake3:abc".into(),
            stdout: "test\n".into(),
            stderr: "warn\n".into(),
            exit_code: 1,
            duration_secs: 0.5,
            started_at: "t0".into(),
            finished_at: "t1".into(),
        };
        let display = format!("{entry}");
        assert!(display.contains("exit_code: 1"));
    }

    #[test]
    fn run_log_serde_roundtrip() {
        let entry = RunLogEntry {
            resource_id: "svc".into(),
            resource_type: "service".into(),
            action: "destroy".into(),
            machine: "host".into(),
            transport: "ssh".into(),
            script: "systemctl stop svc".into(),
            script_hash: "blake3:fff".into(),
            stdout: "stopped".into(),
            stderr: String::new(),
            exit_code: 0,
            duration_secs: 0.1,
            started_at: "s".into(),
            finished_at: "f".into(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: RunLogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.resource_id, "svc");
        assert_eq!(parsed.exit_code, 0);
    }

    #[test]
    fn log_retention_defaults() {
        let ret = LogRetention::default();
        assert_eq!(ret.keep_runs, 10);
        assert_eq!(ret.keep_failed, 50);
        assert_eq!(ret.max_log_size, 10 * 1024 * 1024);
        assert_eq!(ret.max_total_size, 500 * 1024 * 1024);
    }

    #[test]
    fn log_retention_serde() {
        let yaml = r#"
keep_runs: 5
keep_failed: 20
max_log_size: 1048576
max_total_size: 104857600
"#;
        let ret: LogRetention = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(ret.keep_runs, 5);
        assert_eq!(ret.keep_failed, 20);
    }

    #[test]
    fn generate_run_id_format() {
        let id = generate_run_id();
        assert!(id.starts_with("r-"));
        assert!(id.len() >= 3);
    }

    #[test]
    fn generate_run_id_unique() {
        let a = generate_run_id();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let b = generate_run_id();
        assert_ne!(a, b);
    }

    #[test]
    fn resource_run_status_variants() {
        let noop = ResourceRunStatus::Noop;
        let yaml = serde_yaml_ng::to_string(&noop).unwrap();
        assert!(yaml.contains("noop"));

        let converged = ResourceRunStatus::Converged {
            exit_code: Some(0),
            duration_secs: Some(2.5),
            failed: false,
        };
        let yaml = serde_yaml_ng::to_string(&converged).unwrap();
        assert!(yaml.contains("converged"));

        let skipped = ResourceRunStatus::Skipped {
            reason: Some("dep failed".into()),
        };
        let yaml = serde_yaml_ng::to_string(&skipped).unwrap();
        assert!(yaml.contains("skipped"));
    }

    #[test]
    fn run_summary_default() {
        let summary = RunSummary::default();
        assert_eq!(summary.total, 0);
        assert_eq!(summary.converged, 0);
        assert_eq!(summary.noop, 0);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.skipped, 0);
    }

    #[test]
    fn run_log_empty_stdout_stderr() {
        let entry = RunLogEntry {
            resource_id: "r".into(),
            resource_type: "task".into(),
            action: "apply".into(),
            machine: "m".into(),
            transport: "local".into(),
            script: "true".into(),
            script_hash: "blake3:0".into(),
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
            duration_secs: 0.001,
            started_at: "s".into(),
            finished_at: "f".into(),
        };
        let log = entry.format_log();
        assert!(log.contains("=== STDOUT ===\n\n=== STDERR ==="));
    }
}
