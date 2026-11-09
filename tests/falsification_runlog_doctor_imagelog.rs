//! FJ-2301: Run log, doctor diagnostics, image build log types.
//! Usage: cargo test --test falsification_runlog_doctor_imagelog

use forjar::core::types::*;

// ── helpers ──

fn entry(rid: &str, rtype: &str, exit: i32) -> RunLogEntry {
    RunLogEntry {
        resource_id: rid.into(),
        resource_type: rtype.into(),
        action: "apply".into(),
        machine: "web".into(),
        transport: "ssh".into(),
        script: "echo ok".into(),
        script_hash: "blake3:abc".into(),
        stdout: "ok\n".into(),
        stderr: String::new(),
        exit_code: exit,
        duration_secs: 0.5,
        started_at: "2026-03-09T10:00:00Z".into(),
        finished_at: "2026-03-09T10:00:01Z".into(),
    }
}

fn sysinfo() -> SystemInfo {
    SystemInfo {
        forjar_version: "1.1.1".into(),
        state_dir: "./state/".into(),
        state_dir_exists: true,
        state_dir_writable: true,
        db_size_bytes: Some(2_400_000),
        db_schema_version: Some(3),
        run_log_size_bytes: Some(49_000_000),
        run_log_machine_count: Some(3),
        log_budget_bytes: Some(500_000_000),
    }
}

fn issue(sev: IssueSeverity, msg: &str) -> DoctorIssue {
    DoctorIssue {
        severity: sev,
        message: msg.into(),
        fix_hint: None,
    }
}

fn mh(name: &str, ssh: SshStatus) -> MachineHealth {
    MachineHealth {
        name: name.into(),
        ssh_status: ssh,
        resource_count: Some(5),
        generation: Some(1),
        stored_runs: None,
    }
}

fn tool(name: &str, avail: bool) -> ToolCheck {
    ToolCheck {
        name: name.into(),
        available: avail,
        version: if avail { Some("1.0".into()) } else { None },
        install_hint: None,
    }
}

fn report(issues: Vec<DoctorIssue>) -> DoctorReport {
    DoctorReport {
        system: sysinfo(),
        machines: vec![mh("intel", SshStatus::Ok { latency_ms: 12.0 })],
        tools: vec![tool("bashrs", true)],
        issues,
    }
}

fn layer(name: &str, idx: u32, bytes: u64) -> LayerBuildLog {
    LayerBuildLog {
        log_bytes: bytes,
        ..LayerBuildLog::new(name, idx, 1.0)
    }
}

fn iblog(layers: Vec<LayerBuildLog>) -> ImageBuildLog {
    ImageBuildLog {
        image_ref: "app:1.0".into(),
        layers,
        manifest_log: None,
        push_log: None,
        total_duration_secs: 5.0,
    }
}

// ── FJ-2301: RunMeta ──

#[test]
fn run_meta_new() {
    let m = RunMeta::new("r-abc".into(), "intel".into(), "apply".into());
    assert_eq!(m.run_id, "r-abc");
    assert_eq!(m.command, "apply");
    assert!(m.resources.is_empty());
    assert_eq!(m.summary.total, 0);
}

#[test]
fn run_meta_record_resources() {
    let mut m = RunMeta::new("r-1".into(), "web".into(), "apply".into());
    m.record_resource("pkg", ResourceRunStatus::Noop);
    m.record_resource(
        "file",
        ResourceRunStatus::Converged {
            exit_code: Some(0),
            duration_secs: Some(0.5),
            failed: false,
        },
    );
    m.record_resource(
        "svc",
        ResourceRunStatus::Converged {
            exit_code: Some(1),
            duration_secs: Some(0.3),
            failed: true,
        },
    );
    m.record_resource(
        "cron",
        ResourceRunStatus::Skipped {
            reason: Some("dep failed".into()),
        },
    );
    assert_eq!(m.summary.total, 4);
    assert_eq!(m.summary.noop, 1);
    assert_eq!(m.summary.converged, 1);
    assert_eq!(m.summary.failed, 1);
    assert_eq!(m.summary.skipped, 1);
    assert_eq!(m.resources.len(), 4);
}

#[test]
fn run_meta_serde() {
    let m = RunMeta::new("r-x".into(), "m".into(), "destroy".into());
    let json = serde_json::to_string(&m).unwrap();
    let parsed: RunMeta = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.run_id, "r-x");
    assert_eq!(parsed.command, "destroy");
}

// ── FJ-2301: RunLogEntry ──

#[test]
fn run_log_entry_format_log() {
    let e = entry("nginx-pkg", "package", 0);
    let log = e.format_log();
    assert!(log.contains("=== FORJAR TRANSPORT LOG ==="));
    assert!(log.contains("resource: nginx-pkg"));
    assert!(log.contains("type: package"));
    assert!(log.contains("=== SCRIPT ==="));
    assert!(log.contains("=== STDOUT ==="));
    assert!(log.contains("=== STDERR ==="));
    assert!(log.contains("exit_code: 0"));
}

#[test]
fn run_log_entry_display_uses_format_log() {
    let e = entry("pkg", "package", 0);
    assert_eq!(e.to_string(), e.format_log());
}

#[test]
fn run_log_entry_format_json() {
    let e = entry("pkg", "package", 0);
    let json = e.format_json();
    assert!(json.contains("\"resource_id\":\"pkg\""));
    assert!(json.contains("\"exit_code\":0"));
}

#[test]
fn run_log_entry_format_json_pretty() {
    let e = entry("pkg", "package", 0);
    let pretty = e.format_json_pretty();
    assert!(pretty.contains("resource_id"));
    assert!(pretty.contains('\n')); // pretty-printed has newlines
}

#[test]
fn run_log_entry_serde_roundtrip() {
    let e = entry("svc", "service", 1);
    let json = serde_json::to_string(&e).unwrap();
    let parsed: RunLogEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.resource_id, "svc");
    assert_eq!(parsed.exit_code, 1);
}

// ── FJ-2301: ResourceRunStatus serde ──

#[test]
fn resource_run_status_serde() {
    let noop = ResourceRunStatus::Noop;
    let json = serde_json::to_string(&noop).unwrap();
    assert!(json.contains("\"action\":\"noop\""));
    let conv = ResourceRunStatus::Converged {
        exit_code: Some(0),
        duration_secs: Some(1.2),
        failed: false,
    };
    let json = serde_json::to_string(&conv).unwrap();
    let parsed: ResourceRunStatus = serde_json::from_str(&json).unwrap();
    matches!(parsed, ResourceRunStatus::Converged { failed: false, .. });
}

// ── FJ-2301: LogRetention ──

#[test]
fn log_retention_default() {
    let d = LogRetention::default();
    assert_eq!(d.keep_runs, 10);
    assert_eq!(d.keep_failed, 50);
    assert_eq!(d.max_log_size, 10 * 1024 * 1024);
    assert_eq!(d.max_total_size, 500 * 1024 * 1024);
}

// ── FJ-2301: generate_run_id ──

#[test]
fn generate_run_id_format_and_unique() {
    let id1 = generate_run_id();
    let id2 = generate_run_id();
    assert!(id1.starts_with("r-"));
    assert_eq!(id1.len(), 14); // "r-" + 12 hex
    assert_ne!(id1, id2);
}

// ── FJ-2301: DoctorReport ──

#[test]
fn doctor_healthy_no_issues() {
    let r = report(vec![]);
    assert!(r.is_healthy());
    assert_eq!(r.issue_counts(), (0, 0, 0));
}

#[test]
fn doctor_warning_still_healthy() {
    let r = report(vec![issue(IssueSeverity::Warning, "slow")]);
    assert!(r.is_healthy());
    assert_eq!(r.issue_counts(), (0, 1, 0));
}

#[test]
fn doctor_error_unhealthy() {
    let r = report(vec![issue(IssueSeverity::Error, "broken")]);
    assert!(!r.is_healthy());
    assert_eq!(r.issue_counts(), (1, 0, 0));
}

#[test]
fn doctor_mixed_issues() {
    let r = report(vec![
        issue(IssueSeverity::Error, "e"),
        issue(IssueSeverity::Warning, "w"),
        issue(IssueSeverity::Info, "i"),
        issue(IssueSeverity::Info, "i2"),
    ]);
    assert!(!r.is_healthy());
    assert_eq!(r.issue_counts(), (1, 1, 2));
}

#[test]
fn doctor_format_summary() {
    let r = report(vec![issue(IssueSeverity::Warning, "high latency")]);
    let s = r.format_summary();
    assert!(s.contains("forjar version: 1.1.1"));
    assert!(s.contains("exists, writable"));
    assert!(s.contains("intel: SSH OK"));
    assert!(s.contains("bashrs"));
    assert!(s.contains("WARNING: high latency"));
}

#[test]
fn doctor_format_missing_state() {
    let mut r = report(vec![]);
    r.system.state_dir_exists = false;
    assert!(r.format_summary().contains("MISSING"));
}

#[test]
fn doctor_format_not_writable() {
    let mut r = report(vec![]);
    r.system.state_dir_writable = false;
    assert!(r.format_summary().contains("NOT writable"));
}

// ── FJ-2301: SshStatus serde ──

#[test]
fn ssh_status_serde_all_variants() {
    for status in [
        SshStatus::Ok { latency_ms: 42.5 },
        SshStatus::Failed {
            error: "timeout".into(),
        },
        SshStatus::Local,
        SshStatus::Container,
    ] {
        let json = serde_json::to_string(&status).unwrap();
        let _: SshStatus = serde_json::from_str(&json).unwrap();
    }
}

// ── FJ-2301: MachineHealth, ToolCheck serde ──

#[test]
fn machine_health_serde() {
    let m = mh("test", SshStatus::Container);
    let json = serde_json::to_string(&m).unwrap();
    let parsed: MachineHealth = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.name, "test");
}

#[test]
fn tool_check_serde() {
    let t = tool("docker", false);
    let json = serde_json::to_string(&t).unwrap();
    let parsed: ToolCheck = serde_json::from_str(&json).unwrap();
    assert!(!parsed.available);
}

// ── FJ-2301: LayerBuildLog ──

#[test]
fn layer_new_and_cached() {
    let l = LayerBuildLog::new("deps", 0, 5.0);
    assert!(!l.cached);
    assert!(l.succeeded());
    let c = LayerBuildLog::cached("base", 0);
    assert!(c.cached);
    assert!(c.succeeded());
}

#[test]
fn layer_failed() {
    let l = LayerBuildLog {
        exit_code: Some(1),
        ..LayerBuildLog::new("build", 1, 30.0)
    };
    assert!(!l.succeeded());
    assert!(l.to_string().contains("FAIL"));
}

#[test]
fn layer_display() {
    let l = layer("ml-deps", 2, 4096);
    let s = l.to_string();
    assert!(s.contains("ml-deps"));
    assert!(s.contains("4096 bytes"));
    let c = LayerBuildLog::cached("base", 0);
    assert!(c.to_string().contains("cached"));
}

#[test]
fn layer_default_log_path() {
    assert_eq!(
        LayerBuildLog::default_log_path("img", 3),
        "state/builds/img/layer-3.log"
    );
}

// ── FJ-2301: ImageBuildLog ──

#[test]
fn image_build_log_all_succeeded() {
    let log = iblog(vec![
        LayerBuildLog::cached("base", 0),
        LayerBuildLog::new("code", 1, 2.0),
    ]);
    assert!(log.all_succeeded());
    assert_eq!(log.cached_count(), 1);
    assert_eq!(log.failed_count(), 0);
}

#[test]
fn image_build_log_with_failure() {
    let log = iblog(vec![
        LayerBuildLog::cached("base", 0),
        LayerBuildLog {
            exit_code: Some(1),
            ..LayerBuildLog::new("build", 1, 10.0)
        },
    ]);
    assert!(!log.all_succeeded());
    assert_eq!(log.failed_count(), 1);
}

#[test]
fn image_build_log_total_bytes() {
    let log = iblog(vec![layer("a", 0, 1000), layer("b", 1, 2000)]);
    assert_eq!(log.total_log_bytes(), 3000);
}

#[test]
fn image_build_log_display() {
    let log = iblog(vec![LayerBuildLog::cached("base", 0)]);
    let s = log.to_string();
    assert!(s.contains("app:1.0"));
    assert!(s.contains("5.0s"));
}

#[test]
fn image_build_log_build_dir() {
    assert_eq!(ImageBuildLog::build_dir("my-app"), "state/builds/my-app");
}

#[test]
fn image_build_log_default() {
    let d = ImageBuildLog::default();
    assert!(d.layers.is_empty());
    assert!(d.all_succeeded());
    assert_eq!(d.total_log_bytes(), 0);
}

#[test]
fn image_build_log_serde() {
    let log = iblog(vec![LayerBuildLog::new("code", 0, 1.0)]);
    let json = serde_json::to_string(&log).unwrap();
    let parsed: ImageBuildLog = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.layers.len(), 1);
}
