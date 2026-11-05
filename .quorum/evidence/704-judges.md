# Judges — forjar#704: apply --dry-run previews the drift repairs the apply makes

Lanes and their findings: see `704-lanes.md`. Line numbers are at head
302123a9. The measurements ran on a build host (cargo 1.93.0) at that head
with `cargo test --locked --lib -- cli::tests_dry_run` and `cargo test
--locked --test falsification_704_dry_run_shows_drift_repair`, and the tree
was restored and clean afterwards.

## CONFIRMED

1. [one-path] C1 — A drift finding becomes planner input in exactly one function, `mark_drifted`, which both the persisting apply path and the dry-run overlay call, so the preview and the apply cannot drift apart by spelling the loop twice.
   - evidence: `src/cli/apply_drift.rs:345` defines `mark_drifted`; `src/cli/apply_drift.rs:314` is the real apply's call inside `record_machine_drift`, and `src/cli/apply_drift.rs:369` is the call inside `overlay_drift`.
2. [no-write] C2 — The dry run overlays the gate's findings on the locks it loaded into memory and plans from them; it still writes no lock, which #310 requires of a dry run.
   - evidence: `src/cli/apply_dry_run.rs:48` loads the locks and `src/cli/apply_dry_run.rs:49` calls `overlay_drift` on them before `planner::plan`; `src/cli/apply_drift.rs:364` takes the map by mutable reference and has no write. The binary falsifier compares every state-dir byte across two dry runs at `tests/falsification_704_dry_run_shows_drift_repair.rs:335`.
3. [acceptance] C3 — The ticket's acceptance test converges a leaf and its prerequisite, tampers the prerequisite, and asserts the scoped dry run lists it as Update, a second dry run plans the same, and the real scoped apply restores the file.
   - evidence: `src/cli/tests_dry_run_drift_repair.rs:90` is `scoped_dry_run_lists_dependency_drift_that_apply_repairs`; the Update assertion is at `src/cli/tests_dry_run_drift_repair.rs:110`, the repeat-plan assertion at `src/cli/tests_dry_run_drift_repair.rs:119` and the restore at `src/cli/tests_dry_run_drift_repair.rs:124`. `cargo test --locked --lib -- cli::tests_dry_run` at 302123a9: 3 passed.
4. [parity] C4 — The case table runs seven drift shapes and asserts in each that the dry run lists exactly the resources the real apply then converges, read from the apply's own resource_converged events rather than from the plan.
   - evidence: the rows are at `src/cli/tests_dry_run_case_table.rs:197`, the per-row check at `src/cli/tests_dry_run_case_table.rs:240` and the converged set is read by `src/cli/tests_dry_run_case_table.rs:173`; the test at `src/cli/tests_dry_run_case_table.rs:302` runs every row and names each failing one.
5. [discriminates] C5 — The binary falsifier drives the built forjar over the local transport and reads only its stdout and the state dir; with only the `overlay_drift` call removed, rows a, b, d and e go red and row c, a converged config, stays green; restored, all five pass.
   - evidence: row (a) at `tests/falsification_704_dry_run_shows_drift_repair.rs:205`, (b) at `tests/falsification_704_dry_run_shows_drift_repair.rs:253`, (c) at `tests/falsification_704_dry_run_shows_drift_repair.rs:288`, (d) at `tests/falsification_704_dry_run_shows_drift_repair.rs:309`, (e) at `tests/falsification_704_dry_run_shows_drift_repair.rs:326`. At 302123a9: 5 passed. With `src/cli/apply_dry_run.rs:49` deleted the target exits 101 with a, b, d, e failed and c passed; after `git checkout` the tree is clean and the rerun gives 5 passed.
6. [plumbing] C6 — The other hunks only carry the gate's findings to the dry-run output and update the four existing callers of `apply_dry_run_output` with an empty slice; no behaviour outside the dry run changes.
   - evidence: `src/cli/apply.rs:185` passes `observed_drift`; `src/cli/apply_output.rs:31` takes `observed` and forwards it; `src/cli/tests_cov_apply_b.rs:264` is the first of the four call-site updates, each adding `&[]`.

## REFUTED

1. [impl-receipt] R1 — Superseded local round on the unpushed branch, claude-sonnet-5-5, over a source tree identical to this branch's second commit: "the diff has no `docs/audits/impl-PMAT-704-receipt.md`; a release gate needs one at HEAD, so it must land before merge or cut".
   - corrected: forjar has no such per-ticket file; its merge evidence is `.quorum/<branch>.json` with `.quorum/evidence/*.md`, which this receipt is. The lane itself filed the note as non-blocking and PASSed; the test it would have pointed at is `src/cli/tests_dry_run_drift_repair.rs:90`.
2. [roadmap-fields] R2 — Superseded local round on the unpushed branch, claude-sonnet-5-5: "the PMAT-704 roadmap row has no epic or release field, unlike its neighbour, so it may need fixing before the release cut".
   - corrected: refuted as a blocker for this diff. The release-window check reads the roadmap when a version is tagged, not on this branch, and the lane filed it as non-blocking and PASSed. The row's epic and release assignment is release-train bookkeeping; no code line such as `src/cli/apply_dry_run.rs:49` depends on it.
