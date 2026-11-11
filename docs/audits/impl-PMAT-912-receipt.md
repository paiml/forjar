# Implementation receipt — PMAT-912 — drift and plan compare the live version against the declaration (train)

verdict: PARTIAL — PR #961 after merging main is green on the build host (fmt, clippy, lib 13791 passed, five drift targets, all nine `dogfood-guards` targets with git history), but the cause of the earlier CI red (`dogfood-guards` on head 096226d) was not identified: it does not reproduce on the merged tree and the CI log was read only as far as the gate step.

## Identity

- ticket PMAT-912 (forjar#912, train "drift/plan compare live against the declaration"); kind: code; covers #852, #613, #694
- PR #961, branch `cf/912-drift-train`, worked on local branch `lane-961`, merged with `origin/main` @ dbd1ec3e (0 commits behind main after the merge)
- files: `src/cli/drift.rs`, `src/cli/mod_test_decl.rs`, `src/cli/plan_version_pins.rs`, `src/cli/tests_drift_empty_state.rs` (new), `src/resources/package_check.rs`, `src/resources/tests_package_d.rs`, `CHANGELOG.md`, `docs/roadmaps/roadmap.yaml` (row `GH-912` renamed to `PMAT-912`, labels and acceptance criteria added, status inprogress), this receipt

## Contract

1. `drift` with a config and an empty state dir (no lock for any machine) takes the lockless path, so a declared pin is compared against the live binary, offline too (#852). With no config an empty lock set keeps its old meaning.
2. `plan` names a live version that is not the pin (verdict DRIFTED, detail names the DOWNGRADE), discloses the skipped upstream leg when offline, and reads NOT CHECKED for an undeclared machine (#613).
3. `package_check::check_script` appends the declared `completion_check` as its own assertion for apt, cargo, uv and brew (#694).

Falsifiers (all in the PR): `src/cli/tests_drift_empty_state.rs`, `src/cli/plan_version_pins.rs::tests` (three cases), `src/resources/tests_package_d.rs`.

## CI red and root cause

CI on head 096226d: `dogfood-guards` failed (24 s) and `gate` failed (5 s) because it re-reads `dogfood-guards: failure` (read with `gh pr checks 961` and `gh run view 42866833348 --log-failed`, which printed only the gate step). The failing guard test inside `dogfood-guards` was not identified from CI.

Measured on t2yoga, the nine guard targets CI lists (`cargo test --locked --test ...`), on the merged tree cloned WITH history (commit af3237b3): all nine targets ok (4, 4, 19, 9, 4, 4, 9, 13, 14 passed, 0 failed), rc=0. A first run on a tree synced without `.git` failed one case, `the_shipped_floor_is_the_commit_it_says_it_is`, only because the host copy had no history (`b4719737` resolves in the worktree; `--is-shallow-repository` is false); that is a host artifact, not the CI red. The pre-merge CI red therefore most likely came from the 12 commits main was ahead (merged here, 0 behind); this is an inference, not measured.

Open: gate A of the PR (`falsification_gate_a_the_pr_and_its_commits_name_one_ticket`) judges the PR's commits for a `Pmat-Ticket:` trailer; the three PR commits (ab3df95b, 4b8bb126, 5ff78c60) carry `Refs #` and no trailer, and the merge commits cannot be amended on a pushed branch. The new commit carries `Pmat-Ticket: PMAT-912`.

## Evidence (rust on t2yoga, 2026-11-11, wrapper `fc-pre.sh pr-961`)

- `cargo fmt --all -- --check`: rc=0
- `cargo clippy --all-targets -- -D warnings`: rc=0
- `cargo test --lib`: 13791 passed, 0 failed, 4 ignored
- `cargo test --test falsification_drift_declines_on_empty_scope --test falsification_drift_without_a_lock_measures_the_host --test falsification_drift_names_its_lock --test falsification_drift_unmeasured_is_not_drift --test falsification_drift_verdict_reaches_the_exit_code`: all ok (6, 1, 4, 5, 10 passed)
- the nine `dogfood-guards` targets, tree cloned with history (t2yoga, 21:48Z): rc=0, 9 of 9 targets ok
- RED-before-fix of the three code changes: not measured

LAB reds on plain main that no CI job runs (forjar#858: `falsification_untyped_error_sites_ratchet`, `falsification_budget_dry_run`, `falsification_release_workflow_shape`): not run here.

IMPL-PMAT-912-RECEIPT-END
