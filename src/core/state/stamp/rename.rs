//! PMAT-161 (S2): a RENAME of a stack is one lineage.
//!
//! The same config file applied under a new `name:` used to leave the old
//! name's stamp in `GlobalLock::stacks`, so a dir ONE config had ever been
//! applied to "held 2 stacks" — and `multi_stack_restore_refusal` counts
//! stamps, so the renamed stack's own `undo` was refused for ever, by the very
//! apply the refusal names as the remedy. The apply now retires the old entry
//! into the new name; everything that decides WHETHER an entry is a rename
//! (the recorded config file) lives in the parent module.

use super::{records_config_file, StackStamp};
use crate::core::types::GlobalLock;
use std::path::Path;

/// PMAT-161 (S2): retire the stamp this apply RENAMES — an entry under another
/// name recording the very config file now being applied — and return what it
/// owned. One file is one stack, so such a stamp is that stack's OLD name, and
/// leaving it behind made a dir ONE config had applied to hold two stamps:
/// `multi_stack_restore_refusal` counts stamps, so the renamed stack's own
/// `undo` was refused for ever by the apply the refusal named as the remedy.
/// Evidence is positive only, as in [`machine_owner`] — a stamp with no file
/// (1.0) is left alone. A `note:`, not a `warning:`: this is the rename
/// [`stack_conflict`] has always exempted.
pub(super) fn retire_renamed(
    lock: &mut GlobalLock,
    state_dir: &Path,
    config: (&str, Option<&Path>),
) -> Option<StackStamp> {
    let (config_name, config_file) = config;
    let old = lock
        .stacks
        .iter()
        .find(|(stack, stamp)| {
            stack.as_str() != config_name && records_config_file(stamp, config_file)
        })
        .map(|(stack, _)| stack.clone())?;
    let stamp = lock.stacks.shift_remove(&old)?;
    eprintln!(
        "note: stack '{old}' renamed to '{config_name}' in {}",
        state_dir.display()
    );
    Some(stamp)
}

/// `first`, then whatever of `second` it does not already hold.
fn union(first: Vec<String>, second: &[String]) -> Vec<String> {
    let mut merged = first;
    for item in second {
        if !merged.contains(item) {
            merged.push(item.clone());
        }
    }
    merged
}

/// The stamp this apply writes: its own `-f` and machines, plus what its own
/// previous entry and a retired predecessor (a rename) owned.
pub(super) fn next_stamp(
    previous: Option<&StackStamp>,
    retired: Option<StackStamp>,
    stamped_file: Option<String>,
    machines: Vec<String>,
    when: (&str, &str), // (last_apply, generator)
) -> StackStamp {
    let (last_apply, generator) = when;
    let (old_file, old_machines, old_outputs) = match retired {
        Some(s) => (s.file, s.machines, s.outputs),
        None => (None, Vec::new(), Vec::new()),
    };
    StackStamp {
        // A caller with no `-f` keeps the existing record instead of erasing
        // the only evidence `stack_conflict` has.
        file: stamped_file
            .or_else(|| previous.and_then(|s| s.file.clone()))
            .or(old_file),
        machines: union(machines, &old_machines),
        outputs: union(
            previous.map(|s| s.outputs.clone()).unwrap_or_default(),
            &old_outputs,
        ),
        last_apply: last_apply.to_string(),
        generator: generator.to_string(),
    }
}
