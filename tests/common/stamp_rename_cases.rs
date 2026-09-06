//! PMAT-161 (S2): a RENAME of a stack, and what `undo` does across one.
//!
//! Split out of `falsification_state_stamp_per_name.rs` for the 500-line cap
//! only; these are rows of that suite and run in its binary. Nothing here is
//! reachable on its own — `tests/common/` is not a test target.
//!
//! The two rows are one story. PMAT-171 made the RENAME one lineage: the apply
//! that finds a stamp recording the very `-f` it is being applied from retires
//! that stamp into the new name. PMAT-172 is the hole that left: `undo` replays
//! the TARGET generation's recorded config, whose `name:` is the historical
//! one, so the replay re-stamped the dir under the old name and the dir held
//! two stamps again — one real undo was all it took to resurrect the very
//! condition the retirement removed.

use crate::harness::*;

/// (g) PMAT-161 S2. A RENAME is one lineage, not two stacks.
///
/// The same config file applied under a new `name:` used to leave the old
/// name's stamp in `stacks`, so a dir one operator has ever applied one config
/// to "held 2 stacks" — and the multi-stack refusal above, which counts
/// exactly that, refused the renamed stack's own undo for ever. The documented
/// remedy ("run `forjar apply` once to re-stamp") is what CREATED the second
/// entry, so it could not be undone by repeating it.
///
/// The apply that finds a stamp recording the very `-f` it is being applied
/// from retires that stamp into the new name — machines and outputs move with
/// it — and says so in one `note:` line. It is not a warning: nothing is
/// wrong, and this is the rename `stack_conflict` has always exempted.
#[test]
fn a_renamed_stack_is_one_lineage_not_two_stacks() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let state = root.join("state");
    let alpha = write_stack(&root, "alpha", "alpha", "mini", "one");
    assert_eq!(apply(&alpha, &state).0, 0, "the first apply must succeed");

    // The rename: same file, same machine, same resource ids — only `name:`
    // and the content the undo below has to revert.
    rename_stack(&alpha, "alpha", "alpha-renamed", "two");
    let (rc, out) = apply(&alpha, &state);
    assert_eq!(rc, 0, "the apply after a rename failed:\n{out}");
    assert!(
        out.contains("note: stack 'alpha' renamed to 'alpha-renamed'"),
        "the retirement must be stated once, as a note; got:\n{out}"
    );
    assert!(
        warnings(&out).is_empty(),
        "a rename is not a wrong-stack condition and must not warn:\n{out}"
    );

    let lock = lock_of(&state);
    let names: Vec<&str> = lock.stacks.keys().map(String::as_str).collect();
    assert_eq!(
        names,
        ["alpha-renamed"],
        "the old name's stamp must be RETIRED, not left beside the new one — \
         it is what makes a one-config dir count as two stacks"
    );
    assert_eq!(
        lock.stamp_for("alpha-renamed").unwrap().machines,
        ["mini"],
        "the machines the old name owned move to the new one"
    );

    assert_eq!(
        marker(&root, "alpha"),
        "two\n",
        "fixture: the apply moved the host"
    );
    let (rc, out) = undo(&alpha, &state);
    assert_eq!(
        rc, 0,
        "a renamed stack's own undo was refused as a multi-stack restore:\n{out}"
    );
    assert_eq!(
        marker(&root, "alpha"),
        "one\n",
        "the undo was allowed but reverted nothing:\n{out}"
    );

    // ANTI-VACUITY. The retirement retires ONE entry — the one recording this
    // `-f` — and a genuinely second stack still makes every restore refuse.
    //
    // In a dir of its own, deliberately: the undo above replays the target
    // generation's recorded config, which stamps this dir under the stack's
    // HISTORICAL name with no `-f` recorded (`WithheldStampFile`), and a stamp
    // with no file is not evidence of a rename (see `retire_renamed`). Asserted
    // here that state would make "still refuses" true for the wrong reason.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let state = root.join("state");
    let alpha = write_stack(&root, "alpha", "alpha", "mini", "one");
    assert_eq!(apply(&alpha, &state).0, 0, "the first apply must succeed");
    rename_stack(&alpha, "alpha", "alpha-renamed", "two");
    assert_eq!(apply(&alpha, &state).0, 0, "the rename apply must succeed");
    let bravo = write_stack(&root, "bravo", "bravo", "lambda-labs", "one");
    assert_eq!(apply(&bravo, &state).0, 0, "bravo's apply must succeed");
    // One more apply by the renamed stack, so the generation `undo` targets
    // differs from the live state — otherwise undo reports "already at
    // generation N" and returns before the refusal it is here to prove.
    rename_stack(&alpha, "alpha-renamed", "alpha-renamed", "three");
    assert_eq!(apply(&alpha, &state).0, 0, "the fourth apply must succeed");

    let (rc, out) = undo(&alpha, &state);
    assert_ne!(
        rc, 0,
        "a dir holding two real stacks must still refuse:\n{out}"
    );
    assert!(
        out.contains("PMAT-162") && out.contains("'alpha-renamed'") && out.contains("'bravo'"),
        "the refusal must still name both stacks and the ticket; got:\n{out}"
    );
}

/// (h) PMAT-161 S2 / PMAT-172. An `undo` across a rename stamps the dir as the
/// INVOKING stack, not as the name the replayed generation happens to carry.
///
/// THE DEFECT, found reviewing PMAT-171. `undo` re-converges by applying the
/// TARGET generation's RECORDED config, and that config's `name:` is the
/// historical one — `alpha`, not `alpha2`. The rollback that precedes the
/// replay also restores that generation's `forjar.lock.yaml`, whose only stamp
/// is `alpha`. So the replay wrote `stacks['alpha']` beside it (with no `-f`,
/// because `WithheldStampFile` withheld the staged temp path, and a file-less
/// stamp is not positive evidence of a rename), and the dir held TWO stamps
/// again: `multi_stack_restore_refusal` counts stamps, so the NEXT `undo` of a
/// stack that has only ever been itself was refused — one real undo undid the
/// retirement PMAT-171 added.
///
/// The replay is an implementation detail of the command the operator ran. It
/// therefore stamps as the stack that ran it, with the `-f` that ran it.
#[test]
fn an_undo_across_a_rename_stamps_the_invoking_stack_not_the_replayed_name() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let state = root.join("state");
    let alpha = write_stack(&root, "alpha", "alpha", "mini", "one");
    assert_eq!(apply(&alpha, &state).0, 0, "the first apply must succeed");

    // The rename: same file, same machine, same resource ids — only `name:`,
    // and the content, so the undo below has something real to revert.
    rename_stack(&alpha, "alpha", "alpha2", "two");
    let (rc, out) = apply(&alpha, &state);
    assert_eq!(rc, 0, "the apply after the rename failed:\n{out}");
    assert_eq!(
        stack_names(&state),
        ["alpha2"],
        "fixture: PMAT-171's retirement must have left exactly one stamp"
    );
    assert_eq!(
        marker(&root, "alpha"),
        "two\n",
        "fixture: the apply moved the host"
    );

    let (rc, out) = undo(&alpha, &state);
    assert_eq!(rc, 0, "the renamed stack's own undo was refused:\n{out}");
    assert_eq!(
        marker(&root, "alpha"),
        "one\n",
        "the undo was allowed but reverted nothing:\n{out}"
    );

    // THE ROW. Not "an undo happened" — WHOSE NAME the dir is stamped with
    // afterwards. The generation the replay restored carries `alpha`.
    let lock = lock_of(&state);
    assert_eq!(
        stack_names(&state),
        ["alpha2"],
        "the replay re-stamped the dir under the generation's HISTORICAL name, \
         so a dir one config has ever been applied to holds two stamps again — \
         and the next undo is refused as a multi-stack restore"
    );
    assert_eq!(
        lock.name, "alpha2",
        "the top-level stamp names whoever applied last, and that is the stack \
         the operator invoked, not the name inside the replayed generation"
    );
    assert!(
        lock.stamp_for("alpha2").unwrap().file.is_some(),
        "the replay must keep the stack's recorded `-f` — the staged temp config \
         is deleted seconds later, and losing the record re-opens forjar#469"
    );

    // AND THE CYCLE REPEATS. The guard never fires for a stack that has only
    // ever been itself: one more apply, one more undo, still exit 0.
    rename_stack(&alpha, "alpha2", "alpha2", "three");
    let (rc, out) = apply(&alpha, &state);
    assert_eq!(rc, 0, "the apply after the undo failed:\n{out}");
    assert!(
        warnings(&out).is_empty(),
        "the apply after an undo warned — the replay left a stamp it should not \
         have:\n{out}"
    );
    let (rc, out) = undo(&alpha, &state);
    assert_eq!(rc, 0, "the second undo was refused:\n{out}");
    assert!(
        !out.contains("PMAT-162"),
        "a single-stack dir must never reach the multi-stack refusal:\n{out}"
    );
    assert_eq!(
        stack_names(&state),
        ["alpha2"],
        "and the dir is still one stack after the second round trip"
    );
}
