//! PMAT-161 (PMAT-175): which recorded `-f` names the config being applied.
//!
//! The predicate decides whether an entry under ANOTHER name is this stack's
//! old one (a rename, to be retired) or a second stack (to be left alone), so
//! being loose here is not a missed warning: `retire_renamed` DELETES the entry
//! it matches, and with it that stack's machines and output keys.
//!
//! A stamp stores its config RELATIVE to the state dir (`../forjar.yaml`) when
//! the two share a tree. The comparison was made by dropping the `..`
//! components and asking whether the current path ENDS WITH the rest, so
//! `../forjar.yaml` matched every `machines/<m>/forjar.yaml` in the paiml/infra
//! layout — the review lane's executed reproducer, in which applying
//! `machines/mini/forjar.yaml` retired the root stack.
//!
//! In its own file because `stamp/mod.rs` is at the 500-line ceiling.

use super::*;
use crate::core::state::{load_global_lock, update_global_lock};
use std::path::{Path, PathBuf};

fn results(machine: &str) -> Vec<(String, usize, usize, usize)> {
    vec![(machine.to_string(), 1_usize, 1_usize, 0_usize)]
}

/// A config file that exists at `<dir>/<rel>`, so canonicalisation resolves.
fn config_at(root: &Path, rel: &str) -> PathBuf {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "version: '1.0'\n").unwrap();
    path
}

/// The reproducer, at the level the damage happens: two configs whose basenames
/// agree and whose paths do not are two stacks, and the second apply must not
/// retire the first.
#[test]
fn a_config_sharing_a_basename_does_not_retire_the_root_stack() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let state = root.join("state");
    let top = config_at(root, "forjar.yaml");
    let mini = config_at(root, "machines/mini/forjar.yaml");

    update_global_lock(&state, "root-stack", Some(&top), &results("box")).unwrap();
    update_global_lock(&state, "mini", Some(&mini), &results("mini-box")).unwrap();

    let lock = load_global_lock(&state).unwrap().unwrap();
    assert_eq!(
        stack_names(&lock),
        ["root-stack", "mini"],
        "`../forjar.yaml` matched `../machines/mini/forjar.yaml` by its tail, so \
         the root stack was retired as if it had been renamed"
    );
    assert_eq!(
        lock.stamp_for("root-stack").unwrap().machines,
        ["box"],
        "and the machines of the retired stamp were folded into the wrong stack"
    );
}

// ── the predicate itself ─────────────────────────────────────────────

/// The reproducer at its smallest: the recorded path and the config in hand
/// share a basename and nothing else.
#[test]
fn a_shared_basename_in_another_directory_is_not_the_same_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let state = root.join("state");
    let mini = config_at(root, "machines/mini/forjar.yaml");
    assert!(
        !same_config_file("../forjar.yaml", &state, &mini),
        "`../forjar.yaml` names <root>/forjar.yaml, not every */forjar.yaml \
         underneath it"
    );
}

/// And the case the relative form exists for: the SAME file, recorded and
/// applied through the same state dir.
#[test]
fn the_path_this_state_dir_would_record_is_the_same_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let state = root.join("state");
    let top = config_at(root, "forjar.yaml");
    let mini = config_at(root, "machines/mini/forjar.yaml");
    assert!(same_config_file("../forjar.yaml", &state, &top));
    assert!(same_config_file(
        "../machines/mini/forjar.yaml",
        &state,
        &mini
    ));
    assert!(
        !same_config_file("../machines/mini/forjar.yaml", &state, &top),
        "and the comparison is symmetric — neither direction is a suffix match"
    );
}

/// Portability, which is what the relative form is FOR: the same layout in
/// another checkout — the config and the state dir moving together — is the
/// same stack.
#[test]
fn the_same_layout_in_another_checkout_is_still_the_same_file() {
    let two = tempfile::tempdir().unwrap();
    let elsewhere = config_at(two.path(), "forjar.yaml");
    assert!(same_config_file(
        "../forjar.yaml",
        &two.path().join("state"),
        &elsewhere
    ));
}

/// A stamp with NO recorded file is not evidence of anything, and in
/// particular is not evidence of a rename: `retire_renamed` would otherwise
/// delete a 1.0 migration's stamp on the next apply.
#[test]
fn a_file_less_stamp_never_matches() {
    let dir = tempfile::tempdir().unwrap();
    let top = config_at(dir.path(), "forjar.yaml");
    let stamp = StackStamp {
        file: None,
        last_apply: "2026-09-06T00:00:00Z".to_string(),
        generator: "forjar test".to_string(),
        machines: vec!["box".to_string()],
        outputs: Vec::new(),
    };
    assert!(!records_config_file(
        &stamp,
        &dir.path().join("state"),
        Some(&top)
    ));
    // PMAT-183: and there is no looser reading of it left to try — the branch
    // that answered without a state dir is gone, so a file-less stamp is
    // "origin unknown" to every reader, whichever dir it is asked about.
    let other = tempfile::tempdir().unwrap();
    assert!(!records_config_file(
        &stamp,
        &other.path().join("state"),
        Some(&top)
    ));
}

// ── the wrong-stack guard asks the same question (PMAT-183) ──────────

/// PMAT-183. PMAT-175 left ONE reader on the old comparison — [`stack_conflict`]
/// was called without a state dir, so a relative recorded path could not be
/// resolved and was matched by the part it names. The row that stood here
/// PINNED that fallback ("without a state dir the comparison is still by the
/// part it names") and said tightening it would be a deliberate change with a
/// failing test. This is that change, and this is that test.
///
/// The pair is the reproducer's: `../forjar.yaml` recorded, and a
/// `machines/mini/forjar.yaml` carrying the SAME name. The tail matched, the
/// guard exempted it, and a genuinely wrong `-f` was recorded in silence.
#[test]
fn the_wrong_file_guard_does_not_match_by_the_part_it_names() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let state = root.join("state");
    let top = config_at(root, "forjar.yaml");
    let mini = config_at(root, "machines/mini/forjar.yaml");
    update_global_lock(&state, "shared", Some(&top), &results("box")).unwrap();
    let lock = load_global_lock(&state).unwrap().unwrap();

    assert_eq!(
        stack_conflict(&lock, &state, "shared", Some(&mini), &["box".to_string()]),
        Some(StackConflict::OtherFile {
            old: "../forjar.yaml".to_string()
        }),
        "the same name from a file that only shares the recorded basename is \
         the GH-377 case, and the guard was silent on it"
    );
}

/// ANTI-VACUITY: the file the stamp actually records is never a conflict, so
/// the tightened comparison does not warn on every ordinary apply — which is
/// precisely what the relative form exists to prevent.
#[test]
fn the_file_the_stamp_records_is_never_a_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let state = root.join("state");
    let top = config_at(root, "forjar.yaml");
    update_global_lock(&state, "shared", Some(&top), &results("box")).unwrap();
    let lock = load_global_lock(&state).unwrap().unwrap();

    assert_eq!(
        stack_conflict(&lock, &state, "shared", Some(&top), &["box".to_string()]),
        None,
        "re-applying a stack from the very file its stamp records must stay silent"
    );
}
