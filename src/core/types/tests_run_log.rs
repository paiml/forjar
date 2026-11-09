//! Tests for run_log_types.rs — run metadata, log entries, retention policy.

use super::run_log_types::*;

#[test]
fn run_meta_new() {
    let meta = RunMeta::new("r-123".into(), "web".into(), "apply".into());
    assert_eq!(meta.run_id, "r-123");
    assert_eq!(meta.machine, "web");
    assert_eq!(meta.command, "apply");
    assert!(meta.resources.is_empty());
    assert_eq!(meta.summary.total, 0);
}

#[test]
fn run_meta_record_resources() {
    let mut meta = RunMeta::new("r-1".into(), "m".into(), "apply".into());
    meta.record_resource("a", ResourceRunStatus::Noop);
    meta.record_resource(
        "b",
        ResourceRunStatus::Converged {
            exit_code: Some(0),
            duration_secs: Some(1.5),
            failed: false,
        },
    );
    meta.record_resource(
        "c",
        ResourceRunStatus::Converged {
            exit_code: Some(1),
            duration_secs: Some(0.3),
            failed: true,
        },
    );
    meta.record_resource(
        "d",
        ResourceRunStatus::Skipped {
            reason: Some("dep failed".into()),
        },
    );

    assert_eq!(meta.summary.total, 4);
    assert_eq!(meta.summary.noop, 1);
    assert_eq!(meta.summary.converged, 1);
    assert_eq!(meta.summary.failed, 1);
    assert_eq!(meta.summary.skipped, 1);
    assert_eq!(meta.resources.len(), 4);
}

#[test]
fn run_meta_serde_roundtrip() {
    let mut meta = RunMeta::new("r-abc".into(), "intel".into(), "destroy".into());
    meta.generation = Some(5);
    meta.operator = Some("noah@host".into());
    meta.record_resource("pkg", ResourceRunStatus::Noop);
    let yaml = serde_yaml_ng::to_string(&meta).unwrap();
    let parsed: RunMeta = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(parsed.run_id, "r-abc");
    assert_eq!(parsed.generation, Some(5));
    assert_eq!(parsed.summary.noop, 1);
}

#[test]
fn run_log_entry_format() {
    let entry = RunLogEntry {
        resource_id: "nginx".into(),
        resource_type: "package".into(),
        action: "apply".into(),
        machine: "web-1".into(),
        transport: "ssh".into(),
        script: "apt-get install -y nginx".into(),
        script_hash: "blake3:deadbeef".into(),
        stdout: "installed ok".into(),
        stderr: String::new(),
        exit_code: 0,
        duration_secs: 1.234,
        started_at: "2026-03-05T14:30:00Z".into(),
        finished_at: "2026-03-05T14:30:01Z".into(),
    };
    let log = entry.format_log();
    assert!(log.contains("=== FORJAR TRANSPORT LOG ==="));
    assert!(log.contains("resource: nginx"));
    assert!(log.contains("=== SCRIPT ==="));
    assert!(log.contains("apt-get install -y nginx"));
    assert!(log.contains("=== STDOUT ==="));
    assert!(log.contains("installed ok"));
    assert!(log.contains("=== STDERR ==="));
    assert!(log.contains("=== RESULT ==="));
    assert!(log.contains("exit_code: 0"));
    assert!(log.contains("duration_secs: 1.234"));
}

#[test]
fn run_log_entry_display() {
    let entry = RunLogEntry {
        resource_id: "pkg".into(),
        resource_type: "package".into(),
        action: "check".into(),
        machine: "m".into(),
        transport: "local".into(),
        script: "echo test".into(),
        script_hash: "blake3:abc".into(),
        stdout: "test\n".into(),
        stderr: "warn\n".into(),
        exit_code: 1,
        duration_secs: 0.5,
        started_at: "t0".into(),
        finished_at: "t1".into(),
    };
    let display = format!("{entry}");
    assert!(display.contains("exit_code: 1"));
}

#[test]
fn run_log_serde_roundtrip() {
    let entry = RunLogEntry {
        resource_id: "svc".into(),
        resource_type: "service".into(),
        action: "destroy".into(),
        machine: "host".into(),
        transport: "ssh".into(),
        script: "systemctl stop svc".into(),
        script_hash: "blake3:fff".into(),
        stdout: "stopped".into(),
        stderr: String::new(),
        exit_code: 0,
        duration_secs: 0.1,
        started_at: "s".into(),
        finished_at: "f".into(),
    };
    let json = serde_json::to_string(&entry).unwrap();
    let parsed: RunLogEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.resource_id, "svc");
    assert_eq!(parsed.exit_code, 0);
}

#[test]
fn log_retention_defaults() {
    let ret = LogRetention::default();
    assert_eq!(ret.keep_runs, 10);
    assert_eq!(ret.keep_failed, 50);
    assert_eq!(ret.max_log_size, 10 * 1024 * 1024);
    assert_eq!(ret.max_total_size, 500 * 1024 * 1024);
}

#[test]
fn log_retention_serde() {
    let yaml = r#"
keep_runs: 5
keep_failed: 20
max_log_size: 1048576
max_total_size: 104857600
"#;
    let ret: LogRetention = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(ret.keep_runs, 5);
    assert_eq!(ret.keep_failed, 20);
}

#[test]
fn generate_run_id_format() {
    let id = generate_run_id();
    assert!(id.starts_with("r-"));
    assert!(id.len() >= 3);
}

#[test]
fn generate_run_id_unique() {
    let a = generate_run_id();
    std::thread::sleep(std::time::Duration::from_millis(1));
    let b = generate_run_id();
    assert_ne!(a, b);
}

#[test]
fn resource_run_status_variants() {
    let noop = ResourceRunStatus::Noop;
    let yaml = serde_yaml_ng::to_string(&noop).unwrap();
    assert!(yaml.contains("noop"));

    let converged = ResourceRunStatus::Converged {
        exit_code: Some(0),
        duration_secs: Some(2.5),
        failed: false,
    };
    let yaml = serde_yaml_ng::to_string(&converged).unwrap();
    assert!(yaml.contains("converged"));

    let skipped = ResourceRunStatus::Skipped {
        reason: Some("dep failed".into()),
    };
    let yaml = serde_yaml_ng::to_string(&skipped).unwrap();
    assert!(yaml.contains("skipped"));
}

#[test]
fn run_summary_default() {
    let summary = RunSummary::default();
    assert_eq!(summary.total, 0);
    assert_eq!(summary.converged, 0);
    assert_eq!(summary.noop, 0);
    assert_eq!(summary.failed, 0);
    assert_eq!(summary.skipped, 0);
}

#[test]
fn run_log_empty_stdout_stderr() {
    let entry = RunLogEntry {
        resource_id: "r".into(),
        resource_type: "task".into(),
        action: "apply".into(),
        machine: "m".into(),
        transport: "local".into(),
        script: "true".into(),
        script_hash: "blake3:0".into(),
        stdout: String::new(),
        stderr: String::new(),
        exit_code: 0,
        duration_secs: 0.001,
        started_at: "s".into(),
        finished_at: "f".into(),
    };
    let log = entry.format_log();
    assert!(log.contains("=== STDOUT ===\n\n=== STDERR ==="));
}
