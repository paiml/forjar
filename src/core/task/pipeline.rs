//! FJ-2700: Pipeline stage execution engine.
//!
//! Processes ordered multi-stage pipelines with content-addressed caching,
//! inter-stage gates, and state tracking.

use crate::core::types::{PipelineStage, PipelineState, StageState, StageStatus};
use std::path::Path;

/// Result of executing a single pipeline stage.
#[derive(Debug, Clone)]
pub struct StageExecResult {
    /// Stage name.
    pub name: String,
    /// Whether the stage was skipped (cache hit).
    pub cached: bool,
    /// Exit code (0 = success).
    pub exit_code: i32,
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// BLAKE3 hash of inputs (if computed).
    pub input_hash: Option<String>,
}

/// Build a pipeline execution plan: which stages to run, which to skip.
///
/// # Examples
///
/// ```
/// use forjar::core::task::pipeline::{plan_pipeline, should_skip_stage};
/// use forjar::core::types::{PipelineStage, PipelineState, StageState, StageStatus};
///
/// let stages = vec![
///     PipelineStage { name: "lint".into(), ..Default::default() },
///     PipelineStage { name: "test".into(), gate: true, ..Default::default() },
///     PipelineStage { name: "deploy".into(), ..Default::default() },
/// ];
/// let plan = plan_pipeline(&stages, &PipelineState::default(), true, &std::path::Path::new("."));
/// assert_eq!(plan.len(), 3);
/// assert!(!plan[0].skip); // No previous state → must run
/// ```
pub fn plan_pipeline(
    stages: &[PipelineStage],
    previous_state: &PipelineState,
    cache_enabled: bool,
    base_dir: &Path,
) -> Vec<StagePlan> {
    stages
        .iter()
        .map(|stage| {
            let skip = should_skip_stage(stage, previous_state, cache_enabled, base_dir);
            StagePlan {
                name: stage.name.clone(),
                skip,
                is_gate: stage.gate,
            }
        })
        .collect()
}

/// Determine if a stage can be skipped due to caching.
///
/// A stage is skipped when:
/// 1. Caching is enabled
/// 2. The stage has inputs
/// 3. The previous state has a matching stage with `Passed` status
/// 4. The input hash matches the stored input hash
pub fn should_skip_stage(
    stage: &PipelineStage,
    previous_state: &PipelineState,
    cache_enabled: bool,
    base_dir: &Path,
) -> bool {
    if !cache_enabled || stage.inputs.is_empty() {
        return false;
    }

    let prev = previous_state.stages.iter().find(|s| s.name == stage.name);

    let prev = match prev {
        Some(s) if s.status == StageStatus::Passed => s,
        _ => return false,
    };

    let stored_hash = match &prev.input_hash {
        Some(h) => h,
        None => return false,
    };

    match super::hash_inputs(&stage.inputs, base_dir) {
        Ok(Some(current_hash)) => current_hash == *stored_hash,
        _ => false,
    }
}

/// A single entry in the pipeline execution plan.
#[derive(Debug, Clone)]
pub struct StagePlan {
    /// Stage name.
    pub name: String,
    /// Whether this stage should be skipped (cache hit).
    pub skip: bool,
    /// Whether this is a gate stage (pipeline stops on failure).
    pub is_gate: bool,
}

/// Build a `PipelineState` from stage execution results.
///
/// # Examples
///
/// ```
/// use forjar::core::task::pipeline::{build_pipeline_state, StageExecResult};
/// use forjar::core::types::StageStatus;
///
/// let results = vec![
///     StageExecResult { name: "lint".into(), cached: false, exit_code: 0, duration_ms: 500, input_hash: None },
///     StageExecResult { name: "test".into(), cached: false, exit_code: 1, duration_ms: 3000, input_hash: None },
/// ];
/// let state = build_pipeline_state(&results);
/// assert_eq!(state.status, StageStatus::Failed);
/// assert_eq!(state.last_completed, Some(0));
/// assert_eq!(state.stages.len(), 2);
/// ```
pub fn build_pipeline_state(results: &[StageExecResult]) -> PipelineState {
    let mut stages = Vec::with_capacity(results.len());
    let mut last_completed: Option<usize> = None;
    let mut overall = StageStatus::Passed;

    for (i, result) in results.iter().enumerate() {
        let status = if result.cached {
            StageStatus::Skipped
        } else if result.exit_code == 0 {
            StageStatus::Passed
        } else {
            StageStatus::Failed
        };

        if status == StageStatus::Passed || status == StageStatus::Skipped {
            last_completed = Some(i);
        }

        if status == StageStatus::Failed {
            overall = StageStatus::Failed;
        }

        stages.push(StageState {
            name: result.name.clone(),
            status,
            exit_code: Some(result.exit_code),
            duration_ms: Some(result.duration_ms),
            input_hash: result.input_hash.clone(),
        });
    }

    PipelineState {
        stages,
        status: overall,
        last_completed,
    }
}

/// Generate the shell command for a pipeline stage.
///
/// Wraps the stage command with working directory changes if needed.
pub fn stage_command(stage: &PipelineStage) -> String {
    match &stage.command {
        Some(cmd) => {
            let mut script = String::from("set -euo pipefail\n");
            script.push_str(cmd);
            if !cmd.ends_with('\n') {
                script.push('\n');
            }
            script
        }
        None => "true\n".to_string(),
    }
}

/// Format a pipeline state as a human-readable summary.
pub fn format_pipeline_summary(state: &PipelineState) -> String {
    let mut out = String::new();
    for (i, stage) in state.stages.iter().enumerate() {
        let status_str = match stage.status {
            StageStatus::Pending => "PENDING",
            StageStatus::Running => "RUNNING",
            StageStatus::Passed => "PASS",
            StageStatus::Failed => "FAIL",
            StageStatus::Skipped => "SKIP",
        };
        let duration = stage
            .duration_ms
            .map(|ms| format!(" ({ms}ms)"))
            .unwrap_or_default();
        out.push_str(&format!(
            "  [{:>2}] [{status_str:>7}] {}{duration}\n",
            i + 1,
            stage.name
        ));
    }

    let overall = match state.status {
        StageStatus::Passed => "PASSED",
        StageStatus::Failed => "FAILED",
        _ => "INCOMPLETE",
    };
    out.push_str(&format!("\n  Pipeline: {overall}\n"));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_pipeline_state_all_pass() {
        let results = vec![
            StageExecResult {
                name: "lint".into(),
                cached: false,
                exit_code: 0,
                duration_ms: 200,
                input_hash: None,
            },
            StageExecResult {
                name: "test".into(),
                cached: false,
                exit_code: 0,
                duration_ms: 1500,
                input_hash: None,
            },
        ];
        let state = build_pipeline_state(&results);
        assert_eq!(state.status, StageStatus::Passed);
        assert_eq!(state.last_completed, Some(1));
        assert_eq!(state.stages.len(), 2);
        assert_eq!(state.stages[0].status, StageStatus::Passed);
        assert_eq!(state.stages[1].status, StageStatus::Passed);
    }

    #[test]
    fn build_pipeline_state_with_failure() {
        let results = vec![
            StageExecResult {
                name: "build".into(),
                cached: false,
                exit_code: 0,
                duration_ms: 5000,
                input_hash: None,
            },
            StageExecResult {
                name: "test".into(),
                cached: false,
                exit_code: 1,
                duration_ms: 3000,
                input_hash: None,
            },
        ];
        let state = build_pipeline_state(&results);
        assert_eq!(state.status, StageStatus::Failed);
        assert_eq!(state.last_completed, Some(0));
        assert_eq!(state.stages[1].status, StageStatus::Failed);
    }

    #[test]
    fn build_pipeline_state_with_cache_skip() {
        let results = vec![
            StageExecResult {
                name: "pull".into(),
                cached: true,
                exit_code: 0,
                duration_ms: 0,
                input_hash: Some("blake3:abc".into()),
            },
            StageExecResult {
                name: "convert".into(),
                cached: false,
                exit_code: 0,
                duration_ms: 2000,
                input_hash: None,
            },
        ];
        let state = build_pipeline_state(&results);
        assert_eq!(state.status, StageStatus::Passed);
        assert_eq!(state.stages[0].status, StageStatus::Skipped);
        assert_eq!(state.stages[1].status, StageStatus::Passed);
    }

    #[test]
    fn build_pipeline_state_empty() {
        let state = build_pipeline_state(&[]);
        assert_eq!(state.status, StageStatus::Passed);
        assert!(state.stages.is_empty());
        assert!(state.last_completed.is_none());
    }

    #[test]
    fn stage_command_with_cmd() {
        let stage = PipelineStage {
            name: "build".into(),
            command: Some("cargo build --release".into()),
            ..Default::default()
        };
        let cmd = stage_command(&stage);
        assert!(cmd.contains("set -euo pipefail"));
        assert!(cmd.contains("cargo build --release"));
    }

    #[test]
    fn stage_command_no_cmd() {
        let stage = PipelineStage {
            name: "noop".into(),
            command: None,
            ..Default::default()
        };
        let cmd = stage_command(&stage);
        assert_eq!(cmd, "true\n");
    }

    #[test]
    fn format_pipeline_summary_passed() {
        let state = PipelineState {
            stages: vec![
                StageState {
                    name: "lint".into(),
                    status: StageStatus::Passed,
                    exit_code: Some(0),
                    duration_ms: Some(200),
                    input_hash: None,
                },
                StageState {
                    name: "test".into(),
                    status: StageStatus::Passed,
                    exit_code: Some(0),
                    duration_ms: Some(1500),
                    input_hash: None,
                },
            ],
            status: StageStatus::Passed,
            last_completed: Some(1),
        };
        let summary = format_pipeline_summary(&state);
        assert!(summary.contains("[   PASS] lint"));
        assert!(summary.contains("[   PASS] test"));
        assert!(summary.contains("Pipeline: PASSED"));
    }

    #[test]
    fn format_pipeline_summary_failed() {
        let state = PipelineState {
            stages: vec![
                StageState {
                    name: "build".into(),
                    status: StageStatus::Passed,
                    exit_code: Some(0),
                    duration_ms: Some(5000),
                    input_hash: None,
                },
                StageState {
                    name: "deploy".into(),
                    status: StageStatus::Failed,
                    exit_code: Some(1),
                    duration_ms: Some(100),
                    input_hash: None,
                },
            ],
            status: StageStatus::Failed,
            last_completed: Some(0),
        };
        let summary = format_pipeline_summary(&state);
        assert!(summary.contains("[   FAIL] deploy"));
        assert!(summary.contains("Pipeline: FAILED"));
    }

    #[test]
    fn format_pipeline_summary_with_skip() {
        let state = PipelineState {
            stages: vec![StageState {
                name: "cached".into(),
                status: StageStatus::Skipped,
                exit_code: Some(0),
                duration_ms: Some(0),
                input_hash: Some("hash".into()),
            }],
            status: StageStatus::Passed,
            last_completed: Some(0),
        };
        let summary = format_pipeline_summary(&state);
        assert!(summary.contains("[   SKIP] cached"));
    }

    #[test]
    fn plan_pipeline_no_cache() {
        let stages = vec![
            PipelineStage {
                name: "a".into(),
                ..Default::default()
            },
            PipelineStage {
                name: "b".into(),
                ..Default::default()
            },
        ];
        let plan = plan_pipeline(&stages, &PipelineState::default(), false, Path::new("."));
        assert_eq!(plan.len(), 2);
        assert!(!plan[0].skip);
        assert!(!plan[1].skip);
    }

    #[test]
    fn plan_pipeline_gate_flag() {
        let stages = vec![PipelineStage {
            name: "gate".into(),
            gate: true,
            ..Default::default()
        }];
        let plan = plan_pipeline(&stages, &PipelineState::default(), true, Path::new("."));
        assert!(plan[0].is_gate);
    }

    #[test]
    fn should_skip_no_inputs() {
        let stage = PipelineStage {
            name: "a".into(),
            ..Default::default()
        };
        let state = PipelineState::default();
        assert!(!should_skip_stage(&stage, &state, true, Path::new(".")));
    }

    #[test]
    fn should_skip_no_previous() {
        let stage = PipelineStage {
            name: "a".into(),
            inputs: vec!["*.txt".into()],
            ..Default::default()
        };
        let state = PipelineState::default();
        assert!(!should_skip_stage(&stage, &state, true, Path::new(".")));
    }
}
