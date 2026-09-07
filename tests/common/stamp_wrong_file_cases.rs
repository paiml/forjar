//! PMAT-183 (S2, round-2 refuters): the wrong-stack guard was still asking the
//! suffix question PMAT-175 removed from the rename path.
//!
//! Split out of `falsification_state_stamp_per_name.rs` for the 500-line cap
//! only; these are rows of that suite and run in its binary. Nothing here is
//! reachable on its own — `tests/common/` is not a test target.
//!
//! THE DEFECT. A stamp records its `-f` RELATIVE to the state dir
//! (`../forjar.yaml`), and PMAT-175 made the comparison exact — by writing the
//! path again for the state dir in hand and comparing the two strings — for
//! the reader that DELETES on a match (`rename::retire_renamed`). It could not
//! do the same for `stack_conflict`, which was called without a state dir and
//! therefore fell back to `identity::unattributed_match`: the `..` components
//! dropped, the current path asked whether it ENDS WITH the rest.
//!
//! So the guard that exists to catch "one of -f/--state-dir points at the
//! wrong stack" matched `../forjar.yaml` against every
//! `machines/<m>/forjar.yaml` in the paiml/infra layout and stayed SILENT on
//! the one shape it is for: the same `name:` applied from a different file.
//! Both its callers — `state::update_global_lock` and
//! `cli::state_identity::check_state_dir_owner` — hold the state dir already,
//! so the fallback was never load-bearing, only unasked.

use crate::harness::*;

/// (n) PMAT-183. The SAME name from a config that only shares the recorded
/// basename is a wrong-stack apply: `apply` warns, `undo` refuses.
///
/// `<root>/forjar.yaml` is stamped as `../forjar.yaml`;
/// `<root>/machines/mini/forjar.yaml` carries the same `name:` and is a
/// different file. Before the fix the tail matched, the guard exempted it, and
/// the operator's mistake was recorded in silence — with `undo` then replaying
/// the root stack's generations against the manifest's resources, which is the
/// exact defect GH-377 and this suite exist for.
#[test]
fn the_same_name_from_a_file_sharing_the_recorded_basename_warns_and_undo_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let state = root.join("state");
    let top = write_stack(&root, ".", "shared", "box", "one");
    let (rc, out) = apply(&top, &state);
    assert_eq!(rc, 0, "the root stack's apply must succeed:\n{out}");
    assert_eq!(
        lock_of(&state)
            .stamp_for("shared")
            .and_then(|s| s.file.clone())
            .as_deref(),
        Some("../forjar.yaml"),
        "fixture: the stamp must record the RELATIVE form, which is the one a \
         suffix comparison cannot tell apart"
    );

    // ANTI-VACUITY, first: the ordinary repeat apply from the recorded path is
    // silent. An exact comparison that warned here would be worse than the
    // loose one it replaces — every apply of every stack would warn.
    let (rc, out) = apply(&top, &state);
    assert_eq!(rc, 0, "the repeat apply must succeed:\n{out}");
    assert!(
        warnings(&out).is_empty(),
        "re-applying a stack from the very file its stamp records warned:\n{out}"
    );

    // The wrong file, under the same name. `undo` first: the apply below
    // re-stamps the entry, and after that there is nothing left to disagree.
    let mini = write_stack(&root, "machines/mini", "shared", "box", "one");
    let (rc, out) = undo(&mini, &state);
    assert_ne!(
        rc, 0,
        "undo replayed the root stack's generations against a manifest that \
         only shares its basename:\n{out}"
    );
    assert!(
        out.contains("was last applied from"),
        "the refusal must carry the same sentence apply warns with; got:\n{out}"
    );
    assert!(
        out.contains("../forjar.yaml") || out.contains("forjar.yaml"),
        "the refusal must name the file the stamp recorded; got:\n{out}"
    );

    let (rc, out) = apply(&mini, &state);
    assert_eq!(rc, 0, "apply warns here, it does not refuse:\n{out}");
    assert!(
        out.contains("was last applied from"),
        "apply must warn that this name was last applied from another file — \
         the tail matched and the mistake was recorded in silence; got:\n{out}"
    );
}

/// ANTI-VACUITY, second and stronger: the three-stack fleet — every stack from
/// its own file, applied through one state dir — still warns about NOTHING.
///
/// The exact comparison is asked on every apply of every stack, so a mistake
/// in it turns the supported paiml/infra layout back into the warning-on-every
/// -apply defect that #469 fixed.
#[test]
fn the_supported_fleet_layout_is_still_silent_under_the_exact_comparison() {
    let (f, warned) = fleet();
    assert!(
        warned.is_empty(),
        "the exact wrong-file comparison warned on a correct multi-stack \
         apply:\n{}",
        warned.join("\n")
    );
    let (rc, out) = apply(&f.bravo, &f.state);
    assert_eq!(rc, 0, "the repeat apply must succeed:\n{out}");
    assert!(
        warnings(&out).is_empty(),
        "a second apply of one fleet member warned:\n{out}"
    );
}
