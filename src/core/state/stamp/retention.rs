//! PMAT-177 + PMAT-182: the one question a retention sweep asks before it
//! deletes anything.
//!
//! forjar trims a state dir in TWO places — the named snapshots
//! (`cli::apply_snapshot::gc_old_snapshots`) and the numbered generations
//! (`cli::generation::gc_generations`) — and both count what is IN THE DIR
//! against a keep count promised by ONE config's `policy.snapshot_generations`.
//! Neither directory records an owner, so in a dir several stacks share, one
//! stack's apply deleted its neighbours' copies: the pre-apply snapshots and
//! the generations an operator reaches for to recover from exactly this class
//! of mistake.
//!
//! PMAT-177 guarded the snapshot half and PMAT-182 found the other one still
//! open a day later, which is the argument for this module existing at all:
//! the condition lives in ONE function beside the stamps it reads, so a third
//! retention path inherits it and the two that exist cannot drift apart again.
//!
//! Skipping is the safe direction — a dir that grows is recoverable, a deleted
//! generation is not — and the skip is STATED rather than done quietly,
//! because an operator who set `snapshot_generations: 10` and finds forty
//! directories is owed the reason and the ticket that will fix it.

use std::path::Path;

/// How many stacks this state dir holds, when that is more than one.
///
/// Read from the same record `stack_conflict` and the restore refusal read, so
/// the three cannot disagree about how many stacks a dir has. A dir with no
/// global lock, an unreadable one and a single-stack dir all answer `None` —
/// the ordinary case, unchanged.
fn shared_stack_count(state_dir: &Path) -> Option<usize> {
    let lock = crate::core::state::load_global_lock(state_dir)
        .ok()
        .flatten()?;
    let stacks = super::stack_names(&lock).len();
    (stacks > 1).then_some(stacks)
}

/// `Some(note)` when `what` retention must delete NOTHING in this dir, and the
/// line to print saying so; `None` when the dir has one owner and the sweep
/// proceeds exactly as it did before either guard existed.
///
/// Returned rather than printed so the caller owns its stderr and a unit test
/// can read the sentence — the two callers print it verbatim, one line, once
/// per sweep.
#[must_use]
pub fn skip_note(state_dir: &Path, what: &str) -> Option<String> {
    let stacks = shared_stack_count(state_dir)?;
    Some(format!(
        "note: {what} gc skipped: state dir holds {stacks} stacks (PMAT-162)"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::state::{new_global_lock, save_global_lock};
    use crate::core::types::StackStamp;

    /// Write a global lock naming exactly `stacks` stacks.
    fn lock_with(state_dir: &Path, stacks: &[&str]) {
        let mut lock = new_global_lock(stacks[0]);
        for name in stacks {
            lock.stacks.insert(
                (*name).to_string(),
                StackStamp {
                    file: Some(format!("../{name}/forjar.yaml")),
                    last_apply: "2026-01-01T00:00:00Z".to_string(),
                    generator: "forjar test".to_string(),
                    machines: vec![(*name).to_string()],
                    outputs: Vec::new(),
                },
            );
        }
        save_global_lock(state_dir, &lock).unwrap();
    }

    #[test]
    fn no_lock_and_one_stack_are_not_shared() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(skip_note(dir.path(), "snapshot"), None, "no global lock");
        lock_with(dir.path(), &["alpha"]);
        assert_eq!(skip_note(dir.path(), "snapshot"), None, "one stack");
    }

    /// The sentence itself, because both callers print it verbatim and the
    /// binary rows match on its text: the count and the ticket are the two
    /// things it owes the operator.
    #[test]
    fn a_shared_dir_is_told_the_count_and_the_ticket() {
        let dir = tempfile::tempdir().unwrap();
        lock_with(dir.path(), &["alpha", "bravo"]);
        assert_eq!(
            skip_note(dir.path(), "generation").as_deref(),
            Some("note: generation gc skipped: state dir holds 2 stacks (PMAT-162)")
        );
        assert_eq!(
            skip_note(dir.path(), "snapshot").as_deref(),
            Some("note: snapshot gc skipped: state dir holds 2 stacks (PMAT-162)")
        );
    }
}
