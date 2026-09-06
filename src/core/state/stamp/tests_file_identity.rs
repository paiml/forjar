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
