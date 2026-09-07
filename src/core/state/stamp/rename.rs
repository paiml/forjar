//! PMAT-161 (S2): a RENAME of a stack is one lineage.
//!
//! The same config file applied under a new `name:` used to leave the old
//! name's stamp in `GlobalLock::stacks`, so a dir ONE config had ever been
//! applied to "held 2 stacks" — and `multi_stack_restore_refusal` counts
//! stamps, so the renamed stack's own `undo` was refused for ever, by the very
//! apply the refusal names as the remedy. The apply now retires the old entry
//! into the new name; everything that decides WHETHER an entry is a rename
//! (the recorded config file) lives in [`super::identity`].

use super::identity::records_config_file;
use super::StackStamp;
use crate::core::types::GlobalLock;
use std::path::Path;

/// PMAT-161 (S2): retire the stamps this apply RENAMES — entries under another
/// name recording the very config file now being applied — and return what they
/// owned. One file is one stack, so such a stamp is that stack's OLD name, and
/// leaving it behind made a dir ONE config had applied to hold two stamps:
/// `multi_stack_restore_refusal` counts stamps, so the renamed stack's own
/// `undo` was refused for ever by the apply the refusal named as the remedy.
///
/// EVERY match, not the first. `.find()` left any second entry recording this
/// file standing, and one is reachable without a defect anywhere: a dir carries
/// `alpha` from before the rename and `alpha-old` written by an older forjar,
/// both recording the same `-f`. The count `multi_stack_restore_refusal`
/// refuses on would then stay above one for ever, which is the condition this
/// whole retirement exists to clear.
///
/// Evidence is positive only, as in [`super::machine_owner`] — a stamp with no
/// file (1.0) is left alone, and the comparison is the EXACT one (PMAT-175):
/// this function DELETES what it matches, so a suffix match here retires
/// another stack's entry and takes its machines with it. A `note:`, not a
/// `warning:`: this is the rename `stack_conflict` has always exempted.
pub(super) fn retire_renamed(
    lock: &mut GlobalLock,
    state_dir: &Path,
    config: (&str, Option<&Path>),
) -> Vec<StackStamp> {
    let (config_name, config_file) = config;
    let renamed: Vec<String> = lock
        .stacks
        .iter()
        .filter(|(stack, stamp)| {
            stack.as_str() != config_name && records_config_file(stamp, state_dir, config_file)
        })
        .map(|(stack, _)| stack.clone())
        .collect();
    let mut retired = Vec::new();
    for old in renamed {
        let Some(stamp) = lock.stacks.shift_remove(&old) else {
            continue;
        };
        eprintln!(
            "note: stack '{old}' renamed to '{config_name}' in {}",
            state_dir.display()
        );
        retired.push(stamp);
    }
    retired
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
/// previous entry and any retired predecessors (a rename) owned.
pub(super) fn next_stamp(
    previous: Option<&StackStamp>,
    retired: &[StackStamp],
    stamped_file: Option<String>,
    written: Vec<String>,
    declared: Option<&[String]>,
    when: (&str, &str), // (last_apply, generator)
) -> StackStamp {
    let (last_apply, generator) = when;
    StackStamp {
        // A caller with no `-f` keeps the existing record instead of erasing
        // the only evidence `stack_conflict` has.
        file: stamped_file
            .or_else(|| previous.and_then(|s| s.file.clone()))
            .or_else(|| retired.iter().find_map(|s| s.file.clone())),
        machines: claimed_machines(previous, retired, written, declared),
        outputs: inherited_outputs(previous, retired),
        last_apply: last_apply.to_string(),
        generator: generator.to_string(),
    }
}

/// PMAT-176: the machines this stamp claims — (previous ∪ written ∪ retired)
/// ∩ declared.
///
/// The intersection is what makes a release deliberate: a machine leaves the
/// stack when the CONFIG stops declaring it, not when one `apply -m other`
/// happens not to converge it. Without the union, that scoped apply rewrote the
/// set as the single machine it ran and handed the rest to whichever sibling
/// stack asked for them next.
///
/// `declared` is `None` when the caller made no claim about the config (every
/// unit caller, and any future caller with no config in hand); then the set is
/// what it always was — this apply's machines, plus a retired predecessor's.
fn claimed_machines(
    previous: Option<&StackStamp>,
    retired: &[StackStamp],
    written: Vec<String>,
    declared: Option<&[String]>,
) -> Vec<String> {
    let mut machines = written;
    if declared.is_some() {
        machines = union(machines, previous.map_or(&[], |s| s.machines.as_slice()));
    }
    for stamp in retired {
        machines = union(machines, &stamp.machines);
    }
    match declared {
        Some(declared) => {
            machines.retain(|m| declared.contains(m));
            machines
        }
        None => machines,
    }
}

/// The output keys this stamp owns: its own, plus any retired predecessor's.
fn inherited_outputs(previous: Option<&StackStamp>, retired: &[StackStamp]) -> Vec<String> {
    let mut outputs = previous.map(|s| s.outputs.clone()).unwrap_or_default();
    for stamp in retired {
        outputs = union(outputs, &stamp.outputs);
    }
    outputs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(of: &[&str]) -> Vec<String> {
        of.iter().map(|s| (*s).to_string()).collect()
    }

    fn stamp_owning(machines: &[&str]) -> StackStamp {
        StackStamp {
            file: None,
            last_apply: "2026-09-06T00:00:00Z".to_string(),
            generator: "forjar test".to_string(),
            machines: names(machines),
            outputs: Vec::new(),
        }
    }

    fn machines_of(previous: &[&str], written: &[&str], declared: Option<&[&str]>) -> Vec<String> {
        let previous = stamp_owning(previous);
        let declared = declared.map(names);
        claimed_machines(Some(&previous), &[], names(written), declared.as_deref())
    }

    /// PMAT-176. The scoped apply: one machine converged, both still declared,
    /// so both are still this stack's.
    #[test]
    fn a_declared_machine_this_apply_skipped_is_still_owned() {
        assert_eq!(
            machines_of(
                &["mini", "lambda-labs"],
                &["mini"],
                Some(&["mini", "lambda-labs"])
            ),
            names(&["mini", "lambda-labs"]),
        );
    }

    /// The release, and the only one: the config no longer declares it.
    #[test]
    fn a_machine_the_config_stopped_declaring_is_released() {
        assert_eq!(
            machines_of(&["mini", "lambda-labs"], &["mini"], Some(&["mini"])),
            names(&["mini"]),
        );
    }

    /// No claim about the config — every unit caller — is the old behaviour:
    /// the stamp records what this apply wrote.
    #[test]
    fn without_a_declaration_the_stamp_is_what_the_apply_wrote() {
        assert_eq!(
            machines_of(&["mini", "lambda-labs"], &["mini"], None),
            names(&["mini"])
        );
    }

    /// A rename brings the old name's machines with it, still bounded by what
    /// the config declares.
    #[test]
    fn a_retired_predecessors_machines_move_and_are_still_bounded() {
        let retired = [stamp_owning(&["lambda-labs", "gone"])];
        assert_eq!(
            claimed_machines(
                None,
                &retired,
                names(&["mini"]),
                Some(&names(&["mini", "lambda-labs"]))
            ),
            names(&["mini", "lambda-labs"]),
        );
    }
}
