# IMPL-PMAT-194 receipt

Ticket: PMAT-194 (forjar#456), PR #909, branch lane-909.

## What changed

- `build.rs` (already in the PR): both `contracts/` lookups read
  `std::env::var("CARGO_MANIFEST_DIR")` when the script runs, not
  `env!("CARGO_MANIFEST_DIR")` at the time the script was compiled.
- `.github/workflows/ci.yml` (already in the PR): the `examples-validate` job runs
  `cargo test --locked -p forjar-contracts` and the two falsifiers below (#452).
- New this lane: `tests/falsification_tool_jobs_run_where_the_tools_are.rs` — the
  unfiltered-lib-run detector now treats `-p <pkg>` / `--package <pkg>` for any
  package other than the root crate `forjar` as a narrowed run.
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
  (two new narrowed cases: `-p forjar-contracts`, `--package forjar-contracts`)
- `falsification_tool_jobs_run_where_the_tools_are::every_unfiltered_lib_test_job_runs_where_rsync_is`

## Reds that are NOT caused by this PR (also red on plain origin/main)

Measured by reverting the PR's files (`build.rs`, `.github`, the two new tests) to
origin/main in the build clone and running the ten failing test binaries: the same
nine binaries fail identically (the tenth, red 3, passes there). These need a design
decision, a workflow edit (forbidden to this lane), or are host-dependent, so they
are left for the owner:

- `falsification_every_ci_job_runs_on_the_fleet::the_jobs_that_delegate_their_runner_are_exactly_these`:
  ci.yml pins the shared workflow by commit sha, the test expects `@main`.
- `falsification_release_workflow_shape::rule2_release_yml_is_the_only_v_star_tag_push_producer`:
  bench.yml, ci.yml and coverage.yml also trigger on `v*` tags (workflow edit).
- `falsification_untyped_error_sites_ratchet::the_live_tree_holds_both_ceilings`:
  untyped_error_fns = 790 against a ceiling of 778 in `scripts/ratchets/untyped-error-sites.json`.
- `falsification_verb_cli_reader_parity` (two tests): expect sorted listing, observed read_dir order.
- `falsification_budget_dry_run` (three tests), `falsification_comply_count_cannot_run_inside_itself`
  (`ulimit -u` cap of 513 vs this account's thread count),
  `falsification_cron_check_reads_the_owner_crontab` (sudo on the host),
  `falsification_e02_controlmaster_before_the_drift_gate` (fixture ssh spawn count),
  `falsification_io_hash_answers_for_its_machine` (a real connect to 203.0.113.7 times out
  instead of failing as the test names it): the build host's environment; root causes not
  separated from the host in this lane.

## Commands run and results

Host kfocus, through the capped wrapper `fc-pre.sh pr-909`, final tree (head plus this lane's commits):

- `cargo fmt --all -- --check`: rc=0
- `cargo clippy --all-targets -- -D warnings`: rc=0
- `cargo test --lib`: rc=0, 13785 passed, 0 failed, 4 ignored
- `cargo test --test falsification_tool_jobs_run_where_the_tools_are`: rc=0, 4 passed
- `cargo test --test falsification_build_rs_manifest_dir_runtime`: rc=0, 2 passed
- `cargo test --test falsification_ci_runs_contracts_crate_tests`: rc=0, 1 passed
- `cargo test --test falsification_version_matches_manifest`: rc=0, 4 passed
- `cargo test --locked -p forjar-contracts`: rc=0, 1332 passed, 0 failed, 44 ignored

Host framework16, uncapped `cargo test --no-fail-fast` over every integration target, tree before the CHANGELOG
line was added (rust 1.93.0, `cargo`): 384 binaries ok, 9 failed, all nine listed above. The same nine fail
identically on the tree with the PR's files reverted to origin/main; the tool_jobs binary (red 3) passed
there and failed on the PR tree before the detector fix, and passes after it. The workspace-wide run's totals
were not kept: not measured on the final tree. Flips: none seen (the nine reds were identical in both runs).

Not measured: the nine named reds were not fixed or hidden (they are on plain main, no CI job runs them);
the clean-room run on the final tree.

verdict: PARTIAL - fmt, clippy, lib tests and the PR's targets are green on kfocus and the one PR-caused test red is fixed at the detector; nine integration targets that no CI job runs stay red on plain origin/main
IMPL-PMAT-194-RECEIPT-END
