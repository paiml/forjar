# Implementation receipt — PMAT-920 — FJ-LOCK-001: per-machine, atomic, owner-checked state lock with bounded wait

verdict: PASS — the state lock is published atomically (C1), held from the pre-validate gates to the post-apply actions on both apply paths and not at all on a dry run (C2), and carries an owner id, a per-machine scope and a bounded FIFO wait (C3-C5); the lock falsifiers and the whole `cli::` suite are green at this head.

## Identity

- ticket PMAT-920 (forjar#920, master; children #921 C1, #922 C2, C3-C5 from #938); kind: code
- PR #930, branch `forjar-cop/fj-lock-001-c1c2`
- this receipt's own delta: `src/cli/commands/apply_lock_args.rs` (new), `src/cli/commands/apply_args.rs`, `src/cli/commands/mod.rs`, `src/cli/apply_lock_flags.rs`, `src/cli/apply.rs`, `src/cli/apply_from_plan.rs`, `src/cli/dispatch_apply_b.rs`, `src/cli/tests_fj_lock_c2.rs`, `src/cli/tests_fj_lock_c3_c5.rs`, test literals of `ApplyArgs`, `docs/roadmaps/roadmap.yaml` (row PMAT-920 added)

## Root cause (five whys, file-health red on this PR)

1. `file-health` was red on #930: `src/cli/apply.rs` 490 -> 501 lines and `src/cli/commands/apply_args.rs` 704 -> 711.
2. C2 spelled the dry-run-aware lock acquire out in `apply.rs`, and again in `apply_from_plan.rs`.
3. C3/C5 added `--lock-timeout` and widened `--force-unlock` inside `ApplyArgs`, a struct that already fills its file.
4. The ratchet refuses any growth of a file over 500 lines, so both additions needed a home of their own.
5. Neither had one: there was no shared lock helper and no args struct for the lock flags.

## Contract and falsifier

Contract: `apply_lock_flags::hold_apply_locks(config, state_dir, machine, force_unlock, dry_run)` returns `Ok(None)` and touches no lock file when `dry_run`; otherwise it holds the lock of every machine in `executor::lock_targets(config, machine)` until the guard drops. `apply` and `apply --plan-file` both take their lock through it. `--force-unlock [ID]` and `--lock-timeout <DUR>` parse exactly as before, from `ApplyLockArgs` flattened into `ApplyArgs`.

Falsifiers: `cli::tests_fj_lock_c2::falsify_fj_lock_c2_dry_run_takes_no_lock` (new) and `falsify_fj_lock_c2_lock_held_from_pre_validate_to_post_actions`; flag parsing by `cli::tests_fj_lock_c3_c5::c3_c5_flags_parse`; flag wiring by `cli::tests_flag_consumer_guard::every_declared_flag_is_consumed_or_refused`.

## file-health

`apply_args.rs` is back to 704 lines (equal to `main`), `apply.rs` is 499, `apply_from_plan.rs` 369 -> 367. No changed `.rs` file is both over 500 lines and longer than on `main`.

## Evidence (kfocus, `nice -n 19`, 2026-11-11)

RED, with `hold_apply_locks` made to return `Ok(None)` always (`cargo test --lib fj_lock_c2`):

    test cli::tests_fj_lock_c2::falsify_fj_lock_c2_dry_run_takes_no_lock ... FAILED
    test cli::tests_fj_lock_c2::falsify_fj_lock_c2_lock_held_from_pre_validate_to_post_actions ... FAILED
    test result: FAILED. 2 passed; 2 failed; 0 ignored; 0 measured; 13781 filtered out; finished in 0.31s

A first GREEN run was itself red on `every_declared_flag_is_consumed_or_refused`: the guard could not see the two flags being read through `args.lock`. `prepare_state_lock` now takes `&ApplyLockArgs`, which the guard follows; the guard is unchanged.

GREEN at this head:

    cargo test --lib fj_lock     test result: ok. 26 passed; 0 failed; 0 ignored
    cargo test --lib cli::       test result: ok. 8336 passed; 0 failed; 1 ignored
    cargo test --test falsification_plan_file_honours_every_flag --test falsification_yes_is_not_an_integrity_override --test falsification_state_integrity   17, 26 and 6 passed, 0 failed

`cargo fmt --all -- --check` rc=0; `cargo clippy --all-targets -- -D warnings` rc=0.

## Not measured here

The RED runs of the C1 and C3-C5 falsifiers were made by the authors of those commits and are not repeated in this receipt; here they are GREEN only. The issue's 8-parallel-apply falsifier is not implemented (lock span and release are tested instead).

- Three-lane review of PR #930 at a60316a (agy, Gemini 4 Carbon): 3/3 PASS; the refuted claims are listed under `quorum.refuted_claims` in the receipt. One lane finding (the FIFO claim in the `lock_wait.rs` module doc was wider than the code) is fixed in be9aa5db. Quorum receipt: `.quorum/forjar-cop-fj-lock-001-c1c2.json`, written by agy and committed unedited.

IMPL-PMAT-920-RECEIPT-END
