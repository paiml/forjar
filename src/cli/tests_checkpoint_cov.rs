//! Coverage tests for checkpoint.rs — tag matching, machine filter, GC edge cases.

use super::checkpoint::*;
use std::path::Path;

fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
    let file = dir.join("forjar.yaml");
    std::fs::write(&file, yaml).unwrap();
    file
}

// ── tag-based checkpoint detection ───────────────────────────────────

#[test]
fn checkpoint_tag_training() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("data.bin"), b"training data").unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: tags
machines:
  gpu:
    hostname: gpu
    addr: 127.0.0.1
resources:
  data:
    type: file
    machine: gpu
    path: /tmp/data
    content: "x"
    tags: [training]
    output_artifacts:
      - data.bin
"#,
    );
    let result = cmd_checkpoint(&file, None, false, 5, false);
    assert!(result.is_ok());
}

#[test]
fn checkpoint_tag_ml() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: ml
machines:
  gpu:
    hostname: gpu
    addr: 127.0.0.1
resources:
  model:
    type: file
    machine: gpu
    path: /tmp/m
    content: "x"
    tags: [ml]
    output_artifacts:
      - missing-artifact.bin
"#,
    );
    // Artifact doesn't exist — should still work (exists=false path)
    let result = cmd_checkpoint(&file, None, false, 5, false);
    assert!(result.is_ok());
}

#[test]
fn checkpoint_tag_checkpoint() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: ck
machines:
  gpu:
    hostname: gpu
    addr: 127.0.0.1
resources:
  ckpt:
    type: file
    machine: gpu
    path: /tmp/c
    content: "x"
    tags: [checkpoint, experiment-1]
    output_artifacts:
      - nonexistent.pt
"#,
    );
    let result = cmd_checkpoint(&file, None, false, 5, true);
    assert!(result.is_ok());
}

#[test]
fn checkpoint_resource_group() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("weights.bin"), b"w").unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: grp
machines:
  gpu:
    hostname: gpu
    addr: 127.0.0.1
resources:
  w:
    type: file
    machine: gpu
    path: /tmp/w
    content: "x"
    resource_group: checkpoints
    output_artifacts:
      - weights.bin
"#,
    );
    let result = cmd_checkpoint(&file, None, false, 5, false);
    assert!(result.is_ok());
}

// ── machine filter ───────────────────────────────────────────────────

#[test]
fn checkpoint_machine_filter_match() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.pt"), b"model").unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: filter
machines:
  gpu:
    hostname: gpu
    addr: 127.0.0.1
  cpu:
    hostname: cpu
    addr: 127.0.0.1
resources:
  m1:
    type: model
    machine: gpu
    name: llama
    source: /models/llama.gguf
    format: gguf
    output_artifacts:
      - a.pt
  m2:
    type: model
    machine: cpu
    name: bert
    source: /models/bert
    format: onnx
    output_artifacts:
      - a.pt
"#,
    );
    // Filter to gpu only
    let result = cmd_checkpoint(&file, Some("gpu"), false, 5, false);
    assert!(result.is_ok());
}

#[test]
fn checkpoint_machine_filter_no_match() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: nomatch
machines:
  gpu:
    hostname: gpu
    addr: 127.0.0.1
resources:
  m1:
    type: model
    machine: gpu
    name: llama
    source: /models/llama.gguf
    format: gguf
    output_artifacts:
      - x.pt
"#,
    );
    // Filter to nonexistent machine
    let result = cmd_checkpoint(&file, Some("nonexistent"), false, 5, false);
    assert!(result.is_ok());
}

// ── GC edge cases ────────────────────────────────────────────────────

#[test]
fn checkpoint_gc_json() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.pt"), b"old weights").unwrap();
    std::fs::write(dir.path().join("b.pt"), b"new weights data").unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: gcjson
machines:
  gpu:
    hostname: gpu
    addr: 127.0.0.1
resources:
  m:
    type: model
    machine: gpu
    name: llama
    source: /models/llama.gguf
    format: gguf
    output_artifacts:
      - a.pt
      - b.pt
"#,
    );
    let result = cmd_checkpoint(&file, None, true, 1, true);
    assert!(result.is_ok());
}

#[test]
fn checkpoint_gc_keep_all() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.pt"), b"data").unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: keepall
machines:
  gpu:
    hostname: gpu
    addr: 127.0.0.1
resources:
  m:
    type: model
    machine: gpu
    name: llama
    source: /models/llama.gguf
    format: gguf
    output_artifacts:
      - a.pt
"#,
    );
    // keep > total — nothing removed
    let result = cmd_checkpoint(&file, None, true, 100, false);
    assert!(result.is_ok());
}

#[test]
fn checkpoint_gc_empty() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        "version: \"1.0\"\nname: empty\nmachines: {}\nresources: {}\n",
    );
    let result = cmd_checkpoint(&file, None, true, 5, false);
    assert!(result.is_ok());
}

// ── non-checkpoint resources are skipped ─────────────────────────────

#[test]
fn checkpoint_skips_non_checkpoint() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: skip
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
    // No checkpoint resources — should produce empty list
    let result = cmd_checkpoint(&file, None, false, 5, false);
    assert!(result.is_ok());
}

// ── text output with existing + missing artifacts ────────────────────

#[test]
fn checkpoint_text_mixed_exists() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("exists.pt"), b"real checkpoint data here").unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: mixed
machines:
  gpu:
    hostname: gpu
    addr: 127.0.0.1
resources:
  m:
    type: model
    machine: gpu
    name: llama
    source: /models/llama.gguf
    format: gguf
    output_artifacts:
      - exists.pt
      - missing.pt
"#,
    );
    let result = cmd_checkpoint(&file, None, false, 5, false);
    assert!(result.is_ok());
}

#[test]
fn checkpoint_json_mixed_exists() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.pt"), b"checkpoint").unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: mixed_json
machines:
  gpu:
    hostname: gpu
    addr: 127.0.0.1
resources:
  m:
    type: model
    machine: gpu
    name: llama
    source: /models/llama.gguf
    format: gguf
    output_artifacts:
      - a.pt
      - b.pt
"#,
    );
    let result = cmd_checkpoint(&file, None, false, 5, true);
    assert!(result.is_ok());
}
