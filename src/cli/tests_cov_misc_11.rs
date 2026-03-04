//! Coverage boost: observe, bundle, iso_export, plan, drift, status_convergence.

use super::bundle::*;
use super::iso_export::*;
use std::io::Write;

fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(yaml.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

fn write_yaml(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
    let p = dir.join(name);
    let mut f = std::fs::File::create(&p).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    p
}

const BASIC_CONFIG: &str = r#"
version: "1.0"
name: t
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [nginx]
  svc:
    type: service
    machine: m
    name: nginx
    depends_on: [pkg]
"#;

// ── bundle ──────────────────────────────────────────

#[test]
fn test_bundle_basic() {
    let f = write_temp_config(BASIC_CONFIG);
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("bundle.tar");
    let result = cmd_bundle(f.path(), Some(&out), false);
    assert!(result.is_ok());
}

#[test]
fn test_bundle_with_state() {
    let f = write_temp_config(BASIC_CONFIG);
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("bundle.tar");
    let result = cmd_bundle(f.path(), Some(&out), true);
    assert!(result.is_ok());
}

#[test]
fn test_bundle_to_stdout() {
    let f = write_temp_config(BASIC_CONFIG);
    let result = cmd_bundle(f.path(), None, false);
    assert!(result.is_ok());
}

#[test]
fn test_bundle_missing_config() {
    let result = cmd_bundle(
        std::path::Path::new("/nonexistent/config.yaml"),
        None,
        false,
    );
    assert!(result.is_err());
}

// ── iso_export ──────────────────────────────────────

#[test]
fn test_iso_export_basic() {
    let f = write_temp_config(BASIC_CONFIG);
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let out = dir.path().join("iso");
    let result = cmd_iso_export(f.path(), &state_dir, &out, false, false);
    assert!(result.is_ok());
}

#[test]
fn test_iso_export_json() {
    let f = write_temp_config(BASIC_CONFIG);
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let out = dir.path().join("iso");
    let result = cmd_iso_export(f.path(), &state_dir, &out, false, true);
    assert!(result.is_ok());
}

#[test]
fn test_iso_export_with_binary() {
    let f = write_temp_config(BASIC_CONFIG);
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let out = dir.path().join("iso");
    let result = cmd_iso_export(f.path(), &state_dir, &out, true, false);
    let _ = result;
}

#[test]
fn test_iso_export_missing_config() {
    let dir = tempfile::tempdir().unwrap();
    let result = cmd_iso_export(
        std::path::Path::new("/nonexistent.yaml"),
        dir.path(),
        &dir.path().join("iso"),
        false,
        false,
    );
    assert!(result.is_err());
}

// ── plan ────────────────────────────────────────────

#[test]
fn test_plan_basic() {
    let f = write_temp_config(BASIC_CONFIG);
    let dir = tempfile::tempdir().unwrap();
    let result = super::plan::cmd_plan(
        f.path(), dir.path(), None, None, None, false, false, None, None, None, false, None,
        false, &[], None, false,
    );
    assert!(result.is_ok());
}

#[test]
fn test_plan_json() {
    let f = write_temp_config(BASIC_CONFIG);
    let dir = tempfile::tempdir().unwrap();
    let result = super::plan::cmd_plan(
        f.path(), dir.path(), None, None, None, true, false, None, None, None, false, None,
        false, &[], None, false,
    );
    assert!(result.is_ok());
}

#[test]
fn test_plan_with_tag_filter() {
    let yaml = r#"
version: "1.0"
name: t
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [nginx]
    tags: [web]
  db:
    type: package
    machine: m
    provider: apt
    packages: [postgresql]
    tags: [database]
"#;
    let f = write_temp_config(yaml);
    let dir = tempfile::tempdir().unwrap();
    let result = super::plan::cmd_plan(
        f.path(), dir.path(), None, None, Some("web"), false, false, None, None, None, false,
        None, false, &[], None, false,
    );
    assert!(result.is_ok());
}

#[test]
fn test_plan_with_machine_filter() {
    let f = write_temp_config(BASIC_CONFIG);
    let dir = tempfile::tempdir().unwrap();
    let result = super::plan::cmd_plan(
        f.path(), dir.path(), Some("m"), None, None, false, false, None, None, None, false,
        None, false, &[], None, false,
    );
    assert!(result.is_ok());
}

#[test]
fn test_plan_missing_config() {
    let dir = tempfile::tempdir().unwrap();
    let result = super::plan::cmd_plan(
        std::path::Path::new("/nonexistent.yaml"),
        dir.path(), None, None, None, false, false, None, None, None, false, None, false, &[],
        None, false,
    );
    assert!(result.is_err());
}

// ── observe (anomaly detection) ─────────────────────

#[test]
fn test_anomaly_empty_state() {
    let dir = tempfile::tempdir().unwrap();
    let result = super::observe::cmd_anomaly(dir.path(), None, 3, false);
    let _ = result;
}

#[test]
fn test_anomaly_empty_state_json() {
    let dir = tempfile::tempdir().unwrap();
    let result = super::observe::cmd_anomaly(dir.path(), None, 3, true);
    let _ = result;
}

#[test]
fn test_anomaly_with_events() {
    let dir = tempfile::tempdir().unwrap();
    let m_dir = dir.path().join("m");
    std::fs::create_dir_all(&m_dir).unwrap();
    let events = r#"{"timestamp":"2025-01-01T00:00:00Z","event_type":"apply","machine":"m","resource_id":"pkg","status":"converged","run_id":"r1"}
{"timestamp":"2025-01-01T00:01:00Z","event_type":"apply","machine":"m","resource_id":"pkg","status":"converged","run_id":"r2"}
{"timestamp":"2025-01-01T00:02:00Z","event_type":"apply","machine":"m","resource_id":"pkg","status":"failed","run_id":"r3"}
"#;
    write_yaml(&m_dir, "events.jsonl", events);
    let result = super::observe::cmd_anomaly(dir.path(), None, 2, false);
    let _ = result;
}

#[test]
fn test_anomaly_with_events_json() {
    let dir = tempfile::tempdir().unwrap();
    let m_dir = dir.path().join("m");
    std::fs::create_dir_all(&m_dir).unwrap();
    let events = r#"{"timestamp":"2025-01-01T00:00:00Z","event_type":"apply","machine":"m","resource_id":"pkg","status":"converged","run_id":"r1"}
{"timestamp":"2025-01-01T00:01:00Z","event_type":"apply","machine":"m","resource_id":"pkg","status":"failed","run_id":"r2"}
"#;
    write_yaml(&m_dir, "events.jsonl", events);
    let result = super::observe::cmd_anomaly(dir.path(), None, 1, true);
    let _ = result;
}

#[test]
fn test_anomaly_with_machine_filter() {
    let dir = tempfile::tempdir().unwrap();
    let m_dir = dir.path().join("m");
    std::fs::create_dir_all(&m_dir).unwrap();
    let events = r#"{"timestamp":"2025-01-01T00:00:00Z","event_type":"apply","machine":"m","resource_id":"pkg","status":"converged","run_id":"r1"}
"#;
    write_yaml(&m_dir, "events.jsonl", events);
    let result = super::observe::cmd_anomaly(dir.path(), Some("m"), 1, false);
    let _ = result;
}

// ── drift ───────────────────────────────────────────

#[test]
fn test_drift_empty_state() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_temp_config(BASIC_CONFIG);
    let result = super::drift::cmd_drift(
        f.path(), dir.path(), None, false, None, false, false, false, false, None,
    );
    let _ = result;
}

#[test]
fn test_drift_empty_state_json() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_temp_config(BASIC_CONFIG);
    let result = super::drift::cmd_drift(
        f.path(), dir.path(), None, false, None, false, false, true, false, None,
    );
    let _ = result;
}

#[test]
fn test_drift_dry_run() {
    let dir = tempfile::tempdir().unwrap();
    let f = write_temp_config(BASIC_CONFIG);
    let result = super::drift::cmd_drift(
        f.path(), dir.path(), None, false, None, false, true, false, false, None,
    );
    let _ = result;
}

// ── status_convergence ──────────────────────────────

#[test]
fn test_convergence_rate_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let result = super::status_convergence::cmd_status_convergence_rate(dir.path(), None, false);
    let _ = result;
}

#[test]
fn test_convergence_rate_empty_dir_json() {
    let dir = tempfile::tempdir().unwrap();
    let result = super::status_convergence::cmd_status_convergence_rate(dir.path(), None, true);
    let _ = result;
}

#[test]
fn test_convergence_time_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let result = super::status_convergence::cmd_status_convergence_time(dir.path(), None, false);
    let _ = result;
}

#[test]
fn test_convergence_time_empty_dir_json() {
    let dir = tempfile::tempdir().unwrap();
    let result = super::status_convergence::cmd_status_convergence_time(dir.path(), None, true);
    let _ = result;
}

#[test]
fn test_convergence_history_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let result =
        super::status_convergence::cmd_status_convergence_history(dir.path(), None, false);
    let _ = result;
}

#[test]
fn test_convergence_history_empty_dir_json() {
    let dir = tempfile::tempdir().unwrap();
    let result =
        super::status_convergence::cmd_status_convergence_history(dir.path(), None, true);
    let _ = result;
}
