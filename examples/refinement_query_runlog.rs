//! FJ-043/2001/2301: Refinement types, query health, and run log formatting.
//!
//! Demonstrates:
//! - Refinement type validation (Port, FileMode, SemVer, Hostname, AbsPath, ResourceName)
//! - HealthSummary aggregation and table formatting
//! - TimingStats percentile calculation
//! - RunMeta resource accounting and RunLogEntry formatting
//!
//! Usage: cargo run --example refinement_query_runlog

use forjar::core::types::refinement::{AbsPath, FileMode, Hostname, Port, ResourceName, SemVer};
use forjar::core::types::{
    ChurnMetric, HealthSummary, MachineHealthRow, ResourceRunStatus, RunLogEntry, RunMeta,
    TimingStats,
};

fn main() {
    println!("Forjar: Refinement Types, Query Health & Run Logs");
    println!("{}", "=".repeat(55));

    // ── Refinement Types ──
    println!("\n[FJ-043] Refinement Types:");

    let port = Port::new(8080).unwrap();
    println!("  Port: {} (valid)", port.value());
    println!("  Port 0: {}", Port::new(0).unwrap_err());

    let mode = FileMode::new(0o644).unwrap();
    println!(
        "  FileMode: {} (raw {})",
        mode.as_octal_string(),
        mode.value()
    );
    let mode_str = FileMode::from_str("755").unwrap();
    println!("  FileMode from \"755\": {}", mode_str.as_octal_string());

    let ver = SemVer::parse("2.1.0").unwrap();
    println!("  SemVer: {ver}");
    println!("  SemVer \"1.2\": {}", SemVer::parse("1.2").unwrap_err());

    let host = Hostname::new("web-01.example.com").unwrap();
    println!("  Hostname: {}", host.as_str());
    println!(
        "  Hostname \"-bad\": {}",
        Hostname::new("-bad.com").unwrap_err()
    );

    let path = AbsPath::new("/etc/nginx/nginx.conf").unwrap();
    println!("  AbsPath: {}", path.as_str());

    let name = ResourceName::new("pkg-nginx").unwrap();
    println!("  ResourceName: {}", name.as_str());

    // ── Health Summary ──
    println!("\n[FJ-2001] Health Summary:");
    let health = HealthSummary {
        machines: vec![
            MachineHealthRow {
                name: "intel".into(),
                total: 17,
                converged: 15,
                drifted: 1,
                failed: 1,
                last_apply: "2026-03-09T12:00:00Z".into(),
                generation: 12,
            },
            MachineHealthRow {
                name: "jetson".into(),
                total: 8,
                converged: 8,
                drifted: 0,
                failed: 0,
                last_apply: "2026-03-09T12:05:00Z".into(),
                generation: 5,
            },
        ],
    };
    println!("{}", health.format_table());
    assert_eq!(health.total_resources(), 25);
    assert!((health.stack_health_pct() - 92.0).abs() < 0.1);

    // ── Timing Stats ──
    println!("[FJ-2001] Timing Stats:");
    let durations = vec![0.1, 0.2, 0.3, 0.5, 0.8, 1.0, 1.5, 2.0, 3.0, 5.0];
    let stats = TimingStats::from_sorted(&durations).unwrap();
    println!("  {}", stats.format_compact());
    assert_eq!(stats.count, 10);

    // ── Churn Metric ──
    println!("\n[FJ-2001] Churn Metric:");
    let churn = ChurnMetric {
        resource_id: "bash-aliases".into(),
        changed_gens: 4,
        total_gens: 12,
    };
    println!("  {}: {:.1}% churn", churn.resource_id, churn.churn_pct());

    // ── Run Meta ──
    println!("\n[FJ-2301] Run Meta:");
    let mut meta = RunMeta::new("r-abc123".into(), "intel".into(), "apply".into());
    meta.record_resource(
        "pkg-nginx",
        ResourceRunStatus::Converged {
            exit_code: Some(0),
            duration_secs: Some(1.5),
            failed: false,
        },
    );
    meta.record_resource("bash-aliases", ResourceRunStatus::Noop);
    meta.record_resource(
        "svc-broken",
        ResourceRunStatus::Converged {
            exit_code: Some(1),
            duration_secs: Some(0.3),
            failed: true,
        },
    );
    println!(
        "  total={} converged={} noop={} failed={}",
        meta.summary.total, meta.summary.converged, meta.summary.noop, meta.summary.failed,
    );

    // ── Run Log Entry ──
    println!("\n[FJ-2301] Run Log Entry:");
    let entry = RunLogEntry {
        resource_id: "pkg-nginx".into(),
        resource_type: "package".into(),
        action: "apply".into(),
        machine: "web-1".into(),
        transport: "ssh".into(),
        script: "apt-get install -y nginx".into(),
        script_hash: "blake3:abc123".into(),
        stdout: "Reading package lists...".into(),
        stderr: String::new(),
        exit_code: 0,
        duration_secs: 1.234,
        started_at: "2026-03-09T14:30:00Z".into(),
        finished_at: "2026-03-09T14:30:01Z".into(),
    };
    let log = entry.format_log();
    let lines: Vec<&str> = log.lines().collect();
    println!("  (log has {} lines, first: {})", lines.len(), lines[0]);
    assert!(log.contains("=== FORJAR TRANSPORT LOG ==="));
    assert!(log.contains("exit_code: 0"));

    println!("\n{}", "=".repeat(55));
    println!("All refinement/query/runlog criteria survived.");
}
