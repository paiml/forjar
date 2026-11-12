# Implementation receipt — PMAT-659 — PR #902 brought current: check counts an unreachable host as unmeasured

verdict: PARTIAL — fmt and clippy are green and every test the PR adds or touches passes, but 13 other tests in 10 test binaries fail on this tree and fail identically on plain origin/main, so the full suite cannot go green inside this PR.

## Identity

- tickets PMAT-659 (forjar#659) and PMAT-661 (forjar#661); kind: code; PR #902, branch `lane-902` (remote `forjar-cop/packB`)
- merge base with `main` @ dbd1ec3e (origin/main already merged into the branch, no conflicts)
- files this lane touched: `CHANGELOG.md` (two Unreleased entries), `docs/audits/impl-PMAT-659-receipt.md`. No source file was edited: the PR's own code needed no change.

## What changed in the PR (unchanged by this lane)

- `src/cli/check.rs`, `src/cli/check_json.rs` (new): an ssh exit 255 or a transport error is status `unmeasured`, not `fail`; the report counts it, `all_passed` is false, and the run exits 4 ahead of a drift verdict (`CHECK_UNMEASURED_MARKER` in `src/core/error.rs`).
- `src/cli/lock_core.rs`, `src/cli/lock_prune_scope.rs` (new): `lock-prune -f` touches only machines the config declares and refuses a machine another stack's stamp lists, before writing anything.

## Root cause of the red CI (five whys)

1. PR #902's `dogfood-guards` job (run 42864752878) exited 101 and `gate` then failed on it.
2. Its `cargo test --locked --test ...` line named `falsification_coverage_gate_mutation_scope`, and cargo said `no test target named ... in default-run packages`.
3. That test file had been deleted on `main` (#973) after this branch was cut.
4. The branch's `ci.yml` still carried the old list, because the branch was stale against `main` and the workflow edit that dropped the name (#982) was not in it.
5. Nobody merged `main` into the PR branch after #973/#982 landed, so the PR ran a guard list that `main` had already corrected.

Fix at the owning module: the workflow file is `main`'s, and the merge of origin/main already in this branch brings it. Nothing in `.github/` is edited here. Evidence: the `dogfood-guards` command line from the branch's own `ci.yml` was run on t2fw13 against this tree (exit 0, 9 test binaries `ok`, no missing target).

## The other reds are not this PR's

The full suite on this tree has 13 failing tests in 10 binaries. The same 10 binaries were run on an export of plain `origin/main` (dbd1ec3e, no PR content) in the same directory with the same warm target dir: the same 13 tests fail. The PR and main sets are equal.

| test binary | failing tests | what the output shows |
| --- | --- | --- |
| `falsification_untyped_error_sites_ratchet` | `the_live_tree_holds_both_ceilings` | `untyped_error_fns = 790, ceiling 778`; an independent count of `src/` gives 790 on `main` and 790 on this tree, so the PR adds no site (its new signatures span lines) |
| `falsification_every_ci_job_runs_on_the_fleet` | `the_jobs_that_delegate_their_runner_are_exactly_these` | the workflows pin `paiml/.github/...@fbee3788...`, the test expects `@main` |
| `falsification_release_workflow_shape` | `rule2_release_yml_is_the_only_v_star_tag_push_producer` | `bench.yml`, `ci.yml`, `coverage.yml` also list `v*` tag pushes |
| `falsification_budget_dry_run` | 3 tests | reaper preview reported 2 reclaimed targets, not 0 (the reaper saw `/tmp` at 95% used; host-dependent, not confirmed) |
| `falsification_comply_count_cannot_run_inside_itself` | `the_process_cap_is_applied_and_fails_closed` | `ulimit -u` cap of 513 not enforced on this host |
| `falsification_cron_check_reads_the_owner_crontab` | `a_host_that_cannot_read_the_crontab_skips_instead_of_claiming_missing` | the script ran as a non-root user on that host (inferred, not confirmed) |
| `falsification_e02_controlmaster_before_the_drift_gate` | `ssh_master_opens_before_any_gate_query` | one remote query seen, expected one per locked resource |
| `falsification_io_hash_answers_for_its_machine` | `a_remote_cache_true_task_is_never_skipped_from_the_controllers_hash` | `203.0.113.7` connect timed out, message shape differs |
| `falsification_replace_running_binary` | `the_probe_can_fail` | `cp` over a running binary succeeded on this host |
| `falsification_verb_cli_reader_parity` | 2 tests | the filesystem listed the directory in sorted order |

Per the brief, a failure also red on plain `origin/main` is not fixed here. Two of them are owned elsewhere and are not changeable inside this lane's scope: the ratchet ceiling (`scripts/ratchets/untyped-error-sites.json` may only shrink, and 790 > 778 already holds on `main`) and the two workflow-shape tests (they read `.github/workflows/*`, which is forbidden here). The rest depend on the t2fw13 host (disk, ulimit, uid, kernel, filesystem order, network).

## Contract and falsifier

Contract: `forjar check` / `apply --check` exit 4, status `unmeasured`, when a target never answered, and exit 1 only for drift measured on a reachable host. `lock-prune -f <config>` never deletes an entry from a machine the config does not declare, and refuses one another stack also owns.

Falsifiers (committed with the PR, all pass here):

- `tests/falsification_659_check_unreachable_is_unmeasured.rs`: `unreachable_host_is_unmeasured_exit_4`, `unreachable_host_json_status_is_unmeasured`, `local_drift_is_still_a_fail_exit_1`, `converged_local_host_exits_0`
- `tests/falsification_661_lock_prune_scoped_to_config.rs`: `prune_from_one_stack_leaves_the_other_stacks_locks_alone`, `control_single_stack_still_prunes_a_removed_resource`

## Commands run (host t2fw13, `~/src/fc-pre/pr-902`, never on the build-barred machine)

- `cargo fmt --all -- --check` exit 0
- `cargo clippy --all-targets -- -D warnings` exit 0
- `cargo test --no-fail-fast` exit 101: 393 test binaries, 383 `ok`, 10 FAILED, 13 failing tests (table above)
- the `dogfood-guards` command from `.github/workflows/ci.yml` exit 0
- the same 10 failing binaries on an export of origin/main, exit 101, the same 13 tests FAILED
- `cargo test --workspace --no-fail-fast` exit 101: the same 13 tests

## Re-run on the build host gx10 (through the pre-build wrapper, targeted)

- `cargo fmt --all -- --check` RC 0; `cargo clippy --all-targets -- -D warnings` RC 0
- `cargo test --test falsification_659_check_unreachable_is_unmeasured --test falsification_661_lock_prune_scoped_to_config`: 4 + 2 tests, all ok, RC 0
- the `dogfood-guards` command from `ci.yml`: 8 binaries ok, 1 FAILED, RC 101: `the_shipped_floor_is_the_commit_it_says_it_is`. It needs the git history of the checkout and the gx10 tree is synced without `.git`; the same command passed with a `.git` present (earlier run, exit 0). Different inputs, so not a flip.
- `cargo test --lib cli::check` matched 0 tests and `cargo test --lib cli::lock` 4 ok (RC 0); the PR's tests are the two integration binaries above.
- Flips (same inputs, two verdicts): none seen. The 13 failing tests were identical across four runs (two full-suite runs, the workspace run, the origin/main subset run).

IMPL-PMAT-659-RECEIPT-END
