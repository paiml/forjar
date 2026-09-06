//! forjar#469 (PMAT-161): the global lock's stamp is a per-name map.
//!
//! One `state/` dir serves N configs (paiml/infra keeps six machine manifests
//! against one dir). The lock's machine sections were already keyed by name;
//! only the single `name:` stamp collided, so every apply of a different config
//! re-stamped the dir and printed the GH-377 warning.
//!
//! The warning is an `eprintln!`, which a unit test cannot capture, so the
//! condition behind it lives in [`stack_written_from_other_file`] and these
//! tests assert on that function: warning fired ⟺ it returns `Some`.

use super::*;
use crate::core::state::stamp::{stack_conflict, StackConflict};
use std::path::{Path, PathBuf};

/// Read the on-disk global lock as an untyped YAML value.
fn read_lock_yaml(state_dir: &Path) -> serde_yaml_ng::Value {
    let text = std::fs::read_to_string(global_lock_path(state_dir)).expect("read global lock");
    serde_yaml_ng::from_str(&text).expect("parse global lock")
}

fn results(machine: &str) -> Vec<(String, usize, usize, usize)> {
    vec![(machine.to_string(), 1_usize, 1_usize, 0_usize)]
}

/// A config file that exists, so canonicalisation has something to resolve.
fn config_file(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(format!("{name}.yaml"));
    std::fs::write(&path, format!("version: '1.0'\nname: {name}\n")).unwrap();
    path
}

#[test]
fn two_stacks_in_one_state_dir_both_get_a_stamp() {
    let dir = tempfile::tempdir().unwrap();
    let alpha = config_file(dir.path(), "alpha");
    let beta = config_file(dir.path(), "beta");
    let state = dir.path().join("state");

    update_global_lock(&state, "alpha", Some(&alpha), &results("mini")).unwrap();
    update_global_lock(&state, "beta", Some(&beta), &results("lambda-labs")).unwrap();

    let yaml = read_lock_yaml(&state);
    let stacks = yaml.get("stacks").expect("stacks map present");
    assert!(stacks.get("alpha").is_some(), "alpha stamp kept: {yaml:?}");
    assert!(stacks.get("beta").is_some(), "beta stamp written: {yaml:?}");
    assert_eq!(
        yaml.get("name").and_then(|v| v.as_str()),
        Some("beta"),
        "top-level name is the stack that applied last"
    );

    let lock = load_global_lock(&state).unwrap().unwrap();
    assert_eq!(lock.stamp_for("alpha").unwrap().machines, vec!["mini"]);
    assert_eq!(
        lock.stamp_for("beta").unwrap().machines,
        vec!["lambda-labs"]
    );
    assert!(lock.machines.contains_key("mini") && lock.machines.contains_key("lambda-labs"));
    // The supported layout: a DIFFERENT name in the same dir is not the
    // GH-377 case and must not warn.
    assert_eq!(
        stack_written_from_other_file(&lock, &state, "beta", Some(&beta)),
        None
    );
    assert_eq!(
        stack_written_from_other_file(&lock, &state, "alpha", Some(&alpha)),
        None
    );
}

#[test]
fn same_name_from_another_file_is_the_gh377_case() {
    let dir = tempfile::tempdir().unwrap();
    let here = config_file(dir.path(), "here");
    let there = config_file(dir.path(), "there");
    let state = dir.path().join("state");

    update_global_lock(&state, "alpha", Some(&here), &results("mini")).unwrap();
    let lock = load_global_lock(&state).unwrap().unwrap();
    let previous = stack_written_from_other_file(&lock, &state, "alpha", Some(&there))
        .expect("same name, different -f: this is the case the warning is for");
    assert!(
        previous.ends_with("here.yaml"),
        "names the file it was applied from before: {previous}"
    );

    update_global_lock(&state, "alpha", Some(&there), &results("mini")).unwrap();
    let lock = load_global_lock(&state).unwrap().unwrap();
    assert!(
        lock.stamp_for("alpha")
            .unwrap()
            .file
            .as_deref()
            .unwrap()
            .ends_with("there.yaml"),
        "the apply re-stamps the entry with the file it came from"
    );
    assert_eq!(
        stack_written_from_other_file(&lock, &state, "alpha", Some(&there)),
        None,
        "re-applying from the same file is silent"
    );
}

/// Write a pre-fix (schema 1.0, single stamp) global lock by hand.
fn write_legacy_lock(state_dir: &Path) {
    std::fs::create_dir_all(state_dir).unwrap();
    std::fs::write(
        global_lock_path(state_dir),
        "schema: '1.0'\nname: alpha\nlast_apply: '2026-09-01T00:00:00Z'\n\
         generator: forjar 1.24.0\nmachines:\n  mini:\n    resources: 2\n    \
         converged: 2\n    failed: 0\n    last_apply: '2026-09-01T00:00:00Z'\n",
    )
    .unwrap();
}

#[test]
fn legacy_single_stamp_lock_migrates_on_load() {
    let dir = tempfile::tempdir().unwrap();
    write_legacy_lock(dir.path());

    let lock = load_global_lock(dir.path()).unwrap().expect("lock loads");
    let stamp = lock
        .stamp_for("alpha")
        .expect("the old name became a stamp");
    assert_eq!(stamp.file, None, "1.0 recorded no config file");
    assert_eq!(stamp.machines, vec!["mini"]);
    assert_eq!(stamp.last_apply, "2026-09-01T00:00:00Z");
    assert_eq!(stamp.generator, "forjar 1.24.0");
    // Nothing is dropped by the migration.
    assert_eq!(lock.name, "alpha");
    assert_eq!(lock.machines["mini"].resources, 2);
    // An unknown origin is not evidence of a mismatch: no warning on the
    // first apply after migration.
    let after = config_file(dir.path(), "alpha");
    assert_eq!(
        stack_written_from_other_file(&lock, dir.path(), "alpha", Some(&after)),
        None
    );

    save_global_lock(dir.path(), &lock).unwrap();
    let yaml = read_lock_yaml(dir.path());
    assert_eq!(yaml.get("schema").and_then(|v| v.as_str()), Some("1.1"));
    assert_eq!(yaml.get("name").and_then(|v| v.as_str()), Some("alpha"));
    assert!(
        yaml.get("stacks").and_then(|s| s.get("alpha")).is_some(),
        "the migrated stamp is written: {yaml:?}"
    );
}

#[test]
fn migration_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    write_legacy_lock(dir.path());

    let first = load_global_lock(dir.path()).unwrap().unwrap();
    let second = load_global_lock(dir.path()).unwrap().unwrap();
    assert_eq!(first.stacks, second.stacks, "a second load changes nothing");

    save_global_lock(dir.path(), &first).unwrap();
    let reloaded = load_global_lock(dir.path()).unwrap().unwrap();
    assert_eq!(reloaded.stacks, first.stacks);
    assert_eq!(reloaded.name, first.name);
    assert_eq!(reloaded.machines.len(), first.machines.len());
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
fn round_trip_preserves_every_field() {
    let dir = tempfile::tempdir().unwrap();
    let alpha = config_file(dir.path(), "alpha");
    let state = dir.path().join("state");
    update_global_lock(&state, "alpha", Some(&alpha), &results("mini")).unwrap();

    let mut outputs = indexmap::IndexMap::new();
    outputs.insert("endpoint".to_string(), "10.0.0.1".to_string());
    persist_outputs(&state, "alpha", &outputs, false).unwrap();

    let lock = load_global_lock(&state).unwrap().unwrap();
    assert_eq!(lock.outputs["endpoint"], "10.0.0.1");
    assert_eq!(lock.name, "alpha");
    assert_eq!(lock.schema, "1.1");
    assert_eq!(lock.machines["mini"].converged, 1);
    assert!(
        lock.stamp_for("alpha").is_some(),
        "the stamp map survives an outputs-only write"
    );

    save_global_lock(&state, &lock).unwrap();
    let again = load_global_lock(&state).unwrap().unwrap();
    assert_eq!(again.stacks, lock.stacks);
    assert_eq!(again.outputs, lock.outputs);
    assert_eq!(again.last_apply, lock.last_apply);
    assert_eq!(again.generator, lock.generator);
}

// ── machine ownership (the grill's hazard 1) ─────────────────────────

#[test]
fn a_machine_another_stack_owns_is_a_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let alpha = config_file(dir.path(), "alpha");
    let beta = config_file(dir.path(), "beta");
    let state = dir.path().join("state");

    update_global_lock(&state, "alpha", Some(&alpha), &results("mini")).unwrap();
    let lock = load_global_lock(&state).unwrap().unwrap();

    // Generations and machine locks are keyed by machine name alone, so beta
    // writing "mini" would overwrite alpha's history.
    assert_eq!(
        stack_conflict(&lock, &state, "beta", Some(&beta), &["mini".to_string()]),
        Some(StackConflict::MachineOwnedBy {
            machine: "mini".to_string(),
            stack: "alpha".to_string(),
        })
    );
    let sentence = stack_conflict(&lock, &state, "beta", Some(&beta), &["mini".to_string()])
        .unwrap()
        .to_string();
    assert!(
        sentence.contains("mini") && sentence.contains("alpha"),
        "{sentence}"
    );
    assert!(
        sentence.contains("`forjar undo` refuses this combination"),
        "keeps the GH-377 tail: {sentence}"
    );
    // Its own machines are not a conflict.
    assert_eq!(
        stack_conflict(
            &lock,
            &state,
            "beta",
            Some(&beta),
            &["lambda-labs".to_string()]
        ),
        None
    );
}

#[test]
fn a_stack_reapplying_its_own_machine_is_not_a_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let alpha = config_file(dir.path(), "alpha");
    let state = dir.path().join("state");

    update_global_lock(&state, "alpha", Some(&alpha), &results("mini")).unwrap();
    let lock = load_global_lock(&state).unwrap().unwrap();
    assert_eq!(
        stack_conflict(&lock, &state, "alpha", Some(&alpha), &["mini".to_string()]),
        None,
        "a stack re-applying the machine it owns is the normal case"
    );
}

#[test]
fn a_released_machine_can_be_claimed_by_another_stack() {
    let dir = tempfile::tempdir().unwrap();
    let alpha = config_file(dir.path(), "alpha");
    let beta = config_file(dir.path(), "beta");
    let state = dir.path().join("state");

    update_global_lock(&state, "alpha", Some(&alpha), &results("mini")).unwrap();
    // alpha applies again without mini: the machine is released.
    update_global_lock(&state, "alpha", Some(&alpha), &results("lambda-labs")).unwrap();

    let lock = load_global_lock(&state).unwrap().unwrap();
    assert_eq!(
        lock.stamp_for("alpha").unwrap().machines,
        vec!["lambda-labs"]
    );
    assert_eq!(
        stack_conflict(&lock, &state, "beta", Some(&beta), &["mini".to_string()]),
        None,
        "nobody owns mini any more"
    );
}

// ── per-stack outputs (the grill's hazard 2) ─────────────────────────

fn outputs_of(pairs: &[(&str, &str)]) -> indexmap::IndexMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

#[test]
fn outputs_merge_per_stack_instead_of_replacing() {
    let dir = tempfile::tempdir().unwrap();
    let alpha = config_file(dir.path(), "alpha");
    let beta = config_file(dir.path(), "beta");
    let state = dir.path().join("state");

    update_global_lock(&state, "alpha", Some(&alpha), &results("mini")).unwrap();
    persist_outputs(
        &state,
        "alpha",
        &outputs_of(&[("a_host", "10.0.0.1"), ("a_port", "22")]),
        false,
    )
    .unwrap();

    update_global_lock(&state, "beta", Some(&beta), &results("lambda-labs")).unwrap();
    persist_outputs(
        &state,
        "beta",
        &outputs_of(&[("b_host", "10.0.0.2")]),
        false,
    )
    .unwrap();

    let lock = load_global_lock(&state).unwrap().unwrap();
    assert_eq!(
        lock.outputs["a_host"], "10.0.0.1",
        "beta must not wipe alpha"
    );
    assert_eq!(lock.outputs["b_host"], "10.0.0.2");

    // alpha applies again having dropped a_port: its own stale key goes, the
    // other stack's key stays.
    persist_outputs(
        &state,
        "alpha",
        &outputs_of(&[("a_host", "10.0.0.9")]),
        false,
    )
    .unwrap();
    let lock = load_global_lock(&state).unwrap().unwrap();
    assert_eq!(lock.outputs["a_host"], "10.0.0.9");
    assert!(
        !lock.outputs.contains_key("a_port"),
        "dropped output withdrawn"
    );
    assert_eq!(lock.outputs["b_host"], "10.0.0.2");
    assert_eq!(lock.outputs.len(), 2, "{:?}", lock.outputs);
    assert_eq!(lock.stamp_for("alpha").unwrap().outputs, vec!["a_host"]);
    assert_eq!(lock.stamp_for("beta").unwrap().outputs, vec!["b_host"]);
}

// ── file identity (the grill's hazard 3) ─────────────────────────────

#[test]
fn the_stamped_file_is_relative_to_the_state_dir() {
    let dir = tempfile::tempdir().unwrap();
    let alpha = config_file(dir.path(), "alpha");
    let state = dir.path().join("state");
    update_global_lock(&state, "alpha", Some(&alpha), &results("mini")).unwrap();

    let lock = load_global_lock(&state).unwrap().unwrap();
    let file = lock.stamp_for("alpha").unwrap().file.clone().unwrap();
    assert_eq!(
        file, "../alpha.yaml",
        "stored relative to the state dir, so another checkout of the same \
         layout is the same stack"
    );
    assert_eq!(
        stack_written_from_other_file(&lock, &state, "alpha", Some(&alpha)),
        None
    );
}

/// PMAT-183: asked with the CLONE's state dir, because a checkout moves its
/// config and its state together — that is what "the same layout" means. The
/// original dir with this config is `-f` in one tree and `--state-dir` in
/// another, which is the wrong-stack shape and does warn.
#[test]
fn the_same_layout_in_another_checkout_is_the_same_stack() {
    let one = tempfile::tempdir().unwrap();
    let alpha = config_file(one.path(), "alpha");
    let state = one.path().join("state");
    update_global_lock(&state, "alpha", Some(&alpha), &results("mini")).unwrap();
    let lock = load_global_lock(&state).unwrap().unwrap();

    // A second clone of the same tree: same relative layout, different root.
    let two = tempfile::tempdir().unwrap();
    let two_state = two.path().join("state");
    let elsewhere = config_file(two.path(), "alpha");
    assert_eq!(
        stack_written_from_other_file(&lock, &two_state, "alpha", Some(&elsewhere)),
        None,
        "same layout, another checkout — not a wrong-stack warning"
    );

    let unrelated = config_file(two.path(), "beta");
    assert!(
        stack_written_from_other_file(&lock, &two_state, "alpha", Some(&unrelated)).is_some(),
        "a genuinely different config still trips it"
    );
}

#[test]
fn a_migrated_stamp_matches_any_file_once_then_pins() {
    let dir = tempfile::tempdir().unwrap();
    write_legacy_lock(&dir.path().join("state"));
    let state = dir.path().join("state");
    let alpha = config_file(dir.path(), "alpha");

    let lock = load_global_lock(&state).unwrap().unwrap();
    assert_eq!(
        stack_conflict(&lock, &state, "alpha", Some(&alpha), &["mini".to_string()]),
        None,
        "the one-apply window: an unknown origin matches any -f"
    );

    update_global_lock(&state, "alpha", Some(&alpha), &results("mini")).unwrap();
    let lock = load_global_lock(&state).unwrap().unwrap();
    let other = config_file(dir.path(), "other");
    assert!(
        lock.stamp_for("alpha").unwrap().file.is_some(),
        "that apply pinned it"
    );
    assert!(stack_written_from_other_file(&lock, &state, "alpha", Some(&other)).is_some());
}

// ── a rename is one lineage (PMAT-161 S2) ────────────────────────────

#[test]
fn a_rename_retires_the_old_names_stamp_into_the_new_one() {
    let dir = tempfile::tempdir().unwrap();
    // ONE file, applied under two names in turn: that is a rename, and the
    // file is the only evidence of it the lock has.
    let file = config_file(dir.path(), "alpha");
    let state = dir.path().join("state");

    update_global_lock(&state, "alpha", Some(&file), &results("mini")).unwrap();
    persist_outputs(
        &state,
        "alpha",
        &outputs_of(&[("a_host", "10.0.0.1")]),
        false,
    )
    .unwrap();
    update_global_lock(&state, "alpha2", Some(&file), &results("mini")).unwrap();

    let lock = load_global_lock(&state).unwrap().unwrap();
    assert_eq!(
        stack_names(&lock),
        ["alpha2"],
        "the old name's stamp must be retired, or one config's dir counts as \
         two stacks and its own restore is refused for ever"
    );
    let stamp = lock.stamp_for("alpha2").unwrap();
    assert_eq!(stamp.machines, ["mini"], "the machines move with the name");
    assert_eq!(
        stamp.outputs,
        ["a_host"],
        "and so does ownership of the outputs, which are still in the map"
    );
    assert_eq!(lock.outputs["a_host"], "10.0.0.1");
    assert_eq!(
        multi_stack_restore_refusal(&lock, "state", "generation 1"),
        None,
        "after the retirement the multi-stack count is honest"
    );
}

#[test]
fn a_different_file_taking_a_renamed_stacks_machine_still_conflicts() {
    let dir = tempfile::tempdir().unwrap();
    let file = config_file(dir.path(), "alpha");
    let other = config_file(dir.path(), "other");
    let state = dir.path().join("state");

    update_global_lock(&state, "alpha", Some(&file), &results("mini")).unwrap();
    update_global_lock(&state, "alpha2", Some(&file), &results("mini")).unwrap();
    let lock = load_global_lock(&state).unwrap().unwrap();

    // Anti-vacuity: retiring a rename must not retire a genuinely different
    // stack. alpha3 comes from another file and would write alpha2's machine.
    assert_eq!(
        stack_conflict(&lock, &state, "alpha3", Some(&other), &["mini".to_string()]),
        Some(StackConflict::MachineOwnedBy {
            machine: "mini".to_string(),
            stack: "alpha2".to_string(),
        })
    );
    update_global_lock(&state, "alpha3", Some(&other), &results("mini")).unwrap();
    let lock = load_global_lock(&state).unwrap().unwrap();
    assert_eq!(
        stack_names(&lock),
        ["alpha2", "alpha3"],
        "a different file is a second stack, not a rename: both stamps stand"
    );
}
