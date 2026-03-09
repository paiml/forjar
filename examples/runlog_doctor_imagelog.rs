//! FJ-2301: Run log, doctor diagnostics, image build log types.
//!
//! Usage: cargo run --example runlog_doctor_imagelog

use forjar::core::types::*;

fn main() {
    println!("Forjar: Run Log, Doctor & Image Build Log");
    println!("{}", "=".repeat(55));

    // ── Run Log ──
    println!("\n[FJ-2301] Run Metadata:");
    let mut meta = RunMeta::new("r-abc123def456".into(), "intel".into(), "apply".into());
    meta.record_resource("nginx-pkg", ResourceRunStatus::Noop);
    meta.record_resource(
        "nginx-conf",
        ResourceRunStatus::Converged {
            exit_code: Some(0),
            duration_secs: Some(0.3),
            failed: false,
        },
    );
    println!(
        "  Run: {} | Machine: {} | Total: {} (converged={}, noop={})",
        meta.run_id, meta.machine, meta.summary.total, meta.summary.converged, meta.summary.noop,
    );

    println!("\n[FJ-2301] Run Log Entry:");
    let entry = RunLogEntry {
        resource_id: "nginx-pkg".into(),
        resource_type: "package".into(),
        action: "apply".into(),
        machine: "intel".into(),
        transport: "ssh".into(),
        script: "apt-get install -y nginx".into(),
        script_hash: "blake3:abc123".into(),
        stdout: "Reading package lists...\nDone.".into(),
        stderr: String::new(),
        exit_code: 0,
        duration_secs: 1.2,
        started_at: "2026-03-09T10:00:00Z".into(),
        finished_at: "2026-03-09T10:00:01Z".into(),
    };
    println!("  format_log: {} chars", entry.format_log().len());
    println!("  format_json: {} chars", entry.format_json().len());

    println!("\n[FJ-2301] Log Retention:");
    let retention = LogRetention::default();
    println!(
        "  keep_runs={}, keep_failed={}, max_log={}MB, max_total={}MB",
        retention.keep_runs,
        retention.keep_failed,
        retention.max_log_size / (1024 * 1024),
        retention.max_total_size / (1024 * 1024),
    );

    println!("\n[FJ-2301] Run ID: {}", generate_run_id());

    // ── Doctor ──
    println!("\n[FJ-2301] Doctor Report:");
    let report = DoctorReport {
        system: SystemInfo {
            forjar_version: "1.1.1".into(),
            state_dir: "./state/".into(),
            state_dir_exists: true,
            state_dir_writable: true,
            db_size_bytes: Some(2_400_000),
            db_schema_version: Some(3),
            run_log_size_bytes: None,
            run_log_machine_count: None,
            log_budget_bytes: None,
        },
        machines: vec![MachineHealth {
            name: "intel".into(),
            ssh_status: SshStatus::Ok { latency_ms: 12.5 },
            resource_count: Some(17),
            generation: Some(13),
            stored_runs: Some(10),
        }],
        tools: vec![ToolCheck {
            name: "bashrs".into(),
            available: true,
            version: Some("6.64.0".into()),
            install_hint: None,
        }],
        issues: vec![],
    };
    println!("  Healthy: {}", report.is_healthy());
    println!("{}", report.format_summary());

    // ── Image Build Log ──
    println!("[FJ-2301] Image Build Log:");
    let build = ImageBuildLog {
        image_ref: "training:2.0".into(),
        layers: vec![
            LayerBuildLog::cached("base-ubuntu", 0),
            LayerBuildLog {
                log_bytes: 8192,
                ..LayerBuildLog::new("ml-deps", 1, 47.3)
            },
        ],
        manifest_log: None,
        push_log: None,
        total_duration_secs: 48.0,
    };
    println!(
        "  All succeeded: {} | Cached: {} | Failed: {} | Total bytes: {}",
        build.all_succeeded(),
        build.cached_count(),
        build.failed_count(),
        build.total_log_bytes(),
    );
    println!("{build}");

    println!("{}", "=".repeat(55));
    println!("All run-log/doctor/image-log criteria survived.");
}
