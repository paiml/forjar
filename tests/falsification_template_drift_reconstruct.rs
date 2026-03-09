//! FJ-003/016/1280: Template resolution, drift detection, and state
//! reconstruction falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-003: Template resolution
//!   - resolve_template: params, machine refs, nested, missing
//!   - resolve_resource_templates: all string fields resolved
//!   - redact_secrets: secret masking
//! - FJ-016: Drift detection
//!   - check_file_drift: match, mismatch, missing file
//! - FJ-1280: State reconstruction
//!   - reconstruct_at: event replay, timestamp cutoff, empty log
//!
//! Usage: cargo test --test falsification_template_drift_reconstruct

use forjar::core::resolver::{redact_secrets, resolve_resource_templates, resolve_template};
use forjar::core::state::reconstruct::reconstruct_at;
use forjar::core::types::*;
use forjar::tripwire::drift::check_file_drift;
use forjar::tripwire::hasher::hash_string;
use indexmap::IndexMap;
use std::collections::HashMap;

// ============================================================================
// FJ-003: resolve_template — params
// ============================================================================

#[test]
fn template_param_substitution() {
    let mut params = HashMap::new();
    params.insert("port".into(), serde_yaml_ng::Value::Number(8080.into()));
    let machines = IndexMap::new();

    let result = resolve_template("listen {{params.port}}", &params, &machines).unwrap();
    assert_eq!(result, "listen 8080");
}

#[test]
fn template_string_param() {
    let mut params = HashMap::new();
    params.insert(
        "env".into(),
        serde_yaml_ng::Value::String("production".into()),
    );
    let machines = IndexMap::new();

    let result = resolve_template("deploy to {{params.env}}", &params, &machines).unwrap();
    assert_eq!(result, "deploy to production");
}

#[test]
fn template_multiple_params() {
    let mut params = HashMap::new();
    params.insert(
        "host".into(),
        serde_yaml_ng::Value::String("db.local".into()),
    );
    params.insert("port".into(), serde_yaml_ng::Value::Number(5432.into()));
    let machines = IndexMap::new();

    let result = resolve_template(
        "postgres://{{params.host}}:{{params.port}}",
        &params,
        &machines,
    )
    .unwrap();
    assert_eq!(result, "postgres://db.local:5432");
}

#[test]
fn template_no_variables_passthrough() {
    let params = HashMap::new();
    let machines = IndexMap::new();
    let result = resolve_template("no templates here", &params, &machines).unwrap();
    assert_eq!(result, "no templates here");
}

#[test]
fn template_missing_param_errors() {
    let params = HashMap::new();
    let machines = IndexMap::new();
    let err = resolve_template("{{params.missing}}", &params, &machines).unwrap_err();
    assert!(
        err.contains("unknown") || err.contains("missing"),
        "err: {err}"
    );
}

#[test]
fn template_unclosed_brace_errors() {
    let params = HashMap::new();
    let machines = IndexMap::new();
    let err = resolve_template("{{params.x", &params, &machines).unwrap_err();
    assert!(err.contains("unclosed"), "err: {err}");
}

// ============================================================================
// FJ-003: resolve_template — machine refs
// ============================================================================

fn make_machine(hostname: &str, addr: &str, user: &str, arch: &str) -> Machine {
    Machine {
        hostname: hostname.into(),
        addr: addr.into(),
        user: user.into(),
        arch: arch.into(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    }
}

#[test]
fn template_machine_addr() {
    let params = HashMap::new();
    let mut machines = IndexMap::new();
    machines.insert(
        "web-01".into(),
        make_machine("web-01.example.com", "10.0.1.10", "deploy", "x86_64"),
    );

    let result = resolve_template(
        "ssh {{machine.web-01.user}}@{{machine.web-01.addr}}",
        &params,
        &machines,
    )
    .unwrap();
    assert_eq!(result, "ssh deploy@10.0.1.10");
}

#[test]
fn template_machine_hostname() {
    let params = HashMap::new();
    let mut machines = IndexMap::new();
    machines.insert(
        "db-01".into(),
        make_machine("database.local", "10.0.2.5", "root", "aarch64"),
    );

    let result = resolve_template("host={{machine.db-01.hostname}}", &params, &machines).unwrap();
    assert_eq!(result, "host=database.local");
}

#[test]
fn template_machine_unknown_errors() {
    let params = HashMap::new();
    let machines = IndexMap::new();
    let err = resolve_template("{{machine.ghost.addr}}", &params, &machines).unwrap_err();
    assert!(err.contains("unknown"), "err: {err}");
}

#[test]
fn template_machine_unknown_field_errors() {
    let params = HashMap::new();
    let mut machines = IndexMap::new();
    machines.insert("m1".into(), make_machine("m1", "1.2.3.4", "root", "x86_64"));
    let err = resolve_template("{{machine.m1.bogus}}", &params, &machines).unwrap_err();
    assert!(err.contains("unknown"), "err: {err}");
}

// ============================================================================
// FJ-003: resolve_resource_templates
// ============================================================================

#[test]
fn resource_template_resolves_content() {
    let mut params = HashMap::new();
    params.insert(
        "greeting".into(),
        serde_yaml_ng::Value::String("hello".into()),
    );
    let machines = IndexMap::new();

    let resource = Resource {
        resource_type: ResourceType::File,
        content: Some("msg={{params.greeting}}".into()),
        ..Default::default()
    };
    let resolved = resolve_resource_templates(&resource, &params, &machines).unwrap();
    assert_eq!(resolved.content.as_deref(), Some("msg=hello"));
}

#[test]
fn resource_template_resolves_path() {
    let mut params = HashMap::new();
    params.insert("app".into(), serde_yaml_ng::Value::String("myapp".into()));
    let machines = IndexMap::new();

    let resource = Resource {
        resource_type: ResourceType::File,
        path: Some("/etc/{{params.app}}/config.yaml".into()),
        ..Default::default()
    };
    let resolved = resolve_resource_templates(&resource, &params, &machines).unwrap();
    assert_eq!(resolved.path.as_deref(), Some("/etc/myapp/config.yaml"));
}

#[test]
fn resource_template_resolves_packages() {
    let mut params = HashMap::new();
    params.insert("pkg".into(), serde_yaml_ng::Value::String("nginx".into()));
    let machines = IndexMap::new();

    let resource = Resource {
        resource_type: ResourceType::Package,
        packages: vec!["{{params.pkg}}".into()],
        ..Default::default()
    };
    let resolved = resolve_resource_templates(&resource, &params, &machines).unwrap();
    assert_eq!(resolved.packages, vec!["nginx"]);
}

#[test]
fn resource_template_none_fields_stay_none() {
    let params = HashMap::new();
    let machines = IndexMap::new();
    let resource = Resource {
        resource_type: ResourceType::File,
        ..Default::default()
    };
    let resolved = resolve_resource_templates(&resource, &params, &machines).unwrap();
    assert!(resolved.content.is_none());
    assert!(resolved.path.is_none());
}

// ============================================================================
// FJ-2300: redact_secrets
// ============================================================================

#[test]
fn redact_replaces_secret_values() {
    let text = "password=s3cret&token=abc123";
    let secrets = vec!["s3cret".into(), "abc123".into()];
    let redacted = redact_secrets(text, &secrets);
    assert_eq!(redacted, "password=***&token=***");
}

#[test]
fn redact_empty_secrets_noop() {
    let text = "nothing to hide";
    let redacted = redact_secrets(text, &[]);
    assert_eq!(redacted, "nothing to hide");
}

#[test]
fn redact_empty_string_secret_ignored() {
    let text = "keep this";
    let secrets = vec!["".into()];
    let redacted = redact_secrets(text, &secrets);
    assert_eq!(redacted, "keep this");
}

// ============================================================================
// FJ-016: check_file_drift — match
// ============================================================================

#[test]
fn drift_no_change_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.txt");
    std::fs::write(&path, "stable content").unwrap();
    let expected = hash_string("stable content");

    let finding = check_file_drift("my-config", path.to_str().unwrap(), &expected);
    assert!(finding.is_none(), "no drift expected");
}

#[test]
fn drift_content_changed_returns_finding() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.txt");
    std::fs::write(&path, "original").unwrap();
    let expected = hash_string("original");
    // Tamper
    std::fs::write(&path, "tampered").unwrap();

    let finding = check_file_drift("my-config", path.to_str().unwrap(), &expected);
    assert!(finding.is_some());
    let f = finding.unwrap();
    assert_eq!(f.resource_id, "my-config");
    assert_eq!(f.expected_hash, expected);
    assert_ne!(f.actual_hash, expected);
    assert!(f.detail.contains("changed"));
}

#[test]
fn drift_missing_file_returns_finding() {
    let finding = check_file_drift("ghost", "/tmp/nonexistent_forjar_test_file", "blake3:xxx");
    assert!(finding.is_some());
    let f = finding.unwrap();
    assert_eq!(f.actual_hash, "MISSING");
    assert!(f.detail.contains("does not exist"));
}

#[test]
fn drift_directory_hashing() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("subdir");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("a.txt"), "aaa").unwrap();
    std::fs::write(sub.join("b.txt"), "bbb").unwrap();

    // Hash the directory to get expected hash
    let expected = forjar::tripwire::hasher::hash_directory(&sub).unwrap();

    let finding = check_file_drift("my-dir", sub.to_str().unwrap(), &expected);
    assert!(finding.is_none(), "directory hash should match");
}

// ============================================================================
// FJ-1280: reconstruct_at — event replay
// ============================================================================

fn write_events(dir: &std::path::Path, machine: &str, events: &[&str]) {
    let machine_dir = dir.join(machine);
    std::fs::create_dir_all(&machine_dir).unwrap();
    let content = events.join("\n") + "\n";
    std::fs::write(machine_dir.join("events.jsonl"), content).unwrap();
}

#[test]
fn reconstruct_replays_converged() {
    let dir = tempfile::tempdir().unwrap();
    write_events(
        dir.path(),
        "web-01",
        &[
            r#"{"ts":"2026-01-01T00:00:00Z","event":"resource_converged","machine":"web-01","resource":"nginx","duration_seconds":1.5,"hash":"blake3:abc"}"#,
        ],
    );

    let lock = reconstruct_at(dir.path(), "web-01", "2026-12-31T23:59:59Z").unwrap();
    assert_eq!(lock.machine, "web-01");
    assert_eq!(lock.resources.len(), 1);
    let nginx = &lock.resources["nginx"];
    assert_eq!(nginx.status, ResourceStatus::Converged);
    assert_eq!(nginx.hash, "blake3:abc");
}

#[test]
fn reconstruct_timestamp_cutoff() {
    let dir = tempfile::tempdir().unwrap();
    write_events(
        dir.path(),
        "web-01",
        &[
            r#"{"ts":"2026-01-01T00:00:00Z","event":"resource_converged","machine":"web-01","resource":"early","duration_seconds":1.0,"hash":"blake3:e1"}"#,
            r#"{"ts":"2026-06-01T00:00:00Z","event":"resource_converged","machine":"web-01","resource":"late","duration_seconds":2.0,"hash":"blake3:e2"}"#,
        ],
    );

    // Reconstruct at a time between the two events
    let lock = reconstruct_at(dir.path(), "web-01", "2026-03-01T00:00:00Z").unwrap();
    assert_eq!(lock.resources.len(), 1);
    assert!(lock.resources.contains_key("early"));
    assert!(!lock.resources.contains_key("late"));
}

#[test]
fn reconstruct_failed_event() {
    let dir = tempfile::tempdir().unwrap();
    write_events(
        dir.path(),
        "m1",
        &[
            r#"{"ts":"2026-01-01T00:00:00Z","event":"resource_failed","machine":"m1","resource":"broken","error":"timeout"}"#,
        ],
    );

    let lock = reconstruct_at(dir.path(), "m1", "2026-12-31T23:59:59Z").unwrap();
    let broken = &lock.resources["broken"];
    assert_eq!(broken.status, ResourceStatus::Failed);
}

#[test]
fn reconstruct_drift_event_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    write_events(
        dir.path(),
        "m1",
        &[
            r#"{"ts":"2026-01-01T00:00:00Z","event":"resource_converged","machine":"m1","resource":"cfg","duration_seconds":1.0,"hash":"blake3:original"}"#,
            r#"{"ts":"2026-02-01T00:00:00Z","event":"drift_detected","machine":"m1","resource":"cfg","expected_hash":"blake3:original","actual_hash":"blake3:drifted"}"#,
        ],
    );

    let lock = reconstruct_at(dir.path(), "m1", "2026-12-31T23:59:59Z").unwrap();
    let cfg = &lock.resources["cfg"];
    assert_eq!(cfg.status, ResourceStatus::Drifted);
    assert_eq!(cfg.hash, "blake3:drifted");
}

#[test]
fn reconstruct_no_event_log_errors() {
    let dir = tempfile::tempdir().unwrap();
    let err = reconstruct_at(dir.path(), "nonexistent", "2026-01-01T00:00:00Z").unwrap_err();
    assert!(err.contains("no event log"), "err: {err}");
}

#[test]
fn reconstruct_apply_started_sets_hostname() {
    let dir = tempfile::tempdir().unwrap();
    write_events(
        dir.path(),
        "m1",
        &[
            r#"{"ts":"2026-01-01T00:00:00Z","event":"apply_started","machine":"web-01.prod","run_id":"r-abc","forjar_version":"1.0"}"#,
        ],
    );

    let lock = reconstruct_at(dir.path(), "m1", "2026-12-31T23:59:59Z").unwrap();
    assert_eq!(lock.hostname, "web-01.prod");
}

#[test]
fn reconstruct_multiple_updates_last_wins() {
    let dir = tempfile::tempdir().unwrap();
    write_events(
        dir.path(),
        "m1",
        &[
            r#"{"ts":"2026-01-01T00:00:00Z","event":"resource_converged","machine":"m1","resource":"pkg","duration_seconds":1.0,"hash":"blake3:v1"}"#,
            r#"{"ts":"2026-02-01T00:00:00Z","event":"resource_converged","machine":"m1","resource":"pkg","duration_seconds":0.5,"hash":"blake3:v2"}"#,
        ],
    );

    let lock = reconstruct_at(dir.path(), "m1", "2026-12-31T23:59:59Z").unwrap();
    assert_eq!(lock.resources["pkg"].hash, "blake3:v2");
}
