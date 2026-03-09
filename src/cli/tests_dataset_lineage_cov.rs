//! Coverage tests for dataset_lineage.rs — cmd_dataset_lineage text/json,
//! data resources, empty datasets, dependency edges.

use std::path::Path;

fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
    let file = dir.join("forjar.yaml");
    std::fs::write(&file, yaml).unwrap();
    file
}

// ── basic text mode with data resource ──────────────────────────

#[test]
fn lineage_text_with_data_resource() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: lineage-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  etl:
    type: task
    machine: local
    command: "/usr/bin/etl"
    tags:
      - "data"
      - "pipeline"
"#,
    );
    let result = super::dataset_lineage::cmd_dataset_lineage(&file, false);
    assert!(result.is_ok());
}

// ── json mode ───────────────────────────────────────────────────

#[test]
fn lineage_json_with_data_resource() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: lineage-json
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  etl:
    type: task
    machine: local
    command: "/usr/bin/etl"
    tags:
      - "dataset"
"#,
    );
    let result = super::dataset_lineage::cmd_dataset_lineage(&file, true);
    assert!(result.is_ok());
}

// ── empty dataset (no data resources) ───────────────────────────

#[test]
fn lineage_empty_text() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: no-data
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  nginx:
    type: file
    machine: local
    path: /tmp/x
    content: "hello"
"#,
    );
    let result = super::dataset_lineage::cmd_dataset_lineage(&file, false);
    assert!(result.is_ok());
}

#[test]
fn lineage_empty_json() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        "version: \"1.0\"\nname: empty\nmachines: {}\nresources: {}\n",
    );
    let result = super::dataset_lineage::cmd_dataset_lineage(&file, true);
    assert!(result.is_ok());
}

// ── resource with source file ───────────────────────────────────

#[test]
fn lineage_with_source_file() {
    let dir = tempfile::tempdir().unwrap();
    // Create a source file for hashing
    std::fs::write(dir.path().join("input.csv"), "col1,col2\na,b\n").unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: source-lineage
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  ingest:
    type: task
    machine: local
    command: "/usr/bin/ingest"
    source: input.csv
    tags:
      - "data"
"#,
    );
    let result = super::dataset_lineage::cmd_dataset_lineage(&file, false);
    assert!(result.is_ok());
}

// ── resource with output_artifacts ──────────────────────────────

#[test]
fn lineage_with_output_artifacts() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("output.parquet"), "fake-parquet-data").unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: artifact-lineage
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  transform:
    type: task
    machine: local
    command: "/usr/bin/transform"
    output_artifacts:
      - output.parquet
    tags:
      - "transform"
"#,
    );
    let result = super::dataset_lineage::cmd_dataset_lineage(&file, true);
    assert!(result.is_ok());
}

// ── dependency edges ────────────────────────────────────────────

#[test]
fn lineage_with_edges() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: edge-lineage
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  ingest:
    type: task
    machine: local
    command: "/usr/bin/ingest"
    tags:
      - "data"
  transform:
    type: task
    machine: local
    command: "/usr/bin/transform"
    depends_on:
      - ingest
    tags:
      - "pipeline"
"#,
    );
    let result = super::dataset_lineage::cmd_dataset_lineage(&file, false);
    assert!(result.is_ok());
}

// ── resource_group detection ────────────────────────────────────

#[test]
fn lineage_resource_group_data() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: group-lineage
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  db-backup:
    type: task
    machine: local
    command: "/usr/bin/backup"
    resource_group: data-pipeline
"#,
    );
    let result = super::dataset_lineage::cmd_dataset_lineage(&file, true);
    assert!(result.is_ok());
}

// ── nonexistent source file ─────────────────────────────────────

#[test]
fn lineage_missing_source_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: missing-src
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  ingest:
    type: task
    machine: local
    command: "/usr/bin/ingest"
    source: nonexistent.csv
    tags:
      - "data"
"#,
    );
    let result = super::dataset_lineage::cmd_dataset_lineage(&file, false);
    assert!(result.is_ok());
}

// ── ml tag detection ────────────────────────────────────────────

#[test]
fn lineage_ml_tag() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: ml-lineage
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  train:
    type: task
    machine: local
    command: "/usr/bin/train"
    tags:
      - "ml"
"#,
    );
    let result = super::dataset_lineage::cmd_dataset_lineage(&file, true);
    assert!(result.is_ok());
}
