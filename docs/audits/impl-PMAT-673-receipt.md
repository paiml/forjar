# IMPL-PMAT-673 receipt

## What changed (PR #914, forjar#673 and forjar#649)

- `.github/workflows/stress.yml`: each matrix leg runs 3 passes of
  `cargo test --lib -- --test-threads=1` instead of 10, the stress step's
  `timeout-minutes` goes 30 -> 45, and the matrix sets `fail-fast: false`.
- `tests/falsification_stress_workflow_fits_its_timeout.rs` (new): reads the
  workflow and asserts passes x 9 min + 10 min build <= the step timeout, and
  that `fail-fast: false` is present.
- `tests/falsification_sec005_needs_a_token_boundary.rs` (new): `disk-watch`,
  `ask-lane`, `desk-lamp` in quoted assignments (`X="ci-<word>.timer"`,
  `U="https://x/<word>"`) are not flagged SEC005; a real key-shaped literal
  still is. Red on bashrs 6.68.0, green on 7.4.2 (measured on t2yoga).
- `docs/roadmaps/roadmap.yaml`: PMAT-673 row.
- `CHANGELOG.md`: Unreleased entry for the stress lane.
- This receipt.

## Five whys: why was PR #914 red

1. The `gate` job failed. Because it requires `dogfood-guards` and that job
   reported `failure`.
2. `dogfood-guards` failed. Because its `cargo test --locked --test ...` line
   named `falsification_coverage_gate_mutation_scope`, and cargo exited 101 with
   "no test target named" that file.
3. The target did not exist on the branch. Because main deleted that test in
   #973 (gate F measures the coverage floor alone) and the guard job in
   ci.yml was updated to stop naming it in #982; the branch had been cut before
   both and carried the old ci.yml.
4. The branch was not rebased. Because the CI run (00:09Z) was taken on the
   pre-#982 head, and no job re-ran after main moved.
5. Nothing on the branch re-checked itself against main. Because the guard list
   lives in ci.yml, owned by main, not by this PR.

Fix at the owning module: none is needed in this PR's files. The owner of the
guard list is ci.yml on main (#982), which the merge of origin/main already
brought in (merge commit 7087efe9, clean). The nine guard targets now named in
ci.yml all pass on the merged tree. Nothing was skipped or ignored.

## Contract and falsifier tests

- `falsification_stress_workflow_fits_its_timeout`:
  `passes_per_leg_fit_the_timeout`, `a_red_leg_does_not_cancel_its_siblings`
  (both pass; they fail on the old stress.yml: 10 x 9 + 10 > 30, no fail-fast).
- `falsification_sec005_needs_a_token_boundary`:
  `words_ending_in_sk_are_not_api_keys`,
  `a_real_key_shaped_literal_is_still_flagged` (both pass).

## Failures that are not caused by this PR

The first full `cargo test --no-fail-fast` on kfocus (taken while the host was
loaded by other builds) ended with 9 failing targets. The final run, on the
committed tree, ends with 2. None is caused by the PR: it changes no file under
`src/`, and re-running the eight non-doc targets with `stress.yml` put back to
its origin/main content gave the identical failing tests.

Still failing in the final run, both red on origin/main by content:

- `falsification_release_workflow_shape`
  (`rule2_release_yml_is_the_only_v_star_tag_push_producer`): origin/main's
  bench.yml, ci.yml and coverage.yml all carry `tags: ['v*']`.
- `falsification_untyped_error_sites_ratchet` (`the_live_tree_holds_both_ceilings`):
  790 untyped error fns against a ceiling of 778, from src/ on main.

Failed in the first run only, and passed in the final run with no change to the
tree, so they are load or host dependent (the doctests died at link time with an
LLVM linker crash): `falsification_every_ci_job_runs_on_the_fleet`,
`falsification_budget_dry_run`, `falsification_comply_count_cannot_run_inside_itself`,
`falsification_cron_check_reads_the_owner_crontab`,
`falsification_e02_controlmaster_before_the_drift_gate`,
`falsification_io_hash_answers_for_its_machine`, and `--doc`.

These need their own tickets and owners; they are outside the scope of
PMAT-673, and fixing them means editing src/ or main's workflows, a design
decision.

## Commands run (all on a scratch checkout)

- `cargo fmt --all -- --check`: exit 0
- `cargo clippy --all-targets -- -D warnings`: exit 0
- `cargo test --locked` over the nine dogfood-guards targets from ci.yml: exit 0
- `cargo test --no-fail-fast` (full, first run): exit 101, 9 targets failed
- `cargo test --no-fail-fast` over 8 targets with stress.yml at origin/main
  content: exit 101, the same failing tests as the first run
- `cargo test --no-fail-fast` (full, final run on the committed tree): 2 targets
  failed, both listed above

verdict: partial - the PR's own red (dogfood-guards) is green and fmt and clippy are clean; two suite targets still fail and also fail without the PR's change
IMPL-PMAT-673-RECEIPT-END
