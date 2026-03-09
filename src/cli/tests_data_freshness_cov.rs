//! Coverage tests for data_freshness.rs — artifact freshness, store dir, stale detection.

use super::data_freshness::*;
use std::path::Path;

fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
    let file = dir.join("forjar.yaml");
    std::fs::write(&file, yaml).unwrap();
    file
}

// ── freshness with output_artifacts ──────────────────────────────────

#[test]
fn freshness_with_fresh_artifact() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("output.bin"), b"fresh data").unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: fresh
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  data:
    type: file
    machine: m
    path: /tmp/data
    content: "x"
    output_artifacts:
      - output.bin
"#,
    );
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let result = cmd_data_freshness(&file, &state_dir, Some(24), false);
    assert!(result.is_ok());
}

#[test]
fn freshness_with_missing_artifact() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: miss
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  data:
    type: file
    machine: m
    path: /tmp/data
    content: "x"
    output_artifacts:
      - nonexistent.bin
"#,
    );
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    // Missing artifacts are not counted as stale
    let result = cmd_data_freshness(&file, &state_dir, Some(24), false);
    assert!(result.is_ok());
}

#[test]
fn freshness_json_with_artifact() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("model.pt"), b"weights").unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: json
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  model:
    type: file
    machine: m
    path: /tmp/model
    content: "x"
    output_artifacts:
      - model.pt
"#,
    );
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let result = cmd_data_freshness(&file, &state_dir, Some(24), true);
    assert!(result.is_ok());
}

#[test]
fn freshness_json_with_missing_artifact() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: json_miss
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  data:
    type: file
    machine: m
    path: /tmp/data
    content: "x"
    output_artifacts:
      - gone.bin
"#,
    );
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let result = cmd_data_freshness(&file, &state_dir, Some(24), true);
    assert!(result.is_ok());
}

// ── store dir freshness ──────────────────────────────────────────────

#[test]
fn freshness_store_dir_with_files() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("store");
    std::fs::create_dir_all(&store).unwrap();
    std::fs::write(store.join("cache.db"), b"cached data").unwrap();
    std::fs::write(store.join("index.bin"), b"index").unwrap();
    let file = write_config(
        dir.path(),
        "version: \"1.0\"\nname: store\nmachines: {}\nresources: {}\n",
    );
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let result = cmd_data_freshness(&file, &state_dir, Some(24), false);
    assert!(result.is_ok());
}

#[test]
fn freshness_store_dir_json() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("store");
    std::fs::create_dir_all(&store).unwrap();
    std::fs::write(store.join("data.bin"), b"data").unwrap();
    let file = write_config(
        dir.path(),
        "version: \"1.0\"\nname: store_json\nmachines: {}\nresources: {}\n",
    );
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let result = cmd_data_freshness(&file, &state_dir, Some(24), true);
    assert!(result.is_ok());
}

// ── state lock freshness ─────────────────────────────────────────────

#[test]
fn freshness_with_state_lock_json() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        "version: \"1.0\"\nname: state_json\nmachines: {}\nresources: {}\n",
    );
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::fs::write(state_dir.join("forjar.lock.yaml"), "resources: {}\n").unwrap();
    let result = cmd_data_freshness(&file, &state_dir, Some(24), true);
    assert!(result.is_ok());
}

// ── default max age ──────────────────────────────────────────────────

#[test]
fn freshness_default_max_age() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        "version: \"1.0\"\nname: default\nmachines: {}\nresources: {}\n",
    );
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    // None uses default of 24h
    let result = cmd_data_freshness(&file, &state_dir, None, false);
    assert!(result.is_ok());
}

// ── multiple artifacts ───────────────────────────────────────────────

#[test]
fn freshness_multiple_artifacts_mixed() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.bin"), b"exists").unwrap();
    // b.bin doesn't exist
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: multi
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  data:
    type: file
    machine: m
    path: /tmp/data
    content: "x"
    output_artifacts:
      - a.bin
      - b.bin
"#,
    );
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let result = cmd_data_freshness(&file, &state_dir, Some(24), false);
    assert!(result.is_ok());
}

// ── combined store + artifacts + state lock ───────────────────────────

#[test]
fn freshness_all_sources() {
    let dir = tempfile::tempdir().unwrap();
    // Artifact
    std::fs::write(dir.path().join("out.bin"), b"output").unwrap();
    // Store dir
    let store = dir.path().join("store");
    std::fs::create_dir_all(&store).unwrap();
    std::fs::write(store.join("cache.db"), b"cache").unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: all
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  job:
    type: file
    machine: m
    path: /tmp/job
    content: "x"
    output_artifacts:
      - out.bin
"#,
    );
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::fs::write(state_dir.join("forjar.lock.yaml"), "resources: {}\n").unwrap();
    let result = cmd_data_freshness(&file, &state_dir, Some(24), true);
    assert!(result.is_ok());
}
