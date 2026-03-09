//! FJ-2003/2700/2605: Generation diffs, GPU scheduling, barriers, coverage levels.
//! Usage: cargo test --test falsification_diff_task_coverage

use forjar::core::types::*;

// ── FJ-2003: diff_resource_sets ──

#[test]
fn diff_sets_basic() {
    let from = vec![
        ("a", "file", "h1"),
        ("b", "package", "h2"),
        ("c", "service", "h3"),
    ];
    let to = vec![
        ("a", "file", "h1"),
        ("b", "package", "h2new"),
        ("d", "file", "h4"),
    ];
    let diffs = diff_resource_sets(&from, &to);
    assert_eq!(diffs.len(), 4);
    assert_eq!(
        diffs.iter().find(|d| d.resource_id == "a").unwrap().action,
        DiffAction::Unchanged
    );
    assert_eq!(
        diffs.iter().find(|d| d.resource_id == "b").unwrap().action,
        DiffAction::Modified
    );
    assert_eq!(
        diffs.iter().find(|d| d.resource_id == "c").unwrap().action,
        DiffAction::Removed
    );
    assert_eq!(
        diffs.iter().find(|d| d.resource_id == "d").unwrap().action,
        DiffAction::Added
    );
}

#[test]
fn diff_sets_empty() {
    assert!(diff_resource_sets(&[], &[]).is_empty());
}

#[test]
fn diff_sets_all_new() {
    let diffs = diff_resource_sets(&[], &[("a", "file", "h1")]);
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].action, DiffAction::Added);
}

#[test]
fn diff_sets_all_removed() {
    let diffs = diff_resource_sets(&[("a", "file", "h1")], &[]);
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].action, DiffAction::Removed);
}

#[test]
fn diff_sets_sorted() {
    let diffs = diff_resource_sets(
        &[("z", "f", "1"), ("a", "f", "2")],
        &[("z", "f", "1"), ("a", "f", "2")],
    );
    assert_eq!(diffs[0].resource_id, "a");
    assert_eq!(diffs[1].resource_id, "z");
}

#[test]
fn diff_sets_hash_recorded() {
    let diffs = diff_resource_sets(&[("x", "file", "old")], &[("x", "file", "new")]);
    let d = &diffs[0];
    assert_eq!(d.action, DiffAction::Modified);
    assert_eq!(d.old_hash.as_deref(), Some("old"));
    assert_eq!(d.new_hash.as_deref(), Some("new"));
}

// ── FJ-2003: ResourceDiff builders ──

#[test]
fn resource_diff_added() {
    let d = ResourceDiff::added("pkg", "package");
    assert_eq!(d.action, DiffAction::Added);
    assert!(d.old_hash.is_none());
}

#[test]
fn resource_diff_with_hashes_and_detail() {
    let d = ResourceDiff::modified("cfg", "file")
        .with_hashes(Some("old".into()), Some("new".into()))
        .with_detail("content changed");
    assert_eq!(d.old_hash.as_deref(), Some("old"));
    assert_eq!(d.detail.as_deref(), Some("content changed"));
}

// ── FJ-2003: GenerationDiff ──

fn sample_diff() -> GenerationDiff {
    GenerationDiff {
        gen_from: 5,
        gen_to: 8,
        machine: "intel".into(),
        resources: vec![
            ResourceDiff::added("new-pkg", "package"),
            ResourceDiff::modified("config", "file").with_detail("content changed"),
            ResourceDiff::removed("old-svc", "service"),
            ResourceDiff::unchanged("stable", "file"),
        ],
    }
}

#[test]
fn gen_diff_counts() {
    let d = sample_diff();
    assert_eq!(d.added_count(), 1);
    assert_eq!(d.modified_count(), 1);
    assert_eq!(d.removed_count(), 1);
    assert_eq!(d.unchanged_count(), 1);
    assert_eq!(d.change_count(), 3);
    assert!(d.has_changes());
}

#[test]
fn gen_diff_no_changes() {
    let d = GenerationDiff {
        gen_from: 1,
        gen_to: 1,
        machine: "m".into(),
        resources: vec![ResourceDiff::unchanged("x", "file")],
    };
    assert!(!d.has_changes());
    assert_eq!(d.change_count(), 0);
}

#[test]
fn gen_diff_format_summary() {
    let s = sample_diff().format_summary();
    assert!(s.contains("generation 5 → 8"));
    assert!(s.contains("intel"));
    assert!(s.contains("+ new-pkg"));
    assert!(s.contains("~ config"));
    assert!(s.contains("- old-svc"));
    assert!(s.contains("content changed"));
    assert!(!s.contains("stable")); // unchanged omitted
}

#[test]
fn gen_diff_serde_roundtrip() {
    let d = sample_diff();
    let json = serde_json::to_string(&d).unwrap();
    let parsed: GenerationDiff = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.gen_from, 5);
    assert_eq!(parsed.resources.len(), 4);
}

#[test]
fn diff_action_display() {
    assert_eq!(DiffAction::Added.to_string(), "added");
    assert_eq!(DiffAction::Removed.to_string(), "removed");
    assert_eq!(DiffAction::Modified.to_string(), "modified");
    assert_eq!(DiffAction::Unchanged.to_string(), "unchanged");
}

#[test]
fn diff_action_serde() {
    for action in [
        DiffAction::Added,
        DiffAction::Removed,
        DiffAction::Modified,
        DiffAction::Unchanged,
    ] {
        let json = serde_json::to_string(&action).unwrap();
        let parsed: DiffAction = serde_json::from_str(&json).unwrap();
        assert_eq!(action, parsed);
    }
}

// ── FJ-2703: GpuSchedule ──

#[test]
fn gpu_schedule_new() {
    let s = GpuSchedule::new(4);
    assert_eq!(s.total_devices, 4);
    assert_eq!(s.assigned_device_count(), 0);
    assert!(!s.fully_utilized());
}

#[test]
fn gpu_schedule_assign() {
    let mut s = GpuSchedule::new(4);
    s.assign("train", vec![0, 1]);
    s.assign("eval", vec![2]);
    assert_eq!(s.cuda_visible_devices("train"), Some("0,1".into()));
    assert_eq!(s.cuda_visible_devices("eval"), Some("2".into()));
    assert_eq!(s.cuda_visible_devices("missing"), None);
    assert_eq!(s.assigned_device_count(), 3);
    assert!(!s.fully_utilized());
}

#[test]
fn gpu_schedule_fully_utilized() {
    let mut s = GpuSchedule::new(2);
    s.assign("a", vec![0]);
    s.assign("b", vec![1]);
    assert!(s.fully_utilized());
}

#[test]
fn gpu_schedule_round_robin() {
    let s = GpuSchedule::round_robin(&["t1", "t2", "t3", "t4", "t5"], 3);
    assert_eq!(s.cuda_visible_devices("t1"), Some("0".into()));
    assert_eq!(s.cuda_visible_devices("t2"), Some("1".into()));
    assert_eq!(s.cuda_visible_devices("t3"), Some("2".into()));
    assert_eq!(s.cuda_visible_devices("t4"), Some("0".into())); // wraps
    assert_eq!(s.cuda_visible_devices("t5"), Some("1".into()));
}

#[test]
fn gpu_schedule_round_robin_single_device() {
    let s = GpuSchedule::round_robin(&["a", "b", "c"], 1);
    assert_eq!(s.cuda_visible_devices("a"), Some("0".into()));
    assert_eq!(s.cuda_visible_devices("b"), Some("0".into()));
}

// ── FJ-2704: BarrierTask ──

#[test]
fn barrier_new() {
    let b = BarrierTask::new("sync-1", vec!["intel".into(), "jetson".into()]);
    assert_eq!(b.task_id, "sync-1");
    assert!(!b.is_satisfied());
    assert_eq!(b.pending_machines(), vec!["intel", "jetson"]);
    assert!((b.progress_pct() - 0.0).abs() < f64::EPSILON);
}

#[test]
fn barrier_mark_complete() {
    let mut b = BarrierTask::new("sync", vec!["a".into(), "b".into()]);
    b.mark_complete("a");
    assert!(!b.is_satisfied());
    assert_eq!(b.pending_machines(), vec!["b"]);
    assert!((b.progress_pct() - 50.0).abs() < f64::EPSILON);
    b.mark_complete("b");
    assert!(b.is_satisfied());
    assert!(b.pending_machines().is_empty());
    assert!((b.progress_pct() - 100.0).abs() < f64::EPSILON);
}

#[test]
fn barrier_duplicate_complete() {
    let mut b = BarrierTask::new("sync", vec!["a".into()]);
    b.mark_complete("a");
    b.mark_complete("a"); // duplicate
    assert_eq!(b.completed.len(), 1);
    assert!(b.is_satisfied());
}

#[test]
fn barrier_empty_machines() {
    let b = BarrierTask::new("sync", vec![]);
    assert!(b.is_satisfied());
    assert!((b.progress_pct() - 100.0).abs() < f64::EPSILON);
}

#[test]
fn barrier_display_waiting() {
    let b = BarrierTask::new("deploy", vec!["intel".into(), "jetson".into()]);
    let s = format!("{b}");
    assert!(s.contains("barrier/deploy"));
    assert!(s.contains("waiting for"));
}

#[test]
fn barrier_display_satisfied() {
    let mut b = BarrierTask::new("deploy", vec!["a".into()]);
    b.mark_complete("a");
    let s = format!("{b}");
    assert!(s.contains("SATISFIED"));
}

// ── FJ-2700: TaskMode ──

#[test]
fn task_mode_display() {
    assert_eq!(TaskMode::Batch.to_string(), "batch");
    assert_eq!(TaskMode::Pipeline.to_string(), "pipeline");
    assert_eq!(TaskMode::Service.to_string(), "service");
    assert_eq!(TaskMode::Dispatch.to_string(), "dispatch");
}

#[test]
fn task_mode_default() {
    assert_eq!(TaskMode::default(), TaskMode::Batch);
}

#[test]
fn task_mode_serde() {
    for mode in [
        TaskMode::Batch,
        TaskMode::Pipeline,
        TaskMode::Service,
        TaskMode::Dispatch,
    ] {
        let json = serde_json::to_string(&mode).unwrap();
        let parsed: TaskMode = serde_json::from_str(&json).unwrap();
        assert_eq!(mode, parsed);
    }
}

// ── FJ-2605: CoverageLevel ──

#[test]
fn coverage_level_ordering() {
    assert!(CoverageLevel::L0 < CoverageLevel::L1);
    assert!(CoverageLevel::L1 < CoverageLevel::L2);
    assert!(CoverageLevel::L4 < CoverageLevel::L5);
}

#[test]
fn coverage_level_labels() {
    assert_eq!(CoverageLevel::L0.label(), "no tests");
    assert_eq!(CoverageLevel::L3.label(), "convergence tested");
    assert_eq!(CoverageLevel::L5.label(), "preservation tested");
}

#[test]
fn coverage_level_display() {
    assert_eq!(CoverageLevel::L0.to_string(), "L0 (no tests)");
    assert_eq!(CoverageLevel::L5.to_string(), "L5 (preservation tested)");
}

#[test]
fn coverage_level_values() {
    assert_eq!(CoverageLevel::L0.value(), 0);
    assert_eq!(CoverageLevel::L5.value(), 5);
}

#[test]
fn coverage_level_default() {
    assert_eq!(CoverageLevel::default(), CoverageLevel::L0);
}

#[test]
fn coverage_level_serde() {
    for level in [
        CoverageLevel::L0,
        CoverageLevel::L1,
        CoverageLevel::L2,
        CoverageLevel::L3,
        CoverageLevel::L4,
        CoverageLevel::L5,
    ] {
        let json = serde_json::to_string(&level).unwrap();
        let parsed: CoverageLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(level, parsed);
    }
}

// ── FJ-2605: CoverageReport ──

fn rc(id: &str, level: CoverageLevel) -> ResourceCoverage {
    ResourceCoverage {
        resource_id: id.into(),
        level,
        resource_type: "file".into(),
    }
}

#[test]
fn coverage_report_empty() {
    let report = CoverageReport::from_entries(vec![]);
    assert_eq!(report.min_level, CoverageLevel::L0);
    assert_eq!(report.avg_level, 0.0);
    assert!(report.meets_threshold(CoverageLevel::L0));
}

#[test]
fn coverage_report_basic() {
    let report = CoverageReport::from_entries(vec![
        rc("a", CoverageLevel::L4),
        rc("b", CoverageLevel::L3),
        rc("c", CoverageLevel::L1),
    ]);
    assert_eq!(report.min_level, CoverageLevel::L1);
    assert!((report.avg_level - 2.67).abs() < 0.1);
    assert!(report.meets_threshold(CoverageLevel::L1));
    assert!(!report.meets_threshold(CoverageLevel::L2));
    assert_eq!(report.histogram[1], 1); // L1
    assert_eq!(report.histogram[3], 1); // L3
    assert_eq!(report.histogram[4], 1); // L4
}

#[test]
fn coverage_report_all_l5() {
    let report =
        CoverageReport::from_entries(vec![rc("a", CoverageLevel::L5), rc("b", CoverageLevel::L5)]);
    assert!(report.meets_threshold(CoverageLevel::L5));
    assert_eq!(report.avg_level, 5.0);
}

#[test]
fn coverage_report_format() {
    let report = CoverageReport::from_entries(vec![rc("pkg", CoverageLevel::L2)]);
    let text = report.format_report();
    assert!(text.contains("pkg:"));
    assert!(text.contains("L2 (behavior spec)"));
    assert!(text.contains("Min: L2"));
}
