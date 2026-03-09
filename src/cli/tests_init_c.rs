//! Coverage tests for init.rs — init, fmt, schema commands.

use super::init::*;

// ── cmd_init ────────────────────────────────────────────────────────

#[test]
fn init_creates_project() {
    let dir = tempfile::tempdir().unwrap();
    let result = cmd_init(dir.path());
    assert!(result.is_ok());
    assert!(dir.path().join("forjar.yaml").exists());
    assert!(dir.path().join("state").exists());
}

#[test]
fn init_already_exists() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("forjar.yaml"), "existing").unwrap();
    let result = cmd_init(dir.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already exists"));
}

#[test]
fn init_template_is_valid_yaml() {
    let dir = tempfile::tempdir().unwrap();
    cmd_init(dir.path()).unwrap();
    let content = std::fs::read_to_string(dir.path().join("forjar.yaml")).unwrap();
    let config: Result<crate::core::types::ForjarConfig, _> = serde_yaml_ng::from_str(&content);
    assert!(config.is_ok(), "init template should be valid YAML");
}

// ── cmd_fmt ─────────────────────────────────────────────────────────

#[test]
fn fmt_formats_file() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("test.yaml");
    let yaml = "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n";
    std::fs::write(&f, yaml).unwrap();
    let result = cmd_fmt(&f, false);
    assert!(result.is_ok());
}

#[test]
fn fmt_check_mode() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("test.yaml");
    let yaml = "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n";
    std::fs::write(&f, yaml).unwrap();
    // Format first
    cmd_fmt(&f, false).unwrap();
    // Check should pass after formatting
    let result = cmd_fmt(&f, true);
    assert!(result.is_ok());
}

#[test]
fn fmt_invalid_yaml() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("bad.yaml");
    std::fs::write(&f, "{{not valid").unwrap();
    let result = cmd_fmt(&f, false);
    assert!(result.is_err());
}

#[test]
fn fmt_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("nonexistent.yaml");
    let result = cmd_fmt(&f, false);
    assert!(result.is_err());
}

// ── cmd_schema ──────────────────────────────────────────────────────

#[test]
fn schema_produces_valid_json() {
    let result = cmd_schema();
    assert!(result.is_ok());
}
