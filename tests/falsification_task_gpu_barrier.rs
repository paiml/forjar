//! FJ-2703/2704: GPU scheduling and barrier synchronization.
//!
//! Popperian rejection criteria for:
//! - FJ-2703: GpuSchedule (round-robin, assign, utilization, cuda_visible_devices)
//! - FJ-2704: BarrierTask (mark_complete, is_satisfied, pending, progress, display)
//!
//! Usage: cargo test --test falsification_task_gpu_barrier

use forjar::core::types::{BarrierTask, GpuSchedule};

// ============================================================================
// FJ-2703: GpuSchedule — round-robin and utilization
// ============================================================================

#[test]
fn gpu_schedule_round_robin_wraps() {
    let tasks = &["train", "eval", "infer", "export", "test"];
    let schedule = GpuSchedule::round_robin(tasks, 3);
    assert_eq!(schedule.total_devices, 3);
    assert_eq!(schedule.cuda_visible_devices("train").as_deref(), Some("0"));
    assert_eq!(schedule.cuda_visible_devices("eval").as_deref(), Some("1"));
    assert_eq!(schedule.cuda_visible_devices("infer").as_deref(), Some("2"));
    assert_eq!(
        schedule.cuda_visible_devices("export").as_deref(),
        Some("0")
    );
    assert_eq!(schedule.cuda_visible_devices("test").as_deref(), Some("1"));
}

#[test]
fn gpu_schedule_utilization_tracking() {
    let mut schedule = GpuSchedule::new(4);
    assert!(!schedule.fully_utilized());
    assert_eq!(schedule.assigned_device_count(), 0);

    schedule.assign("a", vec![0, 1]);
    assert_eq!(schedule.assigned_device_count(), 2);
    assert!(!schedule.fully_utilized());

    schedule.assign("b", vec![2, 3]);
    assert_eq!(schedule.assigned_device_count(), 4);
    assert!(schedule.fully_utilized());
}

#[test]
fn gpu_schedule_unassigned_returns_none() {
    let schedule = GpuSchedule::new(2);
    assert!(schedule.cuda_visible_devices("missing").is_none());
}

#[test]
fn gpu_schedule_multi_gpu_assignment() {
    let mut schedule = GpuSchedule::new(8);
    schedule.assign("large-model", vec![0, 1, 2, 3]);
    assert_eq!(
        schedule.cuda_visible_devices("large-model").as_deref(),
        Some("0,1,2,3")
    );
    assert_eq!(schedule.assigned_device_count(), 4);
}

#[test]
fn gpu_schedule_single_device() {
    let schedule = GpuSchedule::round_robin(&["only-task"], 1);
    assert_eq!(
        schedule.cuda_visible_devices("only-task").as_deref(),
        Some("0")
    );
    assert!(schedule.fully_utilized());
}

// ============================================================================
// FJ-2704: BarrierTask — synchronization
// ============================================================================

#[test]
fn barrier_initially_unsatisfied() {
    let barrier = BarrierTask::new("sync", vec!["m1".into(), "m2".into(), "m3".into()]);
    assert!(!barrier.is_satisfied());
    assert_eq!(barrier.pending_machines().len(), 3);
    assert!((barrier.progress_pct() - 0.0).abs() < 0.01);
}

#[test]
fn barrier_partial_completion() {
    let mut barrier = BarrierTask::new("sync", vec!["m1".into(), "m2".into()]);
    barrier.mark_complete("m1");
    assert!(!barrier.is_satisfied());
    assert_eq!(barrier.pending_machines(), vec!["m2"]);
    assert!((barrier.progress_pct() - 50.0).abs() < 0.01);
}

#[test]
fn barrier_full_completion() {
    let mut barrier = BarrierTask::new("sync", vec!["m1".into(), "m2".into()]);
    barrier.mark_complete("m1");
    barrier.mark_complete("m2");
    assert!(barrier.is_satisfied());
    assert!(barrier.pending_machines().is_empty());
    assert!((barrier.progress_pct() - 100.0).abs() < 0.01);
}

#[test]
fn barrier_duplicate_mark_ignored() {
    let mut barrier = BarrierTask::new("sync", vec!["m1".into()]);
    barrier.mark_complete("m1");
    barrier.mark_complete("m1");
    assert_eq!(barrier.completed.len(), 1);
}

#[test]
fn barrier_display_waiting() {
    let barrier = BarrierTask::new("deploy", vec!["web".into(), "db".into()]);
    let display = barrier.to_string();
    assert!(display.contains("barrier/deploy"));
    assert!(display.contains("waiting"));
}

#[test]
fn barrier_display_satisfied() {
    let mut barrier = BarrierTask::new("deploy", vec!["web".into()]);
    barrier.mark_complete("web");
    assert!(barrier.to_string().contains("SATISFIED"));
}

#[test]
fn barrier_empty_machines_is_satisfied() {
    let barrier = BarrierTask::new("noop", vec![]);
    assert!(barrier.is_satisfied());
    assert!((barrier.progress_pct() - 100.0).abs() < 0.01);
}

#[test]
fn barrier_timeout_field() {
    let mut barrier = BarrierTask::new("sync", vec!["m1".into()]);
    barrier.timeout_secs = 120;
    assert_eq!(barrier.timeout_secs, 120);
}
