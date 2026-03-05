//! FJ-2301: Doctor diagnostics — system health checks and reporting.
//!
//! ```bash
//! cargo run --example doctor_diagnostics
//! ```

use forjar::core::types::{
    DoctorIssue, DoctorReport, IssueSeverity, MachineHealth, SshStatus, SystemInfo, ToolCheck,
};

fn main() {
    // Build a complete doctor report
    let report = DoctorReport {
        system: SystemInfo {
            forjar_version: env!("CARGO_PKG_VERSION").into(),
            state_dir: "./state/".into(),
            state_dir_exists: true,
            state_dir_writable: true,
            db_size_bytes: Some(2_400_000),
            db_schema_version: Some(3),
            run_log_size_bytes: Some(49_000_000),
            run_log_machine_count: Some(3),
            log_budget_bytes: Some(500_000_000),
        },
        machines: vec![
            MachineHealth {
                name: "intel".into(),
                ssh_status: SshStatus::Ok { latency_ms: 12.5 },
                resource_count: Some(17),
                generation: Some(13),
                stored_runs: Some(10),
            },
            MachineHealth {
                name: "jetson".into(),
                ssh_status: SshStatus::Ok { latency_ms: 34.2 },
                resource_count: Some(8),
                generation: Some(5),
                stored_runs: Some(4),
            },
            MachineHealth {
                name: "lambda".into(),
                ssh_status: SshStatus::Failed {
                    error: "Connection refused (10.0.2.50:22)".into(),
                },
                resource_count: Some(7),
                generation: Some(3),
                stored_runs: Some(2),
            },
        ],
        tools: vec![
            ToolCheck {
                name: "bashrs".into(),
                available: true,
                version: Some("6.64.0".into()),
                install_hint: None,
            },
            ToolCheck {
                name: "blake3".into(),
                available: true,
                version: None,
                install_hint: None,
            },
            ToolCheck {
                name: "docker".into(),
                available: true,
                version: Some("24.0.7".into()),
                install_hint: None,
            },
            ToolCheck {
                name: "pepita".into(),
                available: false,
                version: None,
                install_hint: Some("cargo install pepita".into()),
            },
        ],
        issues: vec![
            DoctorIssue {
                severity: IssueSeverity::Warning,
                message: "lambda is unreachable — apply will skip lambda resources".into(),
                fix_hint: Some("check SSH connectivity: ssh lambda".into()),
            },
            DoctorIssue {
                severity: IssueSeverity::Info,
                message: "2 failed runs in last 7 days (forjar logs --failed)".into(),
                fix_hint: None,
            },
        ],
    };

    // Print the formatted summary
    print!("{}", report.format_summary());

    // Health check
    println!();
    if report.is_healthy() {
        println!("Overall: HEALTHY (warnings are non-blocking)");
    } else {
        println!("Overall: UNHEALTHY (errors detected)");
    }

    let (errors, warnings, info) = report.issue_counts();
    println!("Issues: {errors} errors, {warnings} warnings, {info} info");

    // JSON export for CI
    println!();
    println!("=== JSON Export ===");
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
