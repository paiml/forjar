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
}
