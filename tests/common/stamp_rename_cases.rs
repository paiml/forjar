//! PMAT-161 (S2): a RENAME of a stack, what `undo` does across one, and the
//! four S1 findings the round-1 review quorum returned against the shipped
//! per-name stamp (PMAT-174, PMAT-175, PMAT-176, PMAT-177).
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

// ── the round-1 review findings (PMAT-174..177) ──────────────────────

/// (i) PMAT-174. `apply --rollback-on-failure` in a shared state dir is
/// refused BEFORE the apply, not after it.
///
/// THE DEFECT. The flag promises a restore, and the restore is the whole-dir
/// one this contract refuses in a dir holding more than one stack. The refusal
/// was reached only from `apply_failure_path`, after the executor had
/// converged the host and `apply_post_actions` had rewritten the state dir —
/// and `maybe_rollback_generation` swallowed it into `warning: generation
/// rollback failed`, so the run exited on whatever the resources did. The
/// operator asked for "apply, and rewind if anything fails", was told nothing
/// until the failure, and then got neither the rewind nor an error naming why.
///
/// The gate now runs in the preflight, above the drift gate and the SSH
/// sockets, so the promise is checked before the first byte.
#[test]
fn rollback_on_failure_is_refused_before_the_apply_writes_anything() {
    let (f, _) = fleet();
    // A change alpha would converge, so "refused" and "did nothing" are
    // different claims.
    write_stack(&f.root, "alpha", "alpha", "mini", "two");
    let before = fingerprint(&f.state);

    let (rc, out) = apply_with(&f.alpha, &f.state, &["--rollback-on-failure"]);
    assert_ne!(
        rc, 0,
        "apply --rollback-on-failure ran in a dir holding three stacks, promising \
         a restore that would revert the other two:\n{out}"
    );
    assert!(
        out.contains("PMAT-162"),
        "the refusal must name the ticket that makes restore stack-scoped; got:\n{out}"
    );
    assert!(
        out.contains("bravo") && out.contains("charlie"),
        "the refusal must list the stacks the promised rollback would revert; got:\n{out}"
    );
    assert_eq!(
        fingerprint(&f.state),
        before,
        "the refusal arrived after the apply had already written the state dir:\n{out}"
    );
    assert_eq!(
        marker(&f.root, "alpha"),
        "one\n",
        "the refusal arrived after the host had already been converged:\n{out}"
    );

    // ANTI-VACUITY: the flag is not refused in the dir it can honour.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let state = root.join("state");
    let alpha = write_stack(&root, "alpha", "alpha", "mini", "one");
    let (rc, out) = apply_with(&alpha, &state, &["--rollback-on-failure"]);
    assert_eq!(
        rc, 0,
        "a single-stack --rollback-on-failure was refused:\n{out}"
    );
    assert_eq!(marker(&root, "alpha"), "one\n", "and it must still apply");
}

/// (j) PMAT-175. A config that merely SHARES A BASENAME with a stamped one is
/// not a rename of it.
///
/// THE DEFECT, an executed reproducer from the review lane. A stamp records its
/// `-f` relative to the state dir (`../forjar.yaml`), and
/// `stamp::same_config_file` compared it by dropping the `..` components and
/// asking whether the current path ENDS WITH the rest. Every
/// `machines/<m>/forjar.yaml` in the paiml/infra layout ends with
/// `forjar.yaml`, so the first machine manifest applied into the root stack's
/// state dir was read as that stack RENAMED: `retire_renamed` deleted the root
/// stack's stamp and took its machines and output keys with it.
#[test]
fn a_config_sharing_a_basename_is_not_a_rename_of_the_root_stack() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let state = root.join("state");
    // The root stack: <root>/forjar.yaml, stamped as `../forjar.yaml`.
    let top = write_stack(&root, ".", "root-stack", "box", "one");
    assert_eq!(
        apply(&top, &state).0,
        0,
        "the root stack's apply must succeed"
    );

    // A DIFFERENT stack, in the layout that produced the reproducer:
    // <root>/machines/mini/forjar.yaml, its own name, its own machine.
    let mini = write_stack(&root, "machines/mini", "mini", "mini-box", "one");
    let (rc, out) = apply(&mini, &state);
    assert_eq!(rc, 0, "the machine manifest's apply must succeed:\n{out}");
    assert!(
        !out.contains("renamed to"),
        "a different file in a different directory is a second stack, not a \
         rename of the first; got:\n{out}"
    );

    let lock = lock_of(&state);
    let mut names: Vec<&str> = lock.stacks.keys().map(String::as_str).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        ["mini", "root-stack"],
        "the root stack's stamp was RETIRED by a config that only shares its \
         basename — the dir now claims a stack it has never been applied by"
    );
    assert_eq!(
        lock.stamp_for("root-stack").unwrap().machines,
        ["box"],
        "and its machines went with it"
    );
}

/// (k) PMAT-176. A SCOPED apply (`-m`) narrows what runs, not what the stack
/// OWNS.
///
/// THE DEFECT. `rename::next_stamp` set `machines` from the apply's own
/// results, and a scoped apply produces results for the filtered machine only.
/// So `apply -m mini` rewrote `stacks[alpha].machines` as `[mini]` and RELEASED
/// `lambda-labs` — the ownership record `stack_conflict` reads — although the
/// config still declares it. The next sibling stack naming that machine was
/// then silent about overwriting alpha's generations and per-machine lock.
///
/// A machine is released when the CONFIG stops declaring it, which is an edit
/// the operator made, not when one invocation happens not to converge it.
#[test]
fn a_scoped_apply_keeps_the_machines_the_config_still_declares() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let state = root.join("state");
    let alpha = write_two_machine_stack(&root, "alpha", "alpha", ("mini", "lambda-labs"), "one");
    assert_eq!(apply(&alpha, &state).0, 0, "the full apply must succeed");
    assert_eq!(
        lock_of(&state).stamp_for("alpha").unwrap().machines,
        ["mini", "lambda-labs"],
        "fixture: the unscoped apply owns both machines"
    );

    // The scoped apply: a change on one machine only.
    write_two_machine_stack(&root, "alpha", "alpha", ("mini", "lambda-labs"), "two");
    let (rc, out) = apply_with(&alpha, &state, &["-m", "mini"]);
    assert_eq!(rc, 0, "the scoped apply failed:\n{out}");
    assert_eq!(
        lock_of(&state).stamp_for("alpha").unwrap().machines,
        ["mini", "lambda-labs"],
        "`-m mini` released 'lambda-labs' to any stack that asks for it, although \
         the config still declares it:\n{out}"
    );

    // THE CONSEQUENCE, asserted rather than assumed: a sibling stack claiming
    // the machine alpha did not converge this time is still a conflict.
    let beta = write_stack(&root, "beta", "beta", "lambda-labs", "one");
    let (rc, out) = apply(&beta, &state);
    assert_eq!(rc, 0, "apply warns here, it does not refuse:\n{out}");
    assert!(
        out.contains("which stack 'alpha' owns"),
        "a machine alpha still declares must still be reported as alpha's; got:\n{out}"
    );
}

/// (l) PMAT-177. Snapshot GC does not trim a state dir it cannot attribute.
///
/// THE DEFECT. `gc_old_snapshots` keeps the newest `policy.snapshot_generations`
/// snapshots IN THE DIR and deletes the rest. Snapshots carry no owner, so in a
/// shared dir the count is every stack's, and one stack's apply deleted the
/// pre-apply snapshots another stack made — the copies an operator would use to
/// recover from exactly this class of mistake. Retention is per state dir while
/// the promise (`snapshot_generations`) is per config.
///
/// Ownership is PMAT-162; until it lands, an apply into a dir holding more than
/// one stack skips the GC and says so.
#[test]
fn snapshot_gc_is_skipped_while_the_state_dir_holds_several_stacks() {
    let (f, _) = fleet();
    // More snapshots than any stack's keep count (10), planted rather than
    // applied: the name carries a second-resolution timestamp.
    let planted = plant_snapshots(&f.state, 12);
    write_stack(&f.root, "alpha", "alpha", "mini", "two");

    let (rc, out) = apply(&f.alpha, &f.state);
    assert_eq!(rc, 0, "the apply itself must still succeed:\n{out}");
    assert!(
        out.contains("snapshot gc skipped") && out.contains("PMAT-162"),
        "the skip must be stated, and name the ticket that makes retention \
         per-stack; got:\n{out}"
    );
    let kept = snapshot_names(&f.state);
    for name in &planted {
        assert!(
            kept.contains(name),
            "alpha's apply deleted snapshot {name} out of a dir bravo and charlie \
             also keep snapshots in; kept: {kept:?}"
        );
    }

    // ANTI-VACUITY: a dir with one stack still prunes, exactly as before.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let state = root.join("state");
    let solo = write_stack(&root, "solo", "solo", "mini", "one");
    assert_eq!(apply(&solo, &state).0, 0, "the first apply must succeed");
    let planted = plant_snapshots(&state, 12);
    write_stack(&root, "solo", "solo", "mini", "two");
    let (rc, out) = apply(&solo, &state);
    assert_eq!(rc, 0, "the single-stack apply failed:\n{out}");
    assert!(
        !out.contains("snapshot gc skipped"),
        "a single-stack dir must not skip its own gc:\n{out}"
    );
    let kept = snapshot_names(&state);
    assert!(
        planted.iter().any(|name| !kept.contains(name)),
        "the single-stack gc pruned nothing — the skip is over-broad; kept: {kept:?}"
    );
}
