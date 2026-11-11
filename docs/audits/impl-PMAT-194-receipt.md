# IMPL-PMAT-194 receipt

Ticket: PMAT-194 (forjar#456), PR #909, branch lane-909.

## What changed

- `build.rs` (already in the PR): both `contracts/` lookups read
  `std::env::var("CARGO_MANIFEST_DIR")` when the script runs, not
  `env!("CARGO_MANIFEST_DIR")` at the time the script was compiled.
- `.github/workflows/ci.yml` (already in the PR): the `examples-validate` job runs
  `cargo test --locked -p forjar-contracts` and the two falsifiers below (#452).
- New this lane: `tests/falsification_tool_jobs_run_where_the_tools_are.rs` — the
  unfiltered-lib-run detector treats a run as narrowed when it names packages
  (`-p <pkg>`, `--package <pkg>`, `--package=<pkg>`) and none of them is the root
  crate `forjar` and `--workspace` is absent; cargo runs the union of every `-p`.
- Review round 1 (three non-author lanes, head 6f655ea): `build.rs` now emits
  `cargo::rerun-if-env-changed=CARGO_MANIFEST_DIR` so a second checkout sharing a
  target dir re-runs the script; the multi-`-p` and `--package=` forms in the
  detector; the build.rs source check also catches `option_env!`; the ci.yml
  falsifier accepts `--package` spellings. The ci.yml step itself is unchanged;
  whether the added steps need a gate sign-off is not decided by this receipt.
- `CHANGELOG.md`: Unreleased entry for the build.rs fix and the contracts CI step.
- origin/main was already merged into the branch (clean) before this lane started.

## Reds on PR #909 and their root causes

### Red 1: `dogfood-guards` — `no test target named falsification_coverage_gate_mutation_scope`

Five whys:
1. The job exited 101 on the first `cargo test --test` step.
2. cargo found no such test target in the tree it checked out.
3. The PR branch was stale: its merge result still carried the workflow step that names the target.
4. origin/main removed the target and its workflow step in #973 ("mutation runs on
   schedule only; gate F measures the coverage floor alone").
5. A branch that does not merge the default branch keeps a step whose subject was
   deleted. Root cause: staleness, owned by the branch, not by a module.
Fix: merging origin/main (already done before this lane). The tree now has no
reference to the deleted target (`git grep` over `.github`, `scripts`, `Makefile`: none).

### Red 2: `gate` — aggregate of red 1 (`dogfood-guards: failure`)

The gate job only reports the failing needed job. It clears when red 1 clears and
when red 3 below is fixed.

### Red 3 (found by the full local suite, not shown by CI yet): `falsification_tool_jobs_run_where_the_tools_are::every_unfiltered_lib_test_job_runs_where_rsync_is`

Five whys:
1. The test failed: ci.yml job `examples-validate` runs on `[self-hosted, clean-room, ARM64]`,
   which the test says also matches rsync-less containers.
2. It believed that job runs unfiltered lib tests, because of the new step
   `cargo test --locked -p forjar-contracts` that this PR adds.
3. `cargo_args_are_unfiltered` treated `-p <pkg>` as a value-taking flag only and
   skipped the value, so any `-p` run counted as unfiltered.
4. The test's purpose is to pin jobs that reach `nas_archive`'s rsync-requiring
   safety test, which lives in the root crate (`src/resources/nas_archive/tests.rs`);
   a different package cannot reach it.
5. The detector never modelled package scope. Root cause is in the detector in
   `tests/falsification_tool_jobs_run_where_the_tools_are.rs`, fixed there.
Fix: `-p`/`--package` with any value other than `forjar` narrows the run
(`ROOT_PACKAGE`). `-p forjar --lib` stays unfiltered (existing case kept).
It was RED before the fix on the PR tree (this test failed in the pre-build run;
it passed on the origin/main tree, which lacks the new step) and is GREEN after.

## Contract and falsifier test names

- Contract: forjar#456 — build.rs resolves `contracts/` at run time; forjar#452 —
  CI runs the vendored contracts crate suite; forjar#598 — unfiltered lib tests run on bare metal.
- `falsification_build_rs_manifest_dir_runtime::build_rs_has_no_compile_time_manifest_dir`
- `falsification_build_rs_manifest_dir_runtime::build_rs_reads_manifest_dir_at_run_time`
- `falsification_ci_runs_contracts_crate_tests::ci_runs_the_forjar_contracts_unit_tests`
- `falsification_tool_jobs_run_where_the_tools_are::the_detector_does_not_pin_what_cannot_reach_the_test`
  (narrowed: `-p forjar-contracts`, `--package forjar-contracts`, `--package=forjar-contracts`, two non-root
  packages; unfiltered: `-p forjar-contracts -p forjar` in both orders, `--package` and `--package=` pairs,
  `-p forjar-contracts --workspace`)
- `falsification_build_rs_manifest_dir_runtime::build_rs_reruns_when_the_manifest_dir_changes`
  (RED on kfocus before the build.rs fix, GREEN after; the multi-`-p` case was RED the same way)
- `falsification_tool_jobs_run_where_the_tools_are::every_unfiltered_lib_test_job_runs_where_rsync_is`

## Commands run and results

Host kfocus, through the capped wrapper `fc-pre.sh pr-909`, tree at the commit that carries this receipt's code:

- RED (before the fixes, commit with the new cases only): `cargo test --test falsification_tool_jobs_run_where_the_tools_are --test falsification_build_rs_manifest_dir_runtime --test falsification_ci_runs_contracts_crate_tests --no-fail-fast`: rc=101;
  `build_rs_reruns_when_the_manifest_dir_changes` FAILED, `the_detector_does_not_pin_what_cannot_reach_the_test` FAILED ("missed an unfiltered run: cargo test --locked -p forjar-contracts -p forjar")
- `cargo fmt --all -- --check`: rc=0
- `cargo clippy --all-targets -- -D warnings`: rc=0
- `cargo test --lib`: rc=0, 13785 passed, 0 failed, 4 ignored
- `cargo test --test falsification_tool_jobs_run_where_the_tools_are`: rc=0, 4 passed
- `cargo test --test falsification_build_rs_manifest_dir_runtime`: rc=0, 3 passed
- `cargo test --test falsification_ci_runs_contracts_crate_tests`: rc=0, 1 passed
- `cargo test --test falsification_version_matches_manifest`: rc=0, 4 passed
- `cargo test --test falsification_contract_citations_resolve`: rc=0, 7 passed
- `cargo test --test examples_validate`: rc=0, 2 passed
- `cargo test --locked -p forjar-contracts`: rc=0, 1332 passed, 0 failed, 44 ignored

Earlier measurement, host framework16, tree before the review-round changes: uncapped `cargo test --no-fail-fast`
over every integration target: 384 binaries ok, 9 failed, the nine listed below; the same nine fail identically
with the PR's files reverted to origin/main. No flips seen (identical in both runs).

## Not covered

- Nine integration targets are red on plain origin/main and no CI job runs them (forjar#858). They were neither
  fixed nor hidden: `falsification_budget_dry_run` (3 tests), `falsification_comply_count_cannot_run_inside_itself`,
  `falsification_cron_check_reads_the_owner_crontab`, `falsification_e02_controlmaster_before_the_drift_gate`,
  `falsification_every_ci_job_runs_on_the_fleet`, `falsification_io_hash_answers_for_its_machine`,
  `falsification_release_workflow_shape`, `falsification_untyped_error_sites_ratchet` (790 against ceiling 778),
  `falsification_verb_cli_reader_parity` (2 tests).
- The all-targets run was not repeated on the final tree (not measured); the clean-room run on the final tree.
- Whether the `.github/workflows/ci.yml` steps added by this PR need gate sign-off, and the runner-pool
  placement of `examples-validate`: not decided here.

verdict: PASS - every in-scope claim is measured on kfocus (fmt, clippy, lib, the PR's and the detector's targets, the contracts crate); the nine reds no CI job runs are listed under Not covered
IMPL-PMAT-194-RECEIPT-END
