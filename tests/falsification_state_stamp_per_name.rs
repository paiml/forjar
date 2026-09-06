//! forjar#469 (PMAT-161): ONE state dir, N stacks, keyed by config name.
//!
//! THE DEFECT, measured 2026-09-05 on 1.24.0 → 1.25.2 (found by paiml/infra#442).
//! paiml/infra keeps one `state/` dir for six machine manifests
//! (`machines/<m>/forjar.yaml`, each with its own `name:` and its own machine),
//! and every `machines/*/Makefile` applies with `--state-dir state`. The lock
//! file has always been keyed by config name for its machine sections, but the
//! STAMP — "which stack last applied here" — was a single value, so every apply
//! re-stamped the dir as a different stack:
//!
//! ```text
//! $ forjar apply -f machines/mini/forjar.yaml --state-dir state
//! warning: state dir state was last applied by stack 'forjar-clean-room';
//!          this apply re-stamps it as 'mini' … `forjar undo` refuses this combination
//! ```
//!
//! Every one of those applies touched only its own lock section. The warning
//! was a false positive on EVERY apply of a layout forjar supports, and the
//! `undo` it threatened refused that layout for real.
//!
//! WHAT THIS SUITE PINS. The stamp is a map, `stacks: {name -> StackStamp}`,
//! and both `apply` (warns) and `undo` (refuses) ask ONE question of it —
//! `state::stack_conflict`, so they cannot disagree about what "wrong stack"
//! means. N names with their own machines are silent and undoable; the two
//! conditions that are genuinely dangerous still fire:
//!
//! * the SAME name from a DIFFERENT `-f` (GH-377's case), and
//! * a machine another stack owns, because generations and per-machine locks
//!   are keyed by machine name alone.
//!
//! Every assertion here is at the binary level, because the defect was only
//! visible as stderr text and a lock file on disk.

#[path = "common/stack_stamp_harness.rs"]
mod harness;

// PMAT-161 (S2). The rename rows live in their own file for the 500-line cap
// and run in THIS binary — `tests/common/` is not a test target, so a plain
// `mod` beside the suite would have built a second one.
#[path = "common/stamp_rename_cases.rs"]
mod rename_cases;

// PMAT-182 (S1, round 2). The second retention path — `gc_generations`, which
// PMAT-177 left unguarded — in its own file for the same reason.
#[path = "common/stamp_gc_cases.rs"]
mod gc_cases;

// PMAT-183 (S2, round 2). The wrong-stack guard's own comparison — the one
// reader PMAT-175 left on the suffix match — likewise.
#[path = "common/stamp_wrong_file_cases.rs"]
mod wrong_file_cases;

use harness::*;

/// (a) THE BLOCKER. Three configs, one state dir, zero warnings — and the lock
/// holds three stamps, not one that keeps being overwritten.
#[test]
fn three_stacks_through_one_state_dir_warn_about_nothing() {
    let (f, warned) = fleet();
    assert!(
        warned.is_empty(),
        "the supported six-stack layout warned on a correct apply (forjar#469):\n{}",
        warned.join("\n"),
    );

    let lock = lock_of(&f.state);
    let mut names: Vec<&str> = lock.stacks.keys().map(String::as_str).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        ["alpha", "bravo", "charlie"],
        "each config must have its OWN stamp; the single stamp is what made every \
         apply look like a different stack"
    );
    assert_eq!(lock.stamp_for("alpha").unwrap().machines, ["mini"]);
    assert_eq!(lock.stamp_for("bravo").unwrap().machines, ["lambda-labs"]);
    assert_eq!(lock.stamp_for("charlie").unwrap().machines, ["clean-room"]);
}

/// (a) continued: a state dir holding MORE THAN ONE stack refuses EVERY
/// restore, and refuses it before writing a byte.
///
/// Generations are numbered per STATE DIR and the restore is WHOLE-DIR:
/// `generation::rollback_to_generation` empties the dir (bar `generations/`
/// and the snapshot dirs) and copies one generation back over it. So "undo
/// alpha" in a dir shared with bravo and charlie reverts bravo's and
/// charlie's machine locks too — here to a generation that is bravo's own
/// apply. Per-name stamps (#469) fixed WHO the dir belongs to; they did not
/// make the restore stack-scoped, and a guard is the honest interim state.
///
/// The test that stood here asserted the opposite — that undoing one stack
/// left the other two untouched — and passed only because it drove two extra
/// applies of the undone stack first, making the target generation that
/// stack's OWN. It measured apply ordering, not scoping: undo any generation
/// another stack wrote and the other stacks go back with it.
///
/// All four doors onto the primitive are checked, because a guard placed in
/// `cmd_undo` would leave `rollback --generation` wide open.
#[test]
fn a_state_dir_holding_several_stacks_refuses_every_restore() {
    let (f, _) = fleet();
    // A fourth apply, by alpha, so every generation `undo` can target differs
    // from the live state — otherwise undo reports "already at generation N"
    // and never reaches the restore this is about.
    write_stack(&f.root, "alpha", "alpha", "mini", "two");
    assert_eq!(apply(&f.alpha, &f.state).0, 0);

    let before = fingerprint(&f.state);
    for (label, (rc, out)) in [
        ("undo --yes", undo(&f.alpha, &f.state)),
        (
            "undo --generations 3 --yes",
            undo_generations(&f.alpha, &f.state, "3"),
        ),
        ("undo --resume --yes", undo_resume(&f.alpha, &f.state)),
        // The refusal is about the DIR, not about who asks: bravo owns its own
        // machine here and is refused for the same reason alpha is.
        ("undo --yes, from bravo", undo(&f.bravo, &f.state)),
        ("rollback --generation 0 --yes", rollback(&f.state, "0")),
    ] {
        assert_ne!(
            rc, 0,
            "{label} restored a state dir holding three stacks, reverting the other \
             two with it:\n{out}"
        );
        assert!(
            out.contains("PMAT-162"),
            "{label}: the refusal must name the ticket that makes restore \
             stack-scoped, or it is a dead end; got:\n{out}"
        );
        assert!(
            out.contains("bravo") && out.contains("charlie"),
            "{label}: the refusal must list the stacks it would have reverted; \
             got:\n{out}"
        );
        assert!(
            !out.contains("--force"),
            "{label}: this refusal has no override — offering one is the defect \
             with a flag on it; got:\n{out}"
        );
        assert_eq!(
            fingerprint(&f.state),
            before,
            "{label} refused, but had already written the state dir:\n{out}"
        );
    }

    assert_eq!(
        marker(&f.root, "alpha"),
        "two\n",
        "a refused restore still converged the host"
    );
    assert_eq!(
        marker(&f.root, "bravo"),
        "one\n",
        "a refused restore moved bravo"
    );
    assert_eq!(
        marker(&f.root, "charlie"),
        "one\n",
        "a refused restore moved charlie"
    );
}

/// (a) continued, and the part the fingerprint above cannot see: WHERE in
/// `undo` the refusal sits.
///
/// `cmd_undo` destroys the resources the target generation does not hold —
/// `undo_prune::destroy_absent_from_target`, on the host, with the CURRENT
/// config's definitions — and only then calls the restore. A guard at the
/// restore therefore fires after the destroy has already run. The test above
/// does not catch it because its target generations drop nothing: alpha's
/// resource set never changes there, only its content.
///
/// So this fixture makes the target generation drop a resource. alpha applies
/// a SECOND file resource over a dir whose earlier generations were written
/// without it; `undo` onto one of those generations wants `alpha_extra` gone.
/// The refusal must arrive first, and `extra.txt` must still be on disk
/// afterwards — a command that destroys and then refuses is INV-REFUSAL-IS-
/// BEFORE-THE-FIRST-BYTE violated, the defect with an error message on it.
#[test]
fn a_refused_undo_destroys_nothing_on_the_way_to_refusing() {
    let (f, _) = fleet();
    let alpha = write_stack_plus_extra(&f.root, "alpha", "alpha", "mini", "one");
    let (rc, out) = apply(&alpha, &f.state);
    assert_eq!(rc, 0, "apply of alpha's second resource failed:\n{out}");

    let extra = extra_marker(&f.root, "alpha");
    assert_eq!(
        std::fs::read_to_string(&extra).ok().as_deref(),
        Some("extra\n"),
        "fixture: the resource the undo would destroy must exist first"
    );
    let before = fingerprint(&f.state);

    let (rc, out) = undo(&alpha, &f.state);
    assert_ne!(
        rc, 0,
        "undo in a dir holding three stacks was not refused:\n{out}"
    );
    assert!(
        out.contains("PMAT-162"),
        "the refusal must name the ticket that makes restore stack-scoped; got:\n{out}"
    );
    assert!(
        out.contains("bravo") && out.contains("charlie"),
        "the refusal must list the stacks it would have reverted; got:\n{out}"
    );
    assert_eq!(
        std::fs::read_to_string(&extra).ok().as_deref(),
        Some("extra\n"),
        "undo DESTROYED a resource and then refused the restore — the guard is \
         below the destroy:\n{out}"
    );
    assert_eq!(
        fingerprint(&f.state),
        before,
        "undo refused, but the destroy had already rewritten the state dir:\n{out}"
    );
}

/// (b) The GH-377 case survives, narrowed: the SAME name from a DIFFERENT `-f`.
/// `apply` warns (it does what its arguments say); `undo` refuses (its plan and
/// its work would be about different stacks).
#[test]
fn the_same_name_from_another_config_file_warns_and_undo_refuses() {
    let (f, _) = fleet();
    // A copy of alpha at another path, with its own resource — the wrong-stack
    // case: one name, two configs.
    let copy = write_stack(&f.root, "alpha-copy", "alpha", "mini", "one");

    let (rc, out) = undo(&copy, &f.state);
    assert_ne!(rc, 0, "undo from a different -f for the same name:\n{out}");
    assert!(
        out.contains("was last applied from"),
        "the refusal must carry the same sentence apply warns with; got:\n{out}"
    );
    assert!(
        out.contains(&f.state.display().to_string()),
        "the refusal must print the absolute state dir; got:\n{out}"
    );

    let (rc, out) = apply(&copy, &f.state);
    assert_eq!(rc, 0, "apply must not refuse, only warn:\n{out}");
    assert!(
        out.contains("was last applied from"),
        "apply must still warn on the same name from another -f; got:\n{out}"
    );
}

/// (c) The machine-ownership half. Generations and per-machine locks are keyed
/// by machine name ALONE, so a second stack writing another stack's machine
/// overwrites its history — the same wrong-stack condition, in a different
/// disguise.
#[test]
fn a_machine_another_stack_owns_warns_and_undo_refuses() {
    let (f, _) = fleet();
    // delta is a new NAME (so nothing above fires) that names alpha's machine.
    let delta = write_stack(&f.root, "delta", "delta", "mini", "one");

    let (rc, out) = apply(&delta, &f.state);
    assert_eq!(rc, 0, "apply must not refuse, only warn:\n{out}");
    assert!(
        out.contains("which stack 'alpha' owns"),
        "apply must warn that delta writes alpha's machine; got:\n{out}"
    );

    let (rc, out) = undo(&delta, &f.state);
    assert_ne!(
        rc, 0,
        "undo replayed generations keyed by a machine another stack owns:\n{out}"
    );
    assert!(
        out.contains("which stack 'alpha' owns"),
        "the refusal must carry the same sentence apply warns with; got:\n{out}"
    );
}

/// (d) MIGRATION. A dir carrying the pre-#469 single stamp — written here by
/// hand, in the old format, because a fixture generated by this binary could
/// not be the format that shipped — migrates on the first apply that reads it,
/// silently, and comes out as 1.1 with the old stamp under its own name.
#[test]
fn a_legacy_single_stamp_dir_migrates_on_the_next_apply() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let state = root.join("state");
    let alpha = write_stack(&root, "alpha", "alpha", "mini", "one");
    assert_eq!(apply(&alpha, &state).0, 0);

    let legacy = r#"schema: "1.0"
name: alpha
last_apply: "2026-01-01T00:00:00Z"
generator: forjar 1.24.0
machines:
  mini:
    resources: 1
    converged: 1
    failed: 0
    last_apply: "2026-01-01T00:00:00Z"
"#;
    std::fs::write(state.join("forjar.lock.yaml"), legacy).unwrap();
    // The sidecar is BLAKE3 of the bytes we just replaced. A stale one is a
    // hard apply failure by design (FJ-1270), and re-sealing is not what this
    // test is about; a MISSING sidecar is the documented soft case.
    let _ = std::fs::remove_file(state.join("forjar.lock.yaml.b3"));

    let (rc, out) = apply(&alpha, &state);
    assert_eq!(rc, 0, "apply against a legacy state dir failed:\n{out}");
    assert!(
        warnings(&out).is_empty(),
        "the upgrade itself must be quiet — a migrated stamp records no file, and \
         'origin unknown' is not evidence of a mismatch:\n{out}"
    );

    let lock = lock_of(&state);
    assert_eq!(lock.schema, "1.1", "the migration must be persisted");
    assert_eq!(
        lock.stamp_for("alpha").unwrap().machines,
        ["mini"],
        "the old single stamp must become the entry for its own name"
    );
}

/// (e) `status` attributes each machine to the stack that wrote it, from the
/// stacks map — not all of them to whichever stack happened to apply last.
#[test]
fn status_names_each_machine_under_its_own_stack() {
    let (f, _) = fleet();
    let (rc, out) = run(&["status", "--state-dir", &f.state.display().to_string()]);
    assert_eq!(rc, 0, "status failed:\n{out}");

    for (machine, stack) in [
        ("mini", "alpha"),
        ("lambda-labs", "bravo"),
        ("clean-room", "charlie"),
    ] {
        assert_eq!(
            stack_shown_for(&out, machine).as_deref(),
            Some(stack),
            "status must name {machine} under stack {stack}; got:\n{out}"
        );
    }
}

/// The stack `status` printed under a machine's block, if any.
fn stack_shown_for(out: &str, machine: &str) -> Option<String> {
    let mut lines = out
        .lines()
        .skip_while(|l| !l.starts_with(&format!("Machine: {machine} ")));
    lines.next()?;
    lines
        .take_while(|l| !l.starts_with("Machine: "))
        .find_map(|l| l.trim().strip_prefix("Stack: ").map(str::to_string))
}

/// (f) OVER-CORRECTION GUARD. One config, one state dir — the overwhelmingly
/// common case — is untouched by all of the above: no warning, undo allowed,
/// and `status` prints no per-stack attribution at all.
#[test]
fn a_single_stack_dir_behaves_exactly_as_before() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let state = root.join("state");
    let alpha = write_stack(&root, "alpha", "alpha", "mini", "one");

    for content in ["one", "two"] {
        write_stack(&root, "alpha", "alpha", "mini", content);
        let (rc, out) = apply(&alpha, &state);
        assert_eq!(rc, 0, "apply failed:\n{out}");
        assert!(
            warnings(&out).is_empty(),
            "single-stack apply warned:\n{out}"
        );
    }

    let (rc, out) = undo(&alpha, &state);
    assert_eq!(rc, 0, "a normal single-stack undo was refused:\n{out}");
    assert_eq!(marker(&root, "alpha"), "one\n", "the undo did not revert");

    // Inherited from the multi-stack test this file used to end on: `undo`
    // replays the target generation from a config staged in a temp sibling
    // file, and a stamp that recorded THAT path would pin the stack to a file
    // deleted seconds later — so the operator's next ordinary apply would look
    // like a different `-f`.
    let (rc, out) = apply(&alpha, &state);
    assert_eq!(rc, 0, "re-apply after undo failed:\n{out}");
    assert!(
        warnings(&out).is_empty(),
        "the re-apply after an undo warned — the replay stamped the stack with \
         the staged temp config:\n{out}"
    );

    let (rc, out) = run(&["status", "--state-dir", &state.display().to_string()]);
    assert_eq!(rc, 0, "status failed:\n{out}");
    assert!(
        !out.contains("Stack: ") && !out.contains("Stacks: "),
        "a single-stack dir must print exactly what it printed before #469:\n{out}"
    );
}
