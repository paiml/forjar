//! forjar#469 (PMAT-161): the global lock's stamp is a per-name map.
//!
//! One `state/` dir serves N configs (paiml/infra keeps six machine manifests
//! against one dir). The lock's machine sections were already keyed by name;
//! only the single `name:` stamp collided, so every apply of a different config
//! re-stamped the dir and printed the GH-377 warning.

use super::*;
use std::path::Path;

/// Read the on-disk global lock as an untyped YAML value.
fn read_lock_yaml(state_dir: &Path) -> serde_yaml_ng::Value {
    let text = std::fs::read_to_string(global_lock_path(state_dir)).expect("read global lock");
    serde_yaml_ng::from_str(&text).expect("parse global lock")
}

fn results(machine: &str) -> Vec<(String, usize, usize, usize)> {
    vec![(machine.to_string(), 1_usize, 1_usize, 0_usize)]
}

#[test]
fn two_stacks_in_one_state_dir_both_get_a_stamp() {
    let dir = tempfile::tempdir().unwrap();
    update_global_lock(dir.path(), "alpha", &results("mini")).unwrap();
    update_global_lock(dir.path(), "beta", &results("lambda-labs")).unwrap();

    let yaml = read_lock_yaml(dir.path());
    let stacks = yaml.get("stacks").expect("stacks map present");
    assert!(stacks.get("alpha").is_some(), "alpha stamp kept: {yaml:?}");
    assert!(stacks.get("beta").is_some(), "beta stamp written: {yaml:?}");
    assert_eq!(
        yaml.get("name").and_then(|v| v.as_str()),
        Some("beta"),
        "top-level name is the stack that applied last"
    );
}

#[test]
fn legacy_single_stamp_lock_migrates_on_load() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        global_lock_path(dir.path()),
        "schema: '1.0'\nname: alpha\nlast_apply: '2026-09-01T00:00:00Z'\n\
         generator: forjar 1.24.0\nmachines:\n  mini:\n    resources: 2\n    \
         converged: 2\n    failed: 0\n    last_apply: '2026-09-01T00:00:00Z'\n",
    )
    .unwrap();

    let lock = load_global_lock(dir.path()).unwrap().expect("lock loads");
    save_global_lock(dir.path(), &lock).unwrap();

    let yaml = read_lock_yaml(dir.path());
    assert_eq!(yaml.get("schema").and_then(|v| v.as_str()), Some("1.1"));
    assert_eq!(yaml.get("name").and_then(|v| v.as_str()), Some("alpha"));
    let stamp = yaml
        .get("stacks")
        .and_then(|s| s.get("alpha"))
        .expect("legacy name became its own stamp");
    assert_eq!(
        stamp
            .get("machines")
            .and_then(|m| m.as_sequence())
            .map(Vec::len),
        Some(1),
        "the legacy stamp carries the machines it had: {stamp:?}"
    );
}

#[test]
fn unknown_schema_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        global_lock_path(dir.path()),
        "schema: '9.9'\nname: alpha\nlast_apply: x\ngenerator: forjar 9.9\nmachines: {}\n",
    )
    .unwrap();

    let err = load_global_lock(dir.path()).expect_err("unknown schema must fail closed");
    assert!(err.contains("9.9"), "message names the schema: {err}");
}

#[test]
fn round_trip_preserves_outputs() {
    let dir = tempfile::tempdir().unwrap();
    update_global_lock(dir.path(), "alpha", &results("mini")).unwrap();
    let mut outputs = indexmap::IndexMap::new();
    outputs.insert("endpoint".to_string(), "10.0.0.1".to_string());
    persist_outputs(dir.path(), "alpha", &outputs, false).unwrap();

    let lock = load_global_lock(dir.path()).unwrap().unwrap();
    assert_eq!(lock.outputs["endpoint"], "10.0.0.1");
    assert!(lock.machines.contains_key("mini"));
    assert_eq!(lock.name, "alpha");
    assert!(
        read_lock_yaml(dir.path()).get("stacks").is_some(),
        "the stamp map survives an outputs-only write"
    );
}
