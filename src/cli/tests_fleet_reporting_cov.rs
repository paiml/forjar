//! Coverage tests for fleet_reporting.rs — compliance, export, suggest.

use std::path::Path;

fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
    let file = dir.join("forjar.yaml");
    std::fs::write(&file, yaml).unwrap();
    file
}

// ── cmd_compliance ───────────────────────────────────────────────────

#[test]
fn compliance_no_violations() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: ok
machines:
  web:
    hostname: web
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: web
    path: /tmp/cfg
    content: "hello"
    mode: "0644"
    owner: root
"#,
    );
    let result = super::fleet_reporting::cmd_compliance(&file, false);
    assert!(result.is_ok());
}

#[test]
fn compliance_no_violations_json() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: ok_json
machines:
  web:
    hostname: web
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: web
    path: /tmp/cfg
    content: "hello"
    mode: "0644"
    owner: root
"#,
    );
    let result = super::fleet_reporting::cmd_compliance(&file, true);
    assert!(result.is_ok());
}

#[test]
fn compliance_file_no_mode() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: nomode
machines:
  web:
    hostname: web
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: web
    path: /tmp/cfg
    content: "hello"
    owner: root
"#,
    );
    // No mode → warning
    let result = super::fleet_reporting::cmd_compliance(&file, false);
    assert!(result.is_ok());
}

#[test]
fn compliance_file_no_owner() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: noowner
machines:
  web:
    hostname: web
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: web
    path: /tmp/cfg
    content: "hello"
    mode: "0644"
"#,
    );
    let result = super::fleet_reporting::cmd_compliance(&file, false);
    assert!(result.is_ok());
}

#[test]
fn compliance_file_no_mode_no_owner() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: bare
machines:
  web:
    hostname: web
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: web
    path: /tmp/cfg
    content: "hello"
"#,
    );
    let result = super::fleet_reporting::cmd_compliance(&file, false);
    assert!(result.is_ok());
}

#[test]
fn compliance_system_path_no_owner() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: syspath
machines:
  web:
    hostname: web
    addr: 127.0.0.1
resources:
  nginx-cfg:
    type: file
    machine: web
    path: /etc/nginx/nginx.conf
    content: "worker_processes 1;"
    mode: "0644"
"#,
    );
    // System path without owner → error severity
    let result = super::fleet_reporting::cmd_compliance(&file, false);
    assert!(result.is_ok());
}

#[test]
fn compliance_system_path_json() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: syspath_json
machines:
  web:
    hostname: web
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: web
    path: /usr/local/etc/app.conf
    content: "key=val"
"#,
    );
    let result = super::fleet_reporting::cmd_compliance(&file, true);
    assert!(result.is_ok());
}

#[test]
fn compliance_service_no_enabled() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: svc
machines:
  web:
    hostname: web
    addr: 127.0.0.1
resources:
  nginx:
    type: service
    machine: web
    name: nginx
    state: running
"#,
    );
    let result = super::fleet_reporting::cmd_compliance(&file, false);
    assert!(result.is_ok());
}

#[test]
fn compliance_service_with_enabled() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: svc_ok
machines:
  web:
    hostname: web
    addr: 127.0.0.1
resources:
  nginx:
    type: service
    machine: web
    name: nginx
    state: running
    enabled: true
"#,
    );
    let result = super::fleet_reporting::cmd_compliance(&file, false);
    assert!(result.is_ok());
}

#[test]
fn compliance_empty_resources() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        "version: \"1.0\"\nname: empty\nmachines: {}\nresources: {}\n",
    );
    let result = super::fleet_reporting::cmd_compliance(&file, false);
    assert!(result.is_ok());
}

// ── cmd_export ───────────────────────────────────────────────────────

#[test]
fn export_csv_empty_state() {
    let state_dir = tempfile::tempdir().unwrap();
    let result = super::fleet_reporting::cmd_export(state_dir.path(), "csv", None, None);
    assert!(result.is_ok());
}

#[test]
fn export_json_empty_state() {
    let state_dir = tempfile::tempdir().unwrap();
    let result = super::fleet_reporting::cmd_export(state_dir.path(), "json", None, None);
    assert!(result.is_ok());
}

#[test]
fn export_terraform_empty_state() {
    let state_dir = tempfile::tempdir().unwrap();
    let result = super::fleet_reporting::cmd_export(state_dir.path(), "terraform", None, None);
    assert!(result.is_ok());
}

#[test]
fn export_ansible_empty_state() {
    let state_dir = tempfile::tempdir().unwrap();
    let result = super::fleet_reporting::cmd_export(state_dir.path(), "ansible", None, None);
    assert!(result.is_ok());
}

#[test]
fn export_unknown_format() {
    let state_dir = tempfile::tempdir().unwrap();
    let result = super::fleet_reporting::cmd_export(state_dir.path(), "xml", None, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unknown export format"));
}

#[test]
fn export_csv_with_locks() {
    let state_dir = tempfile::tempdir().unwrap();
    let machine_dir = state_dir.path().join("web");
    std::fs::create_dir_all(&machine_dir).unwrap();
    std::fs::write(
        machine_dir.join("state.lock.yaml"),
        "schema: '1'\nmachine: web\nhostname: w\ngenerated_at: t\ngenerator: g\nblake3_version: b\nresources:\n  nginx:\n    type: package\n    status: converged\n    hash: abc123\n",
    )
    .unwrap();
    let result = super::fleet_reporting::cmd_export(state_dir.path(), "csv", None, None);
    assert!(result.is_ok());
}

#[test]
fn export_ansible_with_locks() {
    let state_dir = tempfile::tempdir().unwrap();
    let machine_dir = state_dir.path().join("web");
    std::fs::create_dir_all(&machine_dir).unwrap();
    std::fs::write(
        machine_dir.join("state.lock.yaml"),
        "schema: '1'\nmachine: web\nhostname: w\ngenerated_at: t\ngenerator: g\nblake3_version: b\nresources:\n  nginx:\n    type: package\n    status: converged\n    hash: abc\n  redis:\n    type: package\n    status: converged\n    hash: def\n",
    )
    .unwrap();
    let result = super::fleet_reporting::cmd_export(state_dir.path(), "ansible", None, None);
    assert!(result.is_ok());
}

#[test]
fn export_to_output_file() {
    let state_dir = tempfile::tempdir().unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    let out = out_dir.path().join("export.csv");
    let result =
        super::fleet_reporting::cmd_export(state_dir.path(), "csv", None, Some(out.as_path()));
    assert!(result.is_ok());
    assert!(out.exists());
}

#[test]
fn export_machine_filter() {
    let state_dir = tempfile::tempdir().unwrap();
    let web_dir = state_dir.path().join("web");
    let db_dir = state_dir.path().join("db");
    std::fs::create_dir_all(&web_dir).unwrap();
    std::fs::create_dir_all(&db_dir).unwrap();
    std::fs::write(
        web_dir.join("state.lock.yaml"),
        "schema: '1'\nmachine: web\nhostname: w\ngenerated_at: t\ngenerator: g\nblake3_version: b\nresources:\n  nginx:\n    type: package\n    status: converged\n    hash: abc\n",
    )
    .unwrap();
    std::fs::write(
        db_dir.join("state.lock.yaml"),
        "schema: '1'\nmachine: db\nhostname: d\ngenerated_at: t\ngenerator: g\nblake3_version: b\nresources:\n  pg:\n    type: package\n    status: converged\n    hash: def\n",
    )
    .unwrap();
    let result =
        super::fleet_reporting::cmd_export(state_dir.path(), "json", Some("web"), None);
    assert!(result.is_ok());
}

// ── cmd_suggest ──────────────────────────────────────────────────────

#[test]
fn suggest_no_suggestions() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        "version: \"1.0\"\nname: ok\nmachines: {}\nresources: {}\n",
    );
    let result = super::fleet_reporting::cmd_suggest(&file, false);
    assert!(result.is_ok());
}

#[test]
fn suggest_no_suggestions_json() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        "version: \"1.0\"\nname: ok\nmachines: {}\nresources: {}\n",
    );
    let result = super::fleet_reporting::cmd_suggest(&file, true);
    assert!(result.is_ok());
}

#[test]
fn suggest_unused_params() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: params
machines:
  m:
    hostname: m
    addr: 127.0.0.1
params:
  port: "8080"
  unused_var: "never_used"
resources:
  cfg:
    type: file
    machine: m
    path: /tmp/cfg
    content: "port={{params.port}}"
"#,
    );
    // unused_var is defined but not in any template → suggestion
    let result = super::fleet_reporting::cmd_suggest(&file, false);
    assert!(result.is_ok());
}

#[test]
fn suggest_unused_params_json() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: params_json
machines:
  m:
    hostname: m
    addr: 127.0.0.1
params:
  orphan: "value"
resources: {}
"#,
    );
    let result = super::fleet_reporting::cmd_suggest(&file, true);
    assert!(result.is_ok());
}

#[test]
fn suggest_no_depends_on() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: deps
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  a:
    type: file
    machine: m
    path: /tmp/a
    content: "a"
  b:
    type: file
    machine: m
    path: /tmp/b
    content: "b"
  c:
    type: file
    machine: m
    path: /tmp/c
    content: "c"
"#,
    );
    // 3 resources on same machine, none with depends_on → suggestions
    let result = super::fleet_reporting::cmd_suggest(&file, false);
    assert!(result.is_ok());
}

// ── cmd_audit ────────────────────────────────────────────────────────

#[test]
fn audit_empty_json() {
    let state_dir = tempfile::tempdir().unwrap();
    let result = super::fleet_reporting::cmd_audit(state_dir.path(), None, 20, true);
    assert!(result.is_ok());
}

#[test]
fn audit_with_machine_filter() {
    let state_dir = tempfile::tempdir().unwrap();
    let result = super::fleet_reporting::cmd_audit(state_dir.path(), Some("web"), 20, false);
    assert!(result.is_ok());
}

#[test]
fn audit_nonexistent_dir() {
    let result = super::fleet_reporting::cmd_audit(
        std::path::Path::new("/nonexistent/state"),
        None,
        20,
        false,
    );
    assert!(result.is_err());
}

#[test]
fn audit_with_event_log() {
    let state_dir = tempfile::tempdir().unwrap();
    let machine_dir = state_dir.path().join("web");
    std::fs::create_dir_all(&machine_dir).unwrap();
    // Write a minimal event log
    let event = r#"{"ts":"2026-03-08T10:00:00Z","event":{"ApplyStarted":{"machine":"web","run_id":"run-1","config_hash":"abc"}}}"#;
    std::fs::write(machine_dir.join("events.jsonl"), format!("{event}\n")).unwrap();
    let result = super::fleet_reporting::cmd_audit(state_dir.path(), None, 20, false);
    assert!(result.is_ok());
}

