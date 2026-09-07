//! PMAT-182 (S1, round-2 refuters): the SECOND retention path.
//!
//! Split out of `falsification_state_stamp_per_name.rs` for the 500-line cap
//! only; these are rows of that suite and run in its binary. Nothing here is
//! reachable on its own — `tests/common/` is not a test target.
//!
//! THE DEFECT. PMAT-177 taught `apply_snapshot::gc_old_snapshots` not to delete
//! what it cannot attribute, but forjar trims a state dir in TWO places, and
//! `generation::gc_generations` was the other one: it sorts the numbered
//! generation dirs and removes the oldest until `policy.snapshot_generations`
//! remain, with no notion of which stack wrote which number. Generations are
//! numbered per state dir, so in the paiml/infra layout the twelfth apply by
//! `mini` deletes the generations `lambda-labs` would have undone to — the
//! same defect PMAT-177 fixed, one directory over, and reachable from three
//! callers (`apply`, `destroy`, and the `generation gc` verb) rather than one.
//!
//! The guard therefore sits at the PRIMITIVE and both retention paths ask ONE
//! helper (`state::stamp::retention::skip_note`), so the two cannot drift apart
//! again the way they did here.

use crate::harness::*;

/// The keep count every `write_stack` config sets — `policy.snapshot_generations`
/// is at once the on-switch and the retention count, so the fixture must drive
/// MORE applies than this or the sweep never runs and the row is vacuous.
const KEEP: usize = 10;

/// (m) PMAT-182. Generation retention does not delete a dir it cannot
/// attribute, and says so once per apply.
#[test]
fn generation_gc_is_skipped_while_the_state_dir_holds_several_stacks() {
    let (f, _) = fleet();
    let mut notes = Vec::new();
    let mut held = generation_numbers(&f.state);

    for _ in 0..=KEEP {
        let (rc, out) = apply(&f.alpha, &f.state);
        assert_eq!(rc, 0, "the apply itself must still succeed:\n{out}");
        notes.push(
            out.lines()
                .filter(|l| l.contains("generation gc skipped"))
                .count(),
        );
        let now = generation_numbers(&f.state);
        assert!(
            now.starts_with(&held),
            "alpha's apply deleted a generation out of a dir bravo and charlie \
             also undo in: {held:?} -> {now:?}\n{out}"
        );
        held = now;
    }

    assert!(
        held.len() > KEEP,
        "fixture: the dir must pass the keep count, or nothing was ever swept: {held:?}"
    );
    assert!(
        notes.iter().all(|&n| n <= 1),
        "the skip is stated once per sweep, not once per generation it kept: {notes:?}"
    );
    let last = apply(&f.alpha, &f.state).1;
    assert!(
        last.contains("note: generation gc skipped: state dir holds 3 stacks (PMAT-162)"),
        "the skip must be stated, name the count and name the ticket that makes \
         retention per-stack; got:\n{last}"
    );
}

/// ANTI-VACUITY: a dir with one stack still prunes to its keep count, exactly
/// as it did before the guard — a retention sweep that refuses to run at all
/// satisfies the row above and is a worse defect than the one it fixes.
#[test]
fn generation_gc_still_prunes_a_single_stack_dir() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let state = root.join("state");
    let solo = write_stack(&root, "solo", "solo", "mini", "one");

    for _ in 0..=KEEP {
        let (rc, out) = apply(&solo, &state);
        assert_eq!(rc, 0, "the single-stack apply failed:\n{out}");
    }
    let (rc, last) = apply(&solo, &state);
    assert_eq!(rc, 0, "the single-stack apply failed:\n{last}");

    let held = generation_numbers(&state);
    assert!(
        !last.contains("generation gc skipped"),
        "a single-stack dir must not skip its own gc:\n{last}"
    );
    assert_eq!(
        held.len(),
        KEEP,
        "the single-stack sweep kept {} generations against a keep count of \
         {KEEP} — retention stopped working: {held:?}",
        held.len()
    );
    assert!(
        !held.contains(&0),
        "the oldest generation must still be the one that goes: {held:?}"
    );
}
