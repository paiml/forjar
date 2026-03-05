//! FJ-2700: Task framework types — modes, pipeline stages, quality gates.

use serde::{Deserialize, Serialize};

/// FJ-2700: Task execution mode.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskMode {
    /// Run-once task that converges to "completed" state (default).
    #[default]
    Batch,
    /// Ordered multi-stage execution with inter-stage gates.
    Pipeline,
    /// Long-running process with health checks and restart policy.
    Service,
    /// Parameterized task triggered on-demand via `forjar run`.
    Dispatch,
}

impl std::fmt::Display for TaskMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Batch => write!(f, "batch"),
            Self::Pipeline => write!(f, "pipeline"),
            Self::Service => write!(f, "service"),
            Self::Dispatch => write!(f, "dispatch"),
        }
    }
}

/// FJ-2700: A single stage in a pipeline task.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineStage {
    /// Stage name (used in state tracking and output).
    pub name: String,
    /// Shell command to execute.
    #[serde(default)]
    pub command: Option<String>,
    /// Input file paths (checked for cache validity).
    #[serde(default)]
    pub inputs: Vec<String>,
    /// Output file paths (hashed for caching and drift).
    #[serde(default)]
    pub outputs: Vec<String>,
    /// If true, pipeline stops when this stage fails.
    #[serde(default)]
    pub gate: bool,
}

/// FJ-2702: Quality gate configuration for a task.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QualityGate {
    /// Gate command (exit 0 = pass).
    #[serde(default)]
    pub command: Option<String>,
    /// Parse mode: "json", "stdout".
    #[serde(default)]
    pub parse: Option<String>,
    /// JSON field to check.
    #[serde(default)]
    pub field: Option<String>,
    /// Allowed values for the field.
    #[serde(default)]
    pub threshold: Vec<String>,
    /// Minimum numeric value.
    #[serde(default)]
    pub min: Option<f64>,
    /// Regex pattern for stdout matching.
    #[serde(default)]
    pub regex: Option<String>,
    /// Action on failure: block (default), warn, skip_dependents.
    #[serde(default)]
    pub on_fail: Option<String>,
    /// Human-readable failure message.
    #[serde(default)]
    pub message: Option<String>,
}

/// FJ-2700: Health check configuration for service-mode tasks.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Shell command to check health (exit 0 = healthy).
    pub command: String,
    /// Check interval (e.g., "30s", "5m").
    #[serde(default)]
    pub interval: Option<String>,
    /// Check timeout (e.g., "5s").
    #[serde(default)]
    pub timeout: Option<String>,
    /// Number of consecutive failures before restart.
    #[serde(default)]
    pub retries: Option<u32>,
}

// ── FJ-2706: Task state model ──

/// Per-stage execution state in a pipeline.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StageStatus {
    #[default]
    Pending,
    Running,
    Passed,
    Failed,
    Skipped,
}

/// FJ-2706: Pipeline state — per-stage tracking stored in lock file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineState {
    /// Per-stage status (stage_name → status).
    pub stages: Vec<StageState>,
    /// Overall pipeline status.
    pub status: StageStatus,
    /// Last completed stage index (0-based).
    #[serde(default)]
    pub last_completed: Option<usize>,
}

/// State of a single pipeline stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageState {
    pub name: String,
    pub status: StageStatus,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    /// BLAKE3 hash of inputs (for cache invalidation).
    #[serde(default)]
    pub input_hash: Option<String>,
}

/// FJ-2706: Service state — PID and health check history.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServiceState {
    /// Process ID (if running).
    #[serde(default)]
    pub pid: Option<u32>,
    /// Whether the service is currently healthy.
    #[serde(default)]
    pub healthy: bool,
    /// Number of consecutive health check failures.
    #[serde(default)]
    pub consecutive_failures: u32,
    /// Last health check timestamp.
    #[serde(default)]
    pub last_check: Option<String>,
    /// Number of restarts since initial start.
    #[serde(default)]
    pub restart_count: u32,
}

/// FJ-2706: Dispatch invocation record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchInvocation {
    /// When the dispatch was triggered.
    pub timestamp: String,
    /// Exit code of the dispatched command.
    pub exit_code: i32,
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// Caller identifier (user/CI/trigger).
    #[serde(default)]
    pub caller: Option<String>,
}

/// FJ-2706: Dispatch state — invocation history.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DispatchState {
    /// Recent invocation history (most recent first).
    pub invocations: Vec<DispatchInvocation>,
    /// Total invocation count.
    pub total_invocations: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_mode_serde_roundtrip() {
        for mode in [TaskMode::Batch, TaskMode::Pipeline, TaskMode::Service, TaskMode::Dispatch] {
            let yaml = serde_yaml_ng::to_string(&mode).unwrap();
            let parsed: TaskMode = serde_yaml_ng::from_str(&yaml).unwrap();
            assert_eq!(mode, parsed);
        }
    }

    #[test]
    fn task_mode_default_is_batch() {
        assert_eq!(TaskMode::default(), TaskMode::Batch);
    }

    #[test]
    fn pipeline_stage_serde() {
        let yaml = r#"
name: build
command: "cargo build --release"
inputs: ["src/**/*.rs"]
outputs: ["target/release/app"]
gate: true
"#;
        let stage: PipelineStage = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(stage.name, "build");
        assert!(stage.gate);
        assert_eq!(stage.outputs.len(), 1);
    }

    #[test]
    fn quality_gate_serde() {
        let yaml = r#"
parse: json
field: grade
threshold: ["A", "B"]
on_fail: block
"#;
        let gate: QualityGate = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(gate.field.as_deref(), Some("grade"));
        assert_eq!(gate.threshold.len(), 2);
    }

    #[test]
    fn health_check_serde() {
        let yaml = r#"
command: "curl -sf http://localhost:8080/health"
interval: "30s"
timeout: "5s"
retries: 3
"#;
        let hc: HealthCheck = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(hc.retries, Some(3));
    }

    #[test]
    fn task_mode_display() {
        assert_eq!(TaskMode::Batch.to_string(), "batch");
        assert_eq!(TaskMode::Pipeline.to_string(), "pipeline");
        assert_eq!(TaskMode::Service.to_string(), "service");
        assert_eq!(TaskMode::Dispatch.to_string(), "dispatch");
    }

    #[test]
    fn pipeline_state_serde() {
        let state = PipelineState {
            stages: vec![
                StageState { name: "lint".into(), status: StageStatus::Passed, exit_code: Some(0), duration_ms: Some(1200), input_hash: None },
                StageState { name: "test".into(), status: StageStatus::Failed, exit_code: Some(1), duration_ms: Some(5000), input_hash: None },
            ],
            status: StageStatus::Failed,
            last_completed: Some(0),
        };
        let yaml = serde_yaml_ng::to_string(&state).unwrap();
        let parsed: PipelineState = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(parsed.stages.len(), 2);
        assert_eq!(parsed.status, StageStatus::Failed);
        assert_eq!(parsed.last_completed, Some(0));
    }

    #[test]
    fn service_state_defaults() {
        let state = ServiceState::default();
        assert!(!state.healthy);
        assert_eq!(state.restart_count, 0);
        assert!(state.pid.is_none());
    }

    #[test]
    fn dispatch_state_serde() {
        let state = DispatchState {
            invocations: vec![DispatchInvocation {
                timestamp: "2026-03-05T12:00:00Z".into(),
                exit_code: 0,
                duration_ms: 350,
                caller: Some("ci".into()),
            }],
            total_invocations: 1,
        };
        let yaml = serde_yaml_ng::to_string(&state).unwrap();
        let parsed: DispatchState = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(parsed.total_invocations, 1);
        assert_eq!(parsed.invocations[0].exit_code, 0);
    }

    #[test]
    fn stage_status_default_is_pending() {
        assert_eq!(StageStatus::default(), StageStatus::Pending);
    }
}
