//! FJ-2301: Run log capture example.
//!
//! Demonstrates the run log types for persistent transport output capture.
//!
//! ```bash
//! cargo run --example run_log
//! ```

use forjar::core::types::{
    LogRetention, ResourceRunStatus, RunLogEntry, RunMeta, generate_run_id,
};

fn main() {
    demo_run_id();
    demo_run_meta();
    demo_log_entry();
    demo_retention();
}

fn demo_run_id() {
    println!("=== FJ-2301: Run ID Generation ===\n");
    for i in 0..3 {
        let id = generate_run_id();
        println!("  Run {i}: {id}");
    }
    println!();
}

fn demo_run_meta() {
    println!("=== FJ-2301: Run Metadata ===\n");
    let run_id = generate_run_id();
    let mut meta = RunMeta::new(run_id.clone(), "intel".into(), "apply".into());
    meta.operator = Some("noah@workstation".into());
    meta.generation = Some(13);

    meta.record_resource("bash-aliases", ResourceRunStatus::Noop);
    meta.record_resource(
        "gitconfig",
        ResourceRunStatus::Converged {
            exit_code: Some(0),
            duration_secs: Some(0.54),
            failed: false,
        },
    );
    meta.record_resource(
        "cargo-tools",
        ResourceRunStatus::Converged {
            exit_code: Some(100),
            duration_secs: Some(1.8),
            failed: true,
        },
    );
    meta.record_resource(
        "stack-tools",
        ResourceRunStatus::Skipped {
            reason: Some("dependency cargo-tools failed".into()),
        },
    );

    println!("  Run: {run_id}");
    println!("  Machine: {}", meta.machine);
    println!("  Summary: {} total, {} converged, {} noop, {} failed, {} skipped",
        meta.summary.total,
        meta.summary.converged,
        meta.summary.noop,
        meta.summary.failed,
        meta.summary.skipped,
    );
    println!();
}

fn demo_log_entry() {
    println!("=== FJ-2301: Structured Log Entry ===\n");
    let entry = RunLogEntry {
        resource_id: "cargo-tools".into(),
        resource_type: "package".into(),
        action: "apply".into(),
        machine: "intel".into(),
        transport: "ssh".into(),
        script: "#!/bin/bash\nset -euo pipefail\napt-get update -qq\napt-get install -y cargo-watch cargo-edit".into(),
        script_hash: "blake3:f8a9b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1".into(),
        stdout: "Reading package lists...\ncargo-edit is already the newest version.\nE: Unable to locate package cargo-watch".into(),
        stderr: "E: Unable to locate package cargo-watch".into(),
        exit_code: 100,
        duration_secs: 1.8,
        started_at: "2026-03-05T14:30:01.400Z".into(),
        finished_at: "2026-03-05T14:30:03.200Z".into(),
    };

    print!("{}", entry.format_log());
    println!();
}

fn demo_retention() {
    println!("=== FJ-2301: Retention Policy ===\n");
    let retention = LogRetention::default();
    println!("  keep_runs: {}", retention.keep_runs);
    println!("  keep_failed: {}", retention.keep_failed);
    println!("  max_log_size: {} MB", retention.max_log_size / 1024 / 1024);
    println!("  max_total_size: {} MB", retention.max_total_size / 1024 / 1024);
}
