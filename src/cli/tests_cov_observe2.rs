//! Coverage tests for cli/observe.rs — cmd_anomaly, cmd_trace.

fn make_event_converged(machine: &str, resource: &str) -> String {
    format!(
        r#"{{"ts":"2026-01-01T00:00:00Z","event":"resource_converged","machine":"{machine}","resource":"{resource}","duration_seconds":1.5,"hash":"blake3:abc"}}"#
    )
}

fn make_event_failed(machine: &str, resource: &str) -> String {
    format!(
        r#"{{"ts":"2026-01-01T00:01:00Z","event":"resource_failed","machine":"{machine}","resource":"{resource}","error":"exit code 1"}}"#
    )
}

fn make_event_drift(machine: &str, resource: &str) -> String {
    format!(
        r#"{{"ts":"2026-01-01T00:02:00Z","event":"drift_detected","machine":"{machine}","resource":"{resource}","expected":"hash1","actual":"hash2"}}"#
    )
}

fn setup_events(dir: &std::path::Path, machine: &str, events: &[String]) {
    let mdir = dir.join(machine);
    std::fs::create_dir_all(&mdir).unwrap();
    let content = events.join("\n") + "\n";
    std::fs::write(mdir.join("events.jsonl"), content).unwrap();
}

fn make_trace_span(machine: &str, name: &str, clock: u64) -> String {
    format!(
        r#"{{"trace_id":"t001","span_id":"s{clock:03}","parent_span_id":null,"name":"{name}","start_time":"2026-01-01T00:00:00Z","duration_us":1500000,"exit_code":0,"resource_type":"package","action":"converge","content_hash":"abc","logical_clock":{clock},"machine":"{machine}"}}"#
    )
}

fn setup_traces(dir: &std::path::Path, machine: &str, spans: &[String]) {
    let mdir = dir.join(machine);
    std::fs::create_dir_all(&mdir).unwrap();
    let content = spans.join("\n") + "\n";
    std::fs::write(mdir.join("trace.jsonl"), content).unwrap();
}

// ── cmd_anomaly ──

#[test]
fn anomaly_empty_dir() {
    let d = tempfile::tempdir().unwrap();
    let r = super::observe::cmd_anomaly(d.path(), None, 3, false);
    assert!(r.is_ok());
}

#[test]
fn anomaly_empty_dir_json() {
    let d = tempfile::tempdir().unwrap();
    let r = super::observe::cmd_anomaly(d.path(), None, 3, true);
    assert!(r.is_ok());
}

#[test]
fn anomaly_missing_dir() {
    let r = super::observe::cmd_anomaly(std::path::Path::new("/nonexistent/state"), None, 3, false);
    assert!(r.is_err());
}

#[test]
fn anomaly_no_events_file() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("web1")).unwrap();
    let r = super::observe::cmd_anomaly(d.path(), None, 3, false);
    assert!(r.is_ok());
}

#[test]
fn anomaly_with_converged_events() {
    let d = tempfile::tempdir().unwrap();
    let events: Vec<String> = (0..5)
        .map(|i| make_event_converged("web1", &format!("pkg{i}")))
        .collect();
    setup_events(d.path(), "web1", &events);
    let r = super::observe::cmd_anomaly(d.path(), None, 1, false);
    assert!(r.is_ok());
}

#[test]
fn anomaly_with_converged_events_json() {
    let d = tempfile::tempdir().unwrap();
    let events: Vec<String> = (0..5)
        .map(|i| make_event_converged("web1", &format!("pkg{i}")))
        .collect();
    setup_events(d.path(), "web1", &events);
    let r = super::observe::cmd_anomaly(d.path(), None, 1, true);
    assert!(r.is_ok());
}

#[test]
fn anomaly_with_failures() {
    let d = tempfile::tempdir().unwrap();
    let mut events = Vec::new();
    for _ in 0..10 {
        events.push(make_event_failed("web1", "bad-pkg"));
    }
    events.push(make_event_converged("web1", "good-pkg"));
    setup_events(d.path(), "web1", &events);
    let r = super::observe::cmd_anomaly(d.path(), None, 1, false);
    assert!(r.is_ok());
}

#[test]
fn anomaly_with_drift() {
    let d = tempfile::tempdir().unwrap();
    let mut events = Vec::new();
    for _ in 0..10 {
        events.push(make_event_drift("web1", "drifty"));
    }
    events.push(make_event_converged("web1", "stable"));
    setup_events(d.path(), "web1", &events);
    let r = super::observe::cmd_anomaly(d.path(), None, 1, false);
    assert!(r.is_ok());
}

#[test]
fn anomaly_machine_filter() {
    let d = tempfile::tempdir().unwrap();
    setup_events(d.path(), "web1", &[make_event_converged("web1", "nginx")]);
    setup_events(d.path(), "db1", &[make_event_failed("db1", "pg")]);
    let r = super::observe::cmd_anomaly(d.path(), Some("web1"), 1, false);
    assert!(r.is_ok());
}

#[test]
fn anomaly_machine_filter_no_match() {
    let d = tempfile::tempdir().unwrap();
    setup_events(d.path(), "web1", &[make_event_converged("web1", "nginx")]);
    let r = super::observe::cmd_anomaly(d.path(), Some("zzz"), 1, false);
    assert!(r.is_ok());
}

#[test]
fn anomaly_high_min_events() {
    let d = tempfile::tempdir().unwrap();
    setup_events(d.path(), "web1", &[make_event_converged("web1", "nginx")]);
    let r = super::observe::cmd_anomaly(d.path(), None, 100, false);
    assert!(r.is_ok());
}

#[test]
fn anomaly_mixed_events_json() {
    let d = tempfile::tempdir().unwrap();
    let events = vec![
        make_event_converged("web1", "nginx"),
        make_event_failed("web1", "nginx"),
        make_event_drift("web1", "nginx"),
        make_event_converged("web1", "nginx"),
        make_event_failed("web1", "nginx"),
    ];
    setup_events(d.path(), "web1", &events);
    let r = super::observe::cmd_anomaly(d.path(), None, 1, true);
    assert!(r.is_ok());
}

#[test]
fn anomaly_multi_machine() {
    let d = tempfile::tempdir().unwrap();
    setup_events(d.path(), "web1", &[
        make_event_converged("web1", "nginx"),
        make_event_converged("web1", "nginx"),
    ]);
    setup_events(d.path(), "db1", &[
        make_event_failed("db1", "pg"),
        make_event_failed("db1", "pg"),
        make_event_failed("db1", "pg"),
    ]);
    let r = super::observe::cmd_anomaly(d.path(), None, 1, false);
    assert!(r.is_ok());
}

// ── cmd_trace ──

#[test]
fn trace_empty_dir() {
    let d = tempfile::tempdir().unwrap();
    let r = super::observe::cmd_trace(d.path(), None, false);
    assert!(r.is_ok());
}

#[test]
fn trace_empty_dir_json() {
    let d = tempfile::tempdir().unwrap();
    let r = super::observe::cmd_trace(d.path(), None, true);
    assert!(r.is_ok());
}

#[test]
fn trace_missing_dir() {
    let r = super::observe::cmd_trace(std::path::Path::new("/nonexistent/state"), None, false);
    assert!(r.is_err());
}

#[test]
fn trace_no_trace_file() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("web1")).unwrap();
    let r = super::observe::cmd_trace(d.path(), None, false);
    assert!(r.is_ok());
}

#[test]
fn trace_with_spans() {
    let d = tempfile::tempdir().unwrap();
    let spans = vec![
        make_trace_span("web1", "apply:nginx", 1),
        make_trace_span("web1", "apply:config", 2),
    ];
    setup_traces(d.path(), "web1", &spans);
    let r = super::observe::cmd_trace(d.path(), None, false);
    assert!(r.is_ok());
}

#[test]
fn trace_with_spans_json() {
    let d = tempfile::tempdir().unwrap();
    let spans = vec![
        make_trace_span("web1", "apply:nginx", 1),
        make_trace_span("web1", "apply:config", 2),
    ];
    setup_traces(d.path(), "web1", &spans);
    let r = super::observe::cmd_trace(d.path(), None, true);
    assert!(r.is_ok());
}

#[test]
fn trace_machine_filter() {
    let d = tempfile::tempdir().unwrap();
    setup_traces(d.path(), "web1", &[make_trace_span("web1", "apply:nginx", 1)]);
    setup_traces(d.path(), "db1", &[make_trace_span("db1", "apply:pg", 2)]);
    let r = super::observe::cmd_trace(d.path(), Some("web1"), false);
    assert!(r.is_ok());
}

#[test]
fn trace_machine_filter_no_match() {
    let d = tempfile::tempdir().unwrap();
    setup_traces(d.path(), "web1", &[make_trace_span("web1", "apply:nginx", 1)]);
    let r = super::observe::cmd_trace(d.path(), Some("zzz"), false);
    assert!(r.is_ok());
}

#[test]
fn trace_multi_machine() {
    let d = tempfile::tempdir().unwrap();
    setup_traces(d.path(), "web1", &[
        make_trace_span("web1", "apply:nginx", 1),
        make_trace_span("web1", "apply:config", 3),
    ]);
    setup_traces(d.path(), "db1", &[
        make_trace_span("db1", "apply:pg", 2),
    ]);
    let r = super::observe::cmd_trace(d.path(), None, false);
    assert!(r.is_ok());
}

#[test]
fn trace_multi_machine_json() {
    let d = tempfile::tempdir().unwrap();
    setup_traces(d.path(), "web1", &[make_trace_span("web1", "apply:nginx", 1)]);
    setup_traces(d.path(), "db1", &[make_trace_span("db1", "apply:pg", 2)]);
    let r = super::observe::cmd_trace(d.path(), None, true);
    assert!(r.is_ok());
}
