//! Additional coverage tests for iso_export.rs — binary inclusion, nested state.

use super::iso_export::*;
use std::path::Path;

fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
    let p = dir.join("forjar.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

const SIMPLE_CFG: &str = r#"
version: "1.0"
name: test-export
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: /tmp/test
    content: "hello"
"#;

// ── cmd_iso_export with binary inclusion ─────────────────────────────

#[test]
fn iso_export_with_binary() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    let output = dir.path().join("iso-bin");
    let p = write_config(dir.path(), SIMPLE_CFG);
    let result = cmd_iso_export(&p, &state, &output, true, false);
    assert!(result.is_ok());
    assert!(output.join("manifest.json").exists());
}

#[test]
fn iso_export_with_binary_json() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    let output = dir.path().join("iso-bin-json");
    let p = write_config(dir.path(), SIMPLE_CFG);
    let result = cmd_iso_export(&p, &state, &output, true, true);
    assert!(result.is_ok());
}

// ── nested state directory ───────────────────────────────────────────

#[test]
fn iso_export_nested_state() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let sub = state.join("local");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("state.lock.yaml"), "resources: {}\n").unwrap();
    std::fs::write(state.join("global.yaml"), "key: val\n").unwrap();
    let output = dir.path().join("iso-nested");
    let p = write_config(dir.path(), SIMPLE_CFG);
    let result = cmd_iso_export(&p, &state, &output, false, false);
    assert!(result.is_ok());
    assert!(output.join("state").exists());
}

// ── manifest structure ───────────────────────────────────────────────

#[test]
fn iso_manifest_fields() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    let output = dir.path().join("iso-fields");
    let p = write_config(dir.path(), SIMPLE_CFG);
    cmd_iso_export(&p, &state, &output, false, false).unwrap();

    let manifest = std::fs::read_to_string(output.join("manifest.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    assert_eq!(v["name"].as_str().unwrap(), "test-export");
    assert_eq!(v["version"].as_str().unwrap(), "1.0");
    assert!(v["total_size"].as_u64().unwrap() > 0);
    assert!(!v["files"].as_array().unwrap().is_empty());
    assert_eq!(v["blake3_root"].as_str().unwrap().len(), 64);
}

#[test]
fn iso_manifest_file_categories() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(state.join("data.yaml"), "test").unwrap();
    let output = dir.path().join("iso-cats");
    let p = write_config(dir.path(), SIMPLE_CFG);
    cmd_iso_export(&p, &state, &output, false, false).unwrap();

    let manifest = std::fs::read_to_string(output.join("manifest.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    let files = v["files"].as_array().unwrap();
    let categories: Vec<&str> = files.iter().map(|f| f["category"].as_str().unwrap()).collect();
    assert!(categories.contains(&"config"));
}

// ── no state directory ───────────────────────────────────────────────

#[test]
fn iso_export_no_state_dir() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("nonexistent-state");
    let output = dir.path().join("iso-nostate");
    let p = write_config(dir.path(), SIMPLE_CFG);
    let result = cmd_iso_export(&p, &state, &output, false, false);
    assert!(result.is_ok());
    assert!(output.join("config/forjar.yaml").exists());
    assert!(!output.join("state").exists() || std::fs::read_dir(output.join("state")).unwrap().count() == 0);
}

// ── IsoFile / IsoManifest serde ──────────────────────────────────────

#[test]
fn iso_manifest_serde() {
    let manifest = IsoManifest {
        name: "test".to_string(),
        version: "1.0".to_string(),
        files: vec![IsoFile {
            path: "config/forjar.yaml".to_string(),
            size: 42,
            blake3: "a".repeat(64),
            category: "config".to_string(),
        }],
        total_size: 42,
        blake3_root: "b".repeat(64),
    };
    let json = serde_json::to_string_pretty(&manifest).unwrap();
    assert!(json.contains("\"name\": \"test\""));
    assert!(json.contains("\"total_size\": 42"));
}
