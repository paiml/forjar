//! Coverage tests for observe.rs — cmd_trace text/json, cmd_anomaly findings, handle_watch_change.

use std::path::Path;

fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
    let file = dir.join("forjar.yaml");
    std::fs::write(&file, yaml).unwrap();
    file
}

fn write_trace(state_dir: &Path, machine: &str, spans_jsonl: &str) {
    let machine_dir = state_dir.join(machine);
    std::fs::create_dir_all(&machine_dir).unwrap();
    std::fs::write(machine_dir.join("trace.jsonl"), spans_jsonl).unwrap();
}

fn make_trace_span(
    trace_id: &str,
    name: &str,
    duration_us: u64,
    exit_code: i32,
    logical_clock: u64,
) -> String {
    serde_json::json!({
        "trace_id": trace_id,
        "span_id": format!("span-{logical_clock}"),
        "parent_span_id": null,
        "name": name,
        "start_time": "2026-03-08T10:00:00Z",
        "duration_us": duration_us,
        "exit_code": exit_code,
        "resource_type": "file",
        "action": "create",
        "content_hash": "abc123",
        "logical_clock": logical_clock,
    })
    .to_string()
}

// ── cmd_trace empty ─────────────────────────────────────────────────

#[test]
fn trace_empty_state_text() {
    let state_dir = tempfile::tempdir().unwrap();
    let result = super::observe::cmd_trace(state_dir.path(), None, false);
    assert!(result.is_ok());
}

#[test]
fn trace_empty_state_json() {
    let state_dir = tempfile::tempdir().unwrap();
    let result = super::observe::cmd_trace(state_dir.path(), None, true);
    assert!(result.is_ok());
}

// ── cmd_trace with data ─────────────────────────────────────────────

#[test]
fn trace_with_spans_text() {
    let state_dir = tempfile::tempdir().unwrap();
    let spans = format!(
        "{}\n{}\n",
        make_trace_span("trace-1", "nginx-cfg", 1_500_000, 0, 1),
        make_trace_span("trace-1", "app-cfg", 500, 0, 2),
    );
    write_trace(state_dir.path(), "web", &spans);
    let result = super::observe::cmd_trace(state_dir.path(), None, false);
    assert!(result.is_ok());
}

#[test]
fn trace_with_spans_json() {
    let state_dir = tempfile::tempdir().unwrap();
    let spans = format!(
        "{}\n{}\n",
        make_trace_span("trace-1", "nginx-cfg", 50_000, 0, 1),
        make_trace_span("trace-1", "app-cfg", 1_234_567, 1, 2),
    );
    write_trace(state_dir.path(), "web", &spans);
    let result = super::observe::cmd_trace(state_dir.path(), None, true);
    assert!(result.is_ok());
}

#[test]
fn trace_machine_filter_match() {
    let state_dir = tempfile::tempdir().unwrap();
    let spans = format!("{}\n", make_trace_span("t1", "cfg", 1000, 0, 1));
    write_trace(state_dir.path(), "web", &spans);
    write_trace(state_dir.path(), "db", &spans);
    let result = super::observe::cmd_trace(state_dir.path(), Some("web"), false);
    assert!(result.is_ok());
}

#[test]
fn trace_machine_filter_no_match() {
    let state_dir = tempfile::tempdir().unwrap();
    let spans = format!("{}\n", make_trace_span("t1", "cfg", 1000, 0, 1));
    write_trace(state_dir.path(), "web", &spans);
    let result = super::observe::cmd_trace(state_dir.path(), Some("nonexistent"), false);
    assert!(result.is_ok());
}

#[test]
fn trace_multiple_traces() {
    let state_dir = tempfile::tempdir().unwrap();
    let spans = format!(
        "{}\n{}\n{}\n",
        make_trace_span("trace-a", "nginx", 2_000_000, 0, 1),
        make_trace_span("trace-b", "app", 100, 0, 2),
        make_trace_span("trace-a", "redis", 50_000, 1, 3),
    );
    write_trace(state_dir.path(), "web", &spans);
    let result = super::observe::cmd_trace(state_dir.path(), None, false);
    assert!(result.is_ok());
}

// ── cmd_trace with failed spans ─────────────────────────────────────

#[test]
fn trace_with_failed_span_text() {
    let state_dir = tempfile::tempdir().unwrap();
    let spans = format!("{}\n", make_trace_span("t1", "broken-pkg", 3000, 1, 1));
    write_trace(state_dir.path(), "web", &spans);
    let result = super::observe::cmd_trace(state_dir.path(), None, false);
    assert!(result.is_ok());
}

// ── cmd_trace duration formatting (exercising format_duration_us) ───

#[test]
fn trace_duration_seconds() {
    let state_dir = tempfile::tempdir().unwrap();
    let spans = format!("{}\n", make_trace_span("t1", "slow", 5_000_000, 0, 1));
    write_trace(state_dir.path(), "web", &spans);
    let result = super::observe::cmd_trace(state_dir.path(), None, false);
    assert!(result.is_ok());
}

#[test]
fn trace_duration_milliseconds() {
    let state_dir = tempfile::tempdir().unwrap();
    let spans = format!("{}\n", make_trace_span("t1", "medium", 50_000, 0, 1));
    write_trace(state_dir.path(), "web", &spans);
    let result = super::observe::cmd_trace(state_dir.path(), None, false);
    assert!(result.is_ok());
}

#[test]
fn trace_duration_microseconds() {
    let state_dir = tempfile::tempdir().unwrap();
    let spans = format!("{}\n", make_trace_span("t1", "fast", 500, 0, 1));
    write_trace(state_dir.path(), "web", &spans);
    let result = super::observe::cmd_trace(state_dir.path(), None, false);
    assert!(result.is_ok());
}

#[test]
fn trace_duration_zero() {
    let state_dir = tempfile::tempdir().unwrap();
    let spans = format!("{}\n", make_trace_span("t1", "instant", 0, 0, 1));
    write_trace(state_dir.path(), "web", &spans);
    let result = super::observe::cmd_trace(state_dir.path(), None, false);
    assert!(result.is_ok());
}

// ── handle_watch_change ─────────────────────────────────────────────

#[test]
fn watch_change_plan_only() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: watch-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: /tmp/forjar-watch-test.txt
    content: hello
"#,
    );
    // auto_apply=false → plan only, no apply
    super::observe::handle_watch_change(&file, &state_dir, false);
}

#[test]
fn watch_change_invalid_config() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let file = dir.path().join("forjar.yaml");
    std::fs::write(&file, "invalid: yaml: [broken").unwrap();
    // Should print error but not panic
    super::observe::handle_watch_change(&file, &state_dir, false);
}

// ── cmd_anomaly text with findings ──────────────────────────────────

#[test]
fn anomaly_text_with_findings() {
    let state_dir = tempfile::tempdir().unwrap();
    let machine_dir = state_dir.path().join("web");
    std::fs::create_dir_all(&machine_dir).unwrap();
    let mut events = String::new();
    // 1 converge + 6 failures → high failure rate anomaly
    events.push_str(&serde_json::json!({
        "ts": "2026-03-08T10:00:00Z",
        "event": { "ResourceConverged": { "machine": "web", "resource": "flaky", "duration_seconds": 1.0, "hash": "abc" }}
    }).to_string());
    events.push('\n');
    for _ in 0..6 {
        events.push_str(&serde_json::json!({
            "ts": "2026-03-08T10:01:00Z",
            "event": { "ResourceFailed": { "machine": "web", "resource": "flaky", "error": "timeout" }}
        }).to_string());
        events.push('\n');
    }
    std::fs::write(machine_dir.join("events.jsonl"), &events).unwrap();
    let result = super::observe::cmd_anomaly(state_dir.path(), None, 3, false);
    assert!(result.is_ok());
}

#[test]
fn anomaly_json_with_findings() {
    let state_dir = tempfile::tempdir().unwrap();
    let machine_dir = state_dir.path().join("web");
    std::fs::create_dir_all(&machine_dir).unwrap();
    let mut events = String::new();
    events.push_str(&serde_json::json!({
        "ts": "2026-03-08T10:00:00Z",
        "event": { "ResourceConverged": { "machine": "web", "resource": "flaky", "duration_seconds": 1.0, "hash": "abc" }}
    }).to_string());
    events.push('\n');
    for _ in 0..6 {
        events.push_str(&serde_json::json!({
            "ts": "2026-03-08T10:01:00Z",
            "event": { "ResourceFailed": { "machine": "web", "resource": "flaky", "error": "timeout" }}
        }).to_string());
        events.push('\n');
    }
    std::fs::write(machine_dir.join("events.jsonl"), &events).unwrap();
    let result = super::observe::cmd_anomaly(state_dir.path(), None, 3, true);
    assert!(result.is_ok());
}

// ── cmd_anomaly with drift events ───────────────────────────────────

#[test]
fn anomaly_drift_events() {
    let state_dir = tempfile::tempdir().unwrap();
    let machine_dir = state_dir.path().join("web");
    std::fs::create_dir_all(&machine_dir).unwrap();
    let mut events = String::new();
    for _ in 0..5 {
        events.push_str(&serde_json::json!({
            "ts": "2026-03-08T10:00:00Z",
            "event": { "DriftDetected": { "machine": "web", "resource": "cfg", "expected_hash": "aaa", "actual_hash": "bbb" }}
        }).to_string());
        events.push('\n');
    }
    std::fs::write(machine_dir.join("events.jsonl"), &events).unwrap();
    let result = super::observe::cmd_anomaly(state_dir.path(), None, 3, false);
    assert!(result.is_ok());
}
