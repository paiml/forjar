//! Coverage tests for iso_export.rs — full export, binary inclusion, JSON mode.

use std::path::Path;

fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
    let file = dir.join("forjar.yaml");
    std::fs::write(&file, yaml).unwrap();
    file
}

// ── basic ISO export text mode ─────────────────────────────────────

#[test]
fn iso_export_basic_text() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("iso-output");
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: iso-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources: {}
"#,
    );
    let result = super::iso_export::cmd_iso_export(&file, &state_dir, &output, false, false);
    assert!(result.is_ok());
    assert!(output.join("manifest.json").exists());
    assert!(output.join("config/forjar.yaml").exists());
}

#[test]
fn iso_export_json_mode() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("iso-json");
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: iso-json
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources: {}
"#,
    );
    let result = super::iso_export::cmd_iso_export(&file, &state_dir, &output, false, true);
    assert!(result.is_ok());
}

// ── ISO export with state directory ────────────────────────────────

#[test]
fn iso_export_with_state_files() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("iso-state");
    let state_dir = dir.path().join("state");
    let machine_dir = state_dir.join("web");
    std::fs::create_dir_all(&machine_dir).unwrap();
    std::fs::write(
        machine_dir.join("state.lock.yaml"),
        "schema: '1'\nmachine: web\nhostname: h\ngenerated_at: t\ngenerator: g\nblake3_version: b\nresources: {}\n",
    )
    .unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: state-export
machines:
  web:
    hostname: h
    addr: 127.0.0.1
resources: {}
"#,
    );
    let result = super::iso_export::cmd_iso_export(&file, &state_dir, &output, false, false);
    assert!(result.is_ok());
    // manifest should reference state files
    let manifest = std::fs::read_to_string(output.join("manifest.json")).unwrap();
    assert!(manifest.contains("state"));
}

// ── ISO export with binary ─────────────────────────────────────────

#[test]
fn iso_export_include_binary() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("iso-bin");
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: bin-export
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources: {}
"#,
    );
    let result = super::iso_export::cmd_iso_export(&file, &state_dir, &output, true, false);
    assert!(result.is_ok());
    // Binary should be included (or silently skipped if current_exe fails)
    let manifest = std::fs::read_to_string(output.join("manifest.json")).unwrap();
    // The manifest should have been generated regardless
    assert!(manifest.contains("blake3_root"));
}

// ── ISO export with nonexistent state dir ──────────────────────────

#[test]
fn iso_export_missing_state_dir() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("iso-nostate");
    let state_dir = dir.path().join("nonexistent-state");
    // Don't create state_dir
    let file = write_config(
        dir.path(),
        "version: \"1.0\"\nname: t\nmachines: {}\nresources: {}\n",
    );
    let result = super::iso_export::cmd_iso_export(&file, &state_dir, &output, false, false);
    assert!(result.is_ok());
}

// ── ISO export with nested state directories ───────────────────────

#[test]
fn iso_export_nested_state() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("iso-nested");
    let state_dir = dir.path().join("state");
    let nested = state_dir.join("web/sub");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join("deep.yaml"), "key: value").unwrap();
    std::fs::write(state_dir.join("top.yaml"), "top: true").unwrap();
    let file = write_config(
        dir.path(),
        "version: \"1.0\"\nname: nested\nmachines: {}\nresources: {}\n",
    );
    let result = super::iso_export::cmd_iso_export(&file, &state_dir, &output, false, false);
    assert!(result.is_ok());
}
