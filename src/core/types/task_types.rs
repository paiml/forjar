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

/// FJ-2703: Multi-GPU parallel scheduling for tasks in the same wave.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GpuSchedule {
    /// GPU device assignments: task_id → device indices.
    pub assignments: std::collections::HashMap<String, Vec<u32>>,
    /// Total available GPU devices.
    pub total_devices: u32,
}

impl GpuSchedule {
    /// Create a schedule for N devices.
    pub fn new(total_devices: u32) -> Self {
        Self {
            assignments: std::collections::HashMap::new(),
            total_devices,
        }
    }

    /// Assign a task to specific GPU devices.
    pub fn assign(&mut self, task_id: &str, devices: Vec<u32>) {
        self.assignments.insert(task_id.to_string(), devices);
    }

    /// Get the `CUDA_VISIBLE_DEVICES` value for a task.
    pub fn cuda_visible_devices(&self, task_id: &str) -> Option<String> {
        self.assignments.get(task_id).map(|devs| {
            devs.iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join(",")
        })
    }

    /// Number of devices currently assigned.
    pub fn assigned_device_count(&self) -> usize {
        let mut seen = std::collections::HashSet::new();
        for devs in self.assignments.values() {
            for &d in devs {
                seen.insert(d);
            }
        }
        seen.len()
    }

    /// Whether all devices are assigned.
    pub fn fully_utilized(&self) -> bool {
        self.assigned_device_count() >= self.total_devices as usize
    }

    /// Round-robin assignment across available devices.
    pub fn round_robin(tasks: &[&str], total_devices: u32) -> Self {
        let mut schedule = Self::new(total_devices);
        for (i, task) in tasks.iter().enumerate() {
            let device = (i as u32) % total_devices;
            schedule.assign(task, vec![device]);
        }
        schedule
    }
}

/// FJ-2704: Barrier task for multi-machine synchronization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarrierTask {
    /// Barrier task identifier.
    pub task_id: String,
    /// Machine names that must complete before barrier releases.
    pub wait_for_machines: Vec<String>,
    /// Optional timeout in seconds (0 = no timeout).
    #[serde(default)]
    pub timeout_secs: u64,
    /// Machines that have reported completion.
    #[serde(default)]
    pub completed: Vec<String>,
}

impl BarrierTask {
    /// Create a new barrier waiting for the given machines.
    pub fn new(task_id: &str, machines: Vec<String>) -> Self {
        Self {
            task_id: task_id.to_string(),
            wait_for_machines: machines,
            timeout_secs: 0,
            completed: Vec::new(),
        }
    }

    /// Mark a machine as completed.
    pub fn mark_complete(&mut self, machine: &str) {
        if !self.completed.contains(&machine.to_string()) {
            self.completed.push(machine.to_string());
        }
    }

    /// Whether the barrier is satisfied (all machines completed).
    pub fn is_satisfied(&self) -> bool {
        self.wait_for_machines
            .iter()
            .all(|m| self.completed.contains(m))
    }

    /// Machines still pending.
    pub fn pending_machines(&self) -> Vec<&str> {
        self.wait_for_machines
            .iter()
            .filter(|m| !self.completed.contains(m))
            .map(|m| m.as_str())
            .collect()
    }

    /// Completion percentage.
    pub fn progress_pct(&self) -> f64 {
        if self.wait_for_machines.is_empty() {
            return 100.0;
        }
        (self.completed.len() as f64 / self.wait_for_machines.len() as f64) * 100.0
    }
}

impl std::fmt::Display for BarrierTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pending = self.pending_machines();
        if pending.is_empty() {
            write!(f, "barrier/{}: SATISFIED", self.task_id)
        } else {
            write!(
                f,
                "barrier/{}: waiting for {} ({:.0}%)",
                self.task_id,
                pending.join(", "),
                self.progress_pct()
            )
        }
    }
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

    #[test]
    fn gpu_schedule_round_robin() {
        let schedule = GpuSchedule::round_robin(&["train-a", "train-b", "train-c"], 2);
        assert_eq!(schedule.cuda_visible_devices("train-a"), Some("0".into()));
        assert_eq!(schedule.cuda_visible_devices("train-b"), Some("1".into()));
        assert_eq!(schedule.cuda_visible_devices("train-c"), Some("0".into()));
        assert!(schedule.fully_utilized());
    }

    #[test]
    fn gpu_schedule_assign() {
        let mut schedule = GpuSchedule::new(4);
        schedule.assign("big-model", vec![0, 1, 2, 3]);
        assert_eq!(schedule.cuda_visible_devices("big-model"), Some("0,1,2,3".into()));
        assert!(schedule.fully_utilized());
        assert_eq!(schedule.assigned_device_count(), 4);
    }

    #[test]
    fn gpu_schedule_partial() {
        let mut schedule = GpuSchedule::new(4);
        schedule.assign("small", vec![0]);
        assert!(!schedule.fully_utilized());
        assert_eq!(schedule.assigned_device_count(), 1);
    }

    #[test]
    fn gpu_schedule_no_task() {
        let schedule = GpuSchedule::new(2);
        assert_eq!(schedule.cuda_visible_devices("missing"), None);
    }

    #[test]
    fn barrier_task_lifecycle() {
        let mut barrier = BarrierTask::new("sync-all", vec!["intel".into(), "jetson".into(), "lambda".into()]);
        assert!(!barrier.is_satisfied());
        assert_eq!(barrier.pending_machines().len(), 3);
        assert!((barrier.progress_pct() - 0.0).abs() < 0.01);

        barrier.mark_complete("intel");
        assert!(!barrier.is_satisfied());
        assert_eq!(barrier.pending_machines(), vec!["jetson", "lambda"]);
        assert!((barrier.progress_pct() - 33.3).abs() < 0.5);

        barrier.mark_complete("jetson");
        barrier.mark_complete("lambda");
        assert!(barrier.is_satisfied());
        assert!(barrier.pending_machines().is_empty());
        assert!((barrier.progress_pct() - 100.0).abs() < 0.01);
    }

    #[test]
    fn barrier_task_display() {
        let mut barrier = BarrierTask::new("sync", vec!["a".into(), "b".into()]);
        assert!(barrier.to_string().contains("waiting for"));
        barrier.mark_complete("a");
        barrier.mark_complete("b");
        assert!(barrier.to_string().contains("SATISFIED"));
    }

    #[test]
    fn barrier_task_duplicate_complete() {
        let mut barrier = BarrierTask::new("sync", vec!["a".into()]);
        barrier.mark_complete("a");
        barrier.mark_complete("a"); // duplicate — should not add twice
        assert_eq!(barrier.completed.len(), 1);
    }

    #[test]
    fn barrier_task_empty() {
        let barrier = BarrierTask::new("noop", vec![]);
        assert!(barrier.is_satisfied());
        assert!((barrier.progress_pct() - 100.0).abs() < 0.01);
    }
}
