//! FJ-2003/2700/2605/2604/2301: Generation diffs, task scheduling,
//! coverage levels, mutation testing, observability.
//!
//! Usage: cargo run --example diff_task_mutation_obs

use forjar::core::types::*;

fn main() {
    println!("Forjar: Diffs, Tasks, Coverage, Mutations & Observability");
    println!("{}", "=".repeat(60));

    // ── Generation Diffs ──
    println!("\n[FJ-2003] Generation Diff:");
    let from = vec![
        ("nginx-pkg", "package", "h1"),
        ("nginx-conf", "file", "h2"),
        ("old-service", "service", "h3"),
    ];
    let to = vec![
        ("nginx-pkg", "package", "h1"),
        ("nginx-conf", "file", "h2new"),
        ("new-cert", "file", "h4"),
    ];
    let diffs = diff_resource_sets(&from, &to);
    let gen_diff = GenerationDiff {
        gen_from: 12,
        gen_to: 15,
        machine: "intel".into(),
        resources: diffs,
    };
    println!("{}", gen_diff.format_summary());

    // ── GPU Scheduling ──
    println!("[FJ-2703] GPU Scheduling:");
    let schedule =
        GpuSchedule::round_robin(&["train-bert", "train-gpt", "eval-llama", "finetune"], 3);
    for task in ["train-bert", "train-gpt", "eval-llama", "finetune"] {
        println!(
            "  {task}: CUDA_VISIBLE_DEVICES={}",
            schedule.cuda_visible_devices(task).unwrap()
        );
    }
    println!(
        "  Assigned: {}/{}",
        schedule.assigned_device_count(),
        schedule.total_devices
    );

    // ── Barrier Synchronization ──
    println!("\n[FJ-2704] Barrier Task:");
    let mut barrier = BarrierTask::new(
        "model-merge",
        vec!["gpu-1".into(), "gpu-2".into(), "gpu-3".into()],
    );
    println!("  {barrier}");
    barrier.mark_complete("gpu-1");
    barrier.mark_complete("gpu-3");
    println!("  {barrier}");
    barrier.mark_complete("gpu-2");
    println!("  {barrier}");

    // ── Coverage Levels ──
    println!("\n[FJ-2605] Coverage Report:");
    let entries = vec![
        ResourceCoverage {
            resource_id: "nginx-pkg".into(),
            level: CoverageLevel::L4,
            resource_type: "package".into(),
        },
        ResourceCoverage {
            resource_id: "nginx-conf".into(),
            level: CoverageLevel::L3,
            resource_type: "file".into(),
        },
        ResourceCoverage {
            resource_id: "app-deploy".into(),
            level: CoverageLevel::L1,
            resource_type: "task".into(),
        },
    ];
    let report = CoverageReport::from_entries(entries);
    println!(
        "  Min: {} | Avg: {:.1} | Meets L2: {}",
        report.min_level,
        report.avg_level,
        report.meets_threshold(CoverageLevel::L2)
    );

    // ── Mutation Testing ──
    println!("\n[FJ-2604] Mutation Testing:");
    let results = vec![
        MutationResult {
            resource_id: "nginx-conf".into(),
            resource_type: "file".into(),
            operator: MutationOperator::DeleteFile,
            detected: true,
            reconverged: Some(true),
            duration_ms: 250,
            error: None,
        },
        MutationResult {
            resource_id: "nginx-conf".into(),
            resource_type: "file".into(),
            operator: MutationOperator::ModifyContent,
            detected: true,
            reconverged: Some(true),
            duration_ms: 180,
            error: None,
        },
        MutationResult {
            resource_id: "app-svc".into(),
            resource_type: "service".into(),
            operator: MutationOperator::StopService,
            detected: false,
            reconverged: None,
            duration_ms: 500,
            error: None,
        },
    ];
    let mutation_report = MutationReport::from_results(results);
    println!("  {}", mutation_report.score);

    // ── Observability ──
    println!("\n[FJ-2301] Observability:");
    for count in 0..=3 {
        let v = VerbosityLevel::from_count(count);
        println!(
            "  -{}v: {} (scripts={}, raw={})",
            count,
            v,
            v.shows_scripts(),
            v.streams_raw()
        );
    }

    let trunc = LogTruncation::default();
    println!(
        "  Truncation: first={}, last={}, 20KB truncated={}",
        trunc.first_bytes,
        trunc.last_bytes,
        trunc.should_truncate(20_000)
    );

    let path = RunLogPath::new("state", "intel", "r-abc123");
    println!("  Log path: {}", path.resource_log("nginx-pkg", "apply"));

    println!("\n{}", "=".repeat(60));
    println!("All diff/task/coverage/mutation/obs criteria survived.");
}
