//! FJ-2301: Observability types — log filtering, truncation, progress, verbosity.

use serde::{Deserialize, Serialize};
use std::fmt;

/// FJ-2301: Verbosity level for terminal output.
///
/// # Examples
///
/// ```
/// use forjar::core::types::VerbosityLevel;
///
/// let v = VerbosityLevel::from_count(3);
/// assert_eq!(v, VerbosityLevel::Trace);
/// assert!(v.streams_raw());
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerbosityLevel {
    /// Normal output — summary only.
    #[default]
    Normal,
    /// `-v`: Show per-resource status.
    Verbose,
    /// `-vv`: Show script content and exit codes.
    VeryVerbose,
    /// `-vvv`: Stream raw stdout/stderr to terminal in real-time.
    Trace,
}

impl VerbosityLevel {
    /// Create from CLI `-v` flag count.
    pub fn from_count(count: u8) -> Self {
        match count {
            0 => Self::Normal,
            1 => Self::Verbose,
            2 => Self::VeryVerbose,
            _ => Self::Trace,
        }
    }

    /// Whether this level streams raw output (>= `-vvv`).
    pub fn streams_raw(&self) -> bool {
        *self >= Self::Trace
    }

    /// Whether this level shows script content (>= `-vv`).
    pub fn shows_scripts(&self) -> bool {
        *self >= Self::VeryVerbose
    }
}

impl fmt::Display for VerbosityLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => write!(f, "normal"),
            Self::Verbose => write!(f, "verbose"),
            Self::VeryVerbose => write!(f, "very-verbose"),
            Self::Trace => write!(f, "trace"),
        }
    }
}

/// FJ-2301: Log filter parameters for `forjar logs`.
///
/// # Examples
///
/// ```
/// use forjar::core::types::LogFilter;
///
/// let filter = LogFilter::for_machine("intel");
/// assert_eq!(filter.machine.as_deref(), Some("intel"));
/// assert!(!filter.failures_only);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogFilter {
    /// Filter by machine name.
    #[serde(default)]
    pub machine: Option<String>,
    /// Filter by run ID.
    #[serde(default)]
    pub run_id: Option<String>,
    /// Filter by resource ID.
    #[serde(default)]
    pub resource: Option<String>,
    /// Show only failed actions.
    #[serde(default)]
    pub failures_only: bool,
    /// Follow mode (live streaming during apply).
    #[serde(default)]
    pub follow: bool,
    /// Maximum number of log entries to show.
    #[serde(default)]
    pub limit: Option<u32>,
    /// Time range filter (e.g., "7d", "24h").
    #[serde(default)]
    pub since: Option<String>,
}

impl LogFilter {
    /// Create a filter for a specific machine.
    pub fn for_machine(machine: &str) -> Self {
        Self {
            machine: Some(machine.to_string()),
            ..Default::default()
        }
    }

    /// Create a filter for a specific run.
    pub fn for_run(run_id: &str) -> Self {
        Self {
            run_id: Some(run_id.to_string()),
            ..Default::default()
        }
    }

    /// Create a filter for failed actions only.
    pub fn failures() -> Self {
        Self {
            failures_only: true,
            ..Default::default()
        }
    }

    /// Whether any filter criteria are set.
    pub fn has_criteria(&self) -> bool {
        self.machine.is_some()
            || self.run_id.is_some()
            || self.resource.is_some()
            || self.failures_only
            || self.since.is_some()
    }
}

/// FJ-2301: Log truncation — keep first N + last N bytes for oversized logs.
///
/// # Examples
///
/// ```
/// use forjar::core::types::LogTruncation;
/// let t = LogTruncation::default();
/// assert!(t.should_truncate(20000));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogTruncation {
    /// Keep first N bytes (default: 8KB).
    pub first_bytes: usize,
    /// Keep last N bytes (default: 8KB).
    pub last_bytes: usize,
}

impl Default for LogTruncation {
    fn default() -> Self {
        Self {
            first_bytes: 8192,
            last_bytes: 8192,
        }
    }
}

impl LogTruncation {
    /// Whether a log of the given size should be truncated.
    pub fn should_truncate(&self, size: usize) -> bool {
        size > self.first_bytes + self.last_bytes
    }

    /// Truncate a log string, preserving first + last bytes with marker.
    pub fn truncate(&self, log: &str) -> String {
        if !self.should_truncate(log.len()) {
            return log.to_string();
        }
        let omitted = log.len() - self.first_bytes - self.last_bytes;
        let first = &log[..self.first_bytes];
        let last = &log[log.len() - self.last_bytes..];
        format!("{first}\n\n--- TRUNCATED ({omitted} bytes omitted) ---\n\n{last}")
    }
}

/// FJ-2301: Result of `forjar logs --gc`.
///
/// # Examples
///
/// ```
/// use forjar::core::types::LogGcResult;
///
/// let gc = LogGcResult {
///     runs_removed: 5,
///     bytes_freed: 1024 * 1024 * 50,
///     runs_kept: 10,
/// };
/// assert_eq!(gc.mb_freed(), 50.0);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogGcResult {
    /// Number of run directories removed.
    pub runs_removed: u32,
    /// Total bytes freed.
    pub bytes_freed: u64,
    /// Number of run directories kept.
    pub runs_kept: u32,
}

impl LogGcResult {
    /// Megabytes freed (rounded to 1 decimal).
    pub fn mb_freed(&self) -> f64 {
        self.bytes_freed as f64 / (1024.0 * 1024.0)
    }
}

impl fmt::Display for LogGcResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GC: removed {} runs ({:.1} MB freed), {} runs kept",
            self.runs_removed,
            self.mb_freed(),
            self.runs_kept,
        )
    }
}

/// FJ-2301: Structured JSON output with `log_path` for CI artifact upload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredLogOutput {
    /// Run identifier.
    pub run_id: String,
    /// Machine name.
    pub machine: String,
    /// Resource identifier.
    pub resource_id: String,
    /// Path to the log file on disk.
    pub log_path: String,
    /// Exit code of the logged action.
    pub exit_code: i32,
    /// Duration in seconds.
    pub duration_secs: f64,
    /// Whether the log was truncated.
    #[serde(default)]
    pub truncated: bool,
}

/// FJ-2301: Progress reporting configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressConfig {
    /// Show progress bars on stderr (default: true for TTY).
    pub show_progress: bool,
    /// Update interval in milliseconds.
    pub update_interval_ms: u32,
}

impl Default for ProgressConfig {
    fn default() -> Self {
        Self {
            show_progress: true,
            update_interval_ms: 100,
        }
    }
}

/// FJ-2301: Run log directory path builder.
///
/// Generates paths like `state/<machine>/runs/<run_id>/`.
///
/// # Examples
///
/// ```
/// use forjar::core::types::RunLogPath;
///
/// let path = RunLogPath::new("state", "intel", "r-abc123");
/// assert_eq!(path.run_dir(), "state/intel/runs/r-abc123");
/// assert_eq!(
///     path.resource_log("nginx-pkg", "apply"),
///     "state/intel/runs/r-abc123/nginx-pkg.apply.log"
/// );
/// assert_eq!(path.meta_path(), "state/intel/runs/r-abc123/meta.yaml");
/// ```
#[derive(Debug, Clone)]
pub struct RunLogPath {
    state_dir: String,
    machine: String,
    run_id: String,
}

impl RunLogPath {
    /// Create a new path builder.
    pub fn new(state_dir: &str, machine: &str, run_id: &str) -> Self {
        Self {
            state_dir: state_dir.to_string(),
            machine: machine.to_string(),
            run_id: run_id.to_string(),
        }
    }

    /// Path to the run directory.
    pub fn run_dir(&self) -> String {
        format!("{}/{}/runs/{}", self.state_dir, self.machine, self.run_id)
    }

    /// Path to a resource log file.
    pub fn resource_log(&self, resource_id: &str, action: &str) -> String {
        format!("{}/{}.{action}.log", self.run_dir(), resource_id)
    }

    /// Path to the run metadata file.
    pub fn meta_path(&self) -> String {
        format!("{}/meta.yaml", self.run_dir())
    }

    /// Path to the runs directory for this machine.
    pub fn runs_dir(&self) -> String {
        format!("{}/{}/runs", self.state_dir, self.machine)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbosity_from_count() {
        assert_eq!(VerbosityLevel::from_count(0), VerbosityLevel::Normal);
        assert_eq!(VerbosityLevel::from_count(1), VerbosityLevel::Verbose);
        assert_eq!(VerbosityLevel::from_count(2), VerbosityLevel::VeryVerbose);
        assert_eq!(VerbosityLevel::from_count(3), VerbosityLevel::Trace);
        assert_eq!(VerbosityLevel::from_count(10), VerbosityLevel::Trace);
    }

    #[test]
    fn verbosity_streams_raw() {
        assert!(!VerbosityLevel::Normal.streams_raw());
        assert!(!VerbosityLevel::Verbose.streams_raw());
        assert!(!VerbosityLevel::VeryVerbose.streams_raw());
        assert!(VerbosityLevel::Trace.streams_raw());
    }

    #[test]
    fn verbosity_shows_scripts() {
        assert!(!VerbosityLevel::Normal.shows_scripts());
        assert!(!VerbosityLevel::Verbose.shows_scripts());
        assert!(VerbosityLevel::VeryVerbose.shows_scripts());
        assert!(VerbosityLevel::Trace.shows_scripts());
    }

    #[test]
    fn verbosity_ordering() {
        assert!(VerbosityLevel::Normal < VerbosityLevel::Verbose);
        assert!(VerbosityLevel::Verbose < VerbosityLevel::VeryVerbose);
        assert!(VerbosityLevel::VeryVerbose < VerbosityLevel::Trace);
    }

    #[test]
    fn verbosity_display() {
        assert_eq!(VerbosityLevel::Normal.to_string(), "normal");
        assert_eq!(VerbosityLevel::Trace.to_string(), "trace");
    }

    #[test]
    fn verbosity_serde_roundtrip() {
        let v = VerbosityLevel::VeryVerbose;
        let json = serde_json::to_string(&v).unwrap();
        let parsed: VerbosityLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(v, parsed);
    }

    #[test]
    fn log_filter_for_machine() {
        let f = LogFilter::for_machine("intel");
        assert_eq!(f.machine.as_deref(), Some("intel"));
        assert!(f.has_criteria());
    }

    #[test]
    fn log_filter_for_run() {
        let f = LogFilter::for_run("r-123");
        assert_eq!(f.run_id.as_deref(), Some("r-123"));
        assert!(f.has_criteria());
    }

    #[test]
    fn log_filter_failures() {
        let f = LogFilter::failures();
        assert!(f.failures_only);
        assert!(f.has_criteria());
    }

    #[test]
    fn log_filter_default_no_criteria() {
        let f = LogFilter::default();
        assert!(!f.has_criteria());
    }

    #[test]
    fn log_truncation_default() {
        let t = LogTruncation::default();
        assert_eq!(t.first_bytes, 8192);
        assert_eq!(t.last_bytes, 8192);
    }

    #[test]
    fn log_truncation_should_truncate() {
        let t = LogTruncation::default();
        assert!(!t.should_truncate(100));
        assert!(!t.should_truncate(16384));
        assert!(t.should_truncate(16385));
    }

    #[test]
    fn log_truncation_truncate_small() {
        let t = LogTruncation {
            first_bytes: 5,
            last_bytes: 5,
        };
        assert_eq!(t.truncate("short"), "short");
    }

    #[test]
    fn log_truncation_truncate_large() {
        let t = LogTruncation {
            first_bytes: 5,
            last_bytes: 5,
        };
        let input = "ABCDE__middle__FGHIJ";
        let result = t.truncate(input);
        assert!(result.starts_with("ABCDE"));
        assert!(result.ends_with("FGHIJ"));
        assert!(result.contains("TRUNCATED"));
        assert!(result.contains("10 bytes omitted"));
    }

    #[test]
    fn log_gc_result_mb_freed() {
        let gc = LogGcResult {
            runs_removed: 3,
            bytes_freed: 10 * 1024 * 1024,
            runs_kept: 7,
        };
        assert!((gc.mb_freed() - 10.0).abs() < 0.01);
    }

    #[test]
    fn log_gc_result_display() {
        let gc = LogGcResult {
            runs_removed: 2,
            bytes_freed: 5 * 1024 * 1024,
            runs_kept: 8,
        };
        let s = gc.to_string();
        assert!(s.contains("removed 2 runs"));
        assert!(s.contains("5.0 MB"));
        assert!(s.contains("8 runs kept"));
    }

    #[test]
    fn structured_log_output_serde() {
        let out = StructuredLogOutput {
            run_id: "r-1".into(),
            machine: "m".into(),
            resource_id: "pkg".into(),
            log_path: "state/m/runs/r-1/pkg.apply.log".into(),
            exit_code: 0,
            duration_secs: 1.0,
            truncated: false,
        };
        let json = serde_json::to_string(&out).unwrap();
        let parsed: StructuredLogOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.run_id, "r-1");
        assert!(!parsed.truncated);
    }

    #[test]
    fn progress_config_default() {
        let pc = ProgressConfig::default();
        assert!(pc.show_progress);
        assert_eq!(pc.update_interval_ms, 100);
    }

    #[test]
    fn run_log_path_builder() {
        let p = RunLogPath::new("state", "intel", "r-abc");
        assert_eq!(p.run_dir(), "state/intel/runs/r-abc");
        assert_eq!(
            p.resource_log("nginx", "apply"),
            "state/intel/runs/r-abc/nginx.apply.log"
        );
        assert_eq!(p.meta_path(), "state/intel/runs/r-abc/meta.yaml");
        assert_eq!(p.runs_dir(), "state/intel/runs");
    }

    #[test]
    fn run_log_path_check_action() {
        let p = RunLogPath::new("s", "m", "r-1");
        assert_eq!(p.resource_log("pkg", "check"), "s/m/runs/r-1/pkg.check.log");
        assert_eq!(
            p.resource_log("svc", "destroy"),
            "s/m/runs/r-1/svc.destroy.log"
        );
    }
}
