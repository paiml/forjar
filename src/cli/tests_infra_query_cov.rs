//! Coverage tests for infra_query.rs — query filters, details, json output.

use super::infra_query::*;
use std::path::Path;

fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
    let file = dir.join("forjar.yaml");
    std::fs::write(&file, yaml).unwrap();
    file
}

const MULTI_RESOURCE_CONFIG: &str = r#"
version: "1.0"
name: query
machines:
  web:
    hostname: web
    addr: 127.0.0.1
  db:
    hostname: db
    addr: 127.0.0.1
resources:
  nginx-cfg:
    type: file
    machine: web
    path: /etc/nginx/nginx.conf
    content: "worker_processes 1;"
    tags: [web, proxy]
  app-cfg:
    type: file
    machine: web
    path: /tmp/app.conf
    content: "key=val"
    tags: [app]
  pg-cfg:
    type: file
    machine: db
    path: /etc/postgresql/pg.conf
    content: "max_connections=100"
    tags: [database]
"#;

// ── no filter (all) ──────────────────────────────────────────────────

#[test]
fn query_no_filter() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(dir.path(), MULTI_RESOURCE_CONFIG);
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let filter = QueryFilter {
        pattern: None,
        resource_type: None,
        machine: None,
        tag: None,
    };
    let result = cmd_query(&file, &state_dir, &filter, false, false);
    assert!(result.is_ok());
}

#[test]
fn query_no_filter_json() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(dir.path(), MULTI_RESOURCE_CONFIG);
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let filter = QueryFilter {
        pattern: None,
        resource_type: None,
        machine: None,
        tag: None,
    };
    let result = cmd_query(&file, &state_dir, &filter, false, true);
    assert!(result.is_ok());
}

#[test]
fn query_no_filter_details() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(dir.path(), MULTI_RESOURCE_CONFIG);
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let filter = QueryFilter {
        pattern: None,
        resource_type: None,
        machine: None,
        tag: None,
    };
    let result = cmd_query(&file, &state_dir, &filter, true, false);
    assert!(result.is_ok());
}

// ── pattern filter ───────────────────────────────────────────────────

#[test]
fn query_pattern_match() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(dir.path(), MULTI_RESOURCE_CONFIG);
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let filter = QueryFilter {
        pattern: Some("nginx".to_string()),
        resource_type: None,
        machine: None,
        tag: None,
    };
    let result = cmd_query(&file, &state_dir, &filter, false, false);
    assert!(result.is_ok());
}

#[test]
fn query_pattern_no_match() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(dir.path(), MULTI_RESOURCE_CONFIG);
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let filter = QueryFilter {
        pattern: Some("nonexistent".to_string()),
        resource_type: None,
        machine: None,
        tag: None,
    };
    let result = cmd_query(&file, &state_dir, &filter, false, false);
    assert!(result.is_ok());
}

// ── resource type filter ─────────────────────────────────────────────

#[test]
fn query_type_filter() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(dir.path(), MULTI_RESOURCE_CONFIG);
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let filter = QueryFilter {
        pattern: None,
        resource_type: Some("file".to_string()),
        machine: None,
        tag: None,
    };
    let result = cmd_query(&file, &state_dir, &filter, false, false);
    assert!(result.is_ok());
}

// ── machine filter ───────────────────────────────────────────────────

#[test]
fn query_machine_filter() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(dir.path(), MULTI_RESOURCE_CONFIG);
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let filter = QueryFilter {
        pattern: None,
        resource_type: None,
        machine: Some("web".to_string()),
        tag: None,
    };
    let result = cmd_query(&file, &state_dir, &filter, false, false);
    assert!(result.is_ok());
}

#[test]
fn query_machine_filter_no_match() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(dir.path(), MULTI_RESOURCE_CONFIG);
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let filter = QueryFilter {
        pattern: None,
        resource_type: None,
        machine: Some("nonexistent".to_string()),
        tag: None,
    };
    let result = cmd_query(&file, &state_dir, &filter, false, false);
    assert!(result.is_ok());
}

// ── tag filter ───────────────────────────────────────────────────────

#[test]
fn query_tag_filter() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(dir.path(), MULTI_RESOURCE_CONFIG);
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let filter = QueryFilter {
        pattern: None,
        resource_type: None,
        machine: None,
        tag: Some("database".to_string()),
    };
    let result = cmd_query(&file, &state_dir, &filter, false, false);
    assert!(result.is_ok());
}

#[test]
fn query_tag_filter_no_match() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(dir.path(), MULTI_RESOURCE_CONFIG);
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let filter = QueryFilter {
        pattern: None,
        resource_type: None,
        machine: None,
        tag: Some("nonexistent".to_string()),
    };
    let result = cmd_query(&file, &state_dir, &filter, false, false);
    assert!(result.is_ok());
}

// ── combined filters ─────────────────────────────────────────────────

#[test]
fn query_combined_filters() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(dir.path(), MULTI_RESOURCE_CONFIG);
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let filter = QueryFilter {
        pattern: Some("cfg".to_string()),
        resource_type: Some("file".to_string()),
        machine: Some("web".to_string()),
        tag: Some("web".to_string()),
    };
    let result = cmd_query(&file, &state_dir, &filter, true, true);
    assert!(result.is_ok());
}

// ── converged status via state lock ──────────────────────────────────

#[test]
fn query_with_converged_state() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(dir.path(), MULTI_RESOURCE_CONFIG);
    let state_dir = dir.path().join("state");
    let web_dir = state_dir.join("web");
    std::fs::create_dir_all(&web_dir).unwrap();
    std::fs::write(
        web_dir.join("state.lock.yaml"),
        "schema: '1'\nmachine: web\nhostname: w\ngenerated_at: t\ngenerator: g\nblake3_version: b\nresources:\n  nginx-cfg:\n    type: file\n    status: converged\n    hash: abc\n",
    )
    .unwrap();
    let filter = QueryFilter {
        pattern: None,
        resource_type: None,
        machine: None,
        tag: None,
    };
    let result = cmd_query(&file, &state_dir, &filter, true, false);
    assert!(result.is_ok());
}

// ── empty config ─────────────────────────────────────────────────────

#[test]
fn query_empty_config() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        "version: \"1.0\"\nname: empty\nmachines: {}\nresources: {}\n",
    );
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let filter = QueryFilter {
        pattern: None,
        resource_type: None,
        machine: None,
        tag: None,
    };
    let result = cmd_query(&file, &state_dir, &filter, false, false);
    assert!(result.is_ok());
}
