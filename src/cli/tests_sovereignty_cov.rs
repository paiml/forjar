//! Coverage tests for sovereignty.rs — cmd_sovereignty text/json, tagged/untagged,
//! state file scanning, empty configs.

use std::path::Path;

fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
    let file = dir.join("forjar.yaml");
    std::fs::write(&file, yaml).unwrap();
    file
}

// ── basic text mode ─────────────────────────────────────────────

#[test]
fn sovereignty_text_with_tags() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: sov-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  db:
    type: file
    machine: local
    path: /tmp/db.conf
    content: "data"
    tags:
      - "jurisdiction:EU"
      - "classification:PII"
      - "residency:eu-west-1"
"#,
    );
    let result = super::sovereignty::cmd_sovereignty(&file, &state_dir, false);
    assert!(result.is_ok());
}

#[test]
fn sovereignty_json_with_tags() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: sov-json
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  db:
    type: file
    machine: local
    path: /tmp/db.conf
    content: "data"
    tags:
      - "jurisdiction:US"
      - "classification:internal"
"#,
    );
    let result = super::sovereignty::cmd_sovereignty(&file, &state_dir, true);
    assert!(result.is_ok());
}

// ── untagged resources ──────────────────────────────────────────

#[test]
fn sovereignty_text_untagged() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: untagged
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: /tmp/x
    content: "hello"
"#,
    );
    let result = super::sovereignty::cmd_sovereignty(&file, &state_dir, false);
    assert!(result.is_ok());
}

// ── with state YAML files ───────────────────────────────────────

#[test]
fn sovereignty_with_state_files() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::fs::write(state_dir.join("global.yaml"), "schema: '1.0'\n").unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: with-state
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources: {}
"#,
    );
    let result = super::sovereignty::cmd_sovereignty(&file, &state_dir, false);
    assert!(result.is_ok());
}

// ── empty resources ─────────────────────────────────────────────

#[test]
fn sovereignty_empty_resources() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let file = write_config(
        dir.path(),
        "version: \"1.0\"\nname: empty\nmachines: {}\nresources: {}\n",
    );
    let result = super::sovereignty::cmd_sovereignty(&file, &state_dir, true);
    assert!(result.is_ok());
}

// ── mixed tagged and untagged ───────────────────────────────────

#[test]
fn sovereignty_mixed_tags() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: mixed
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  tagged:
    type: file
    machine: local
    path: /tmp/a
    content: "a"
    tags:
      - "jurisdiction:UK"
  untagged:
    type: file
    machine: local
    path: /tmp/b
    content: "b"
"#,
    );
    let result = super::sovereignty::cmd_sovereignty(&file, &state_dir, false);
    assert!(result.is_ok());
}

// ── nonexistent state dir ───────────────────────────────────────

#[test]
fn sovereignty_no_state_dir() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("nonexistent");
    let file = write_config(
        dir.path(),
        "version: \"1.0\"\nname: nostate\nmachines: {}\nresources: {}\n",
    );
    let result = super::sovereignty::cmd_sovereignty(&file, &state_dir, false);
    assert!(result.is_ok());
}
