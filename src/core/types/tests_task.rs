//! Tests for task_types.rs — task modes, pipeline stages, GPU scheduling, barriers.

use super::task_types::*;

#[test]
fn task_mode_serde_roundtrip() {
    for mode in [
        TaskMode::Batch,
        TaskMode::Pipeline,
        TaskMode::Service,
        TaskMode::Dispatch,
    ] {
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
            StageState {
                name: "lint".into(),
                status: StageStatus::Passed,
                exit_code: Some(0),
                duration_ms: Some(1200),
                input_hash: None,
            },
            StageState {
                name: "test".into(),
                status: StageStatus::Failed,
                exit_code: Some(1),
                duration_ms: Some(5000),
                input_hash: None,
            },
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
    assert_eq!(
        schedule.cuda_visible_devices("big-model"),
        Some("0,1,2,3".into())
    );
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
    let mut barrier = BarrierTask::new(
        "sync-all",
        vec!["intel".into(), "jetson".into(), "lambda".into()],
    );
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
