# Implementation receipt — PMAT-912 — drift and plan compare the live version against the declaration (train)

verdict: PARTIAL — the code, the drift suites and the dogfood-guards are green on the build host at c22b5be0 (review round 1 fixed), but the branch's three original commits (ab3df95b, 4b8bb126, 5ff78c60) carry no `Pmat-Ticket:` trailer and are pushed, so CB-2113 on them is open, and the cause of the first CI red was never identified.

## Identity

- ticket PMAT-912 (forjar#912, train "drift/plan compare live against the declaration"); kind: code; covers #852, #613, #694
- PR #961, branch `cf/912-drift-train`, worked on local branch `lane-961`, merged with `origin/main` @ dbd1ec3e (0 commits behind main after the merge)
- files: `src/cli/drift.rs`, `src/cli/drift_state.rs`, `src/cli/mod_test_decl.rs`, `src/cli/plan_version_pins.rs`, `src/cli/tests_drift_empty_state.rs` (new), `src/core/executor/output_verify.rs` (test only), `src/resources/package_check.rs`, `src/resources/tests_package_d.rs`, `CHANGELOG.md`, `docs/roadmaps/roadmap.yaml` (row `GH-912` renamed to `PMAT-912`, labels and acceptance criteria added, status inprogress), this receipt

## Contract

1. `drift` with a config and a state dir that holds NO lock for any machine (empty) takes the lockless path, so a declared pin is compared against the live binary, offline too (#852). A state dir that holds another stack's locks is not empty: a machine of this config that was never applied there is not an error (exit 0) (forjar#488, `falsification_drift_scopes_to_the_config_it_was_given`). With no config an empty lock set keeps its old meaning.
2. The plan version-pin pass, `plan_version_pins::measure` (tested at function level; the `forjar plan` CLI wiring is NOT pinned by a test), reports a live version that is not the pin as DRIFTED (detail names the DOWNGRADE), discloses the skipped upstream leg when offline, and reads NOT CHECKED for an undeclared machine (#613).
3. #694 is already on main (5b1adb11); this PR adds only the blank guard and tests. `package_check::check_script` includes both its presence assertions and `completion_check` (no order is pinned) for apt, cargo, uv and brew (pinned by `gh694_completion_check_reaches_the_check_script`). `output_verify::verify_against_host` asks the same check script (test `a_failing_package_completion_check_fails_the_post_apply_verification`). A blank or whitespace-only `completion_check` adds no assertion (as an `if` condition it is a shell syntax error).

Falsifiers (all in the PR): `src/cli/tests_drift_empty_state.rs` (incl. `f852_state_dir_with_another_stacks_lock_is_not_lockless`, `f852_holds_no_lock_tells_empty_from_populated`), `src/resources/tests_package_d.rs::gh694_blank_completion_check_adds_no_assertion`, `src/cli/plan_version_pins.rs::tests` (three cases), `src/resources/tests_package_d.rs`.

## Review round 1 (three non-author lanes FAIL at b23462d4)

Records of what was measured or reported then; they claim no behaviour of the current tree beyond the tests named.

1. Lane 1 and 3, `src/cli/drift.rs`: at b23462d4 `cargo test --test falsification_drift_scopes_to_the_config_it_was_given` was RED (`a_declared_machine_with_no_state_yet_is_not_an_error`, 5 passed 1 failed) and GREEN on plain main dbd1ec3e (6 passed), measured on t2yoga. Fix: `drift_state::holds_no_lock` gates the lockless path; RED commit 4ce10476, fix c22b5be0. The receipt then ran 5 of the 8 `falsification_drift_*` targets and missed this one.
2. Lane 2, `package_check.rs`: a blank `completion_check` was reported to reach the script as an empty `if` condition. Fix: blank is dropped; `gh694_blank_completion_check_adds_no_assertion` was RED on 4ce10476 and is green on c22b5be0.
3. Lane 3, trailers: commits ab3df95b, 4b8bb126 and 5ff78c60 carry `Refs #` and no `Pmat-Ticket:` and are already pushed; fixing them needs a history rewrite, which this lane does not do. Open.
4. Lane 2 also raised a design question about an unscoped `drift` and a machine that was never applied. It is not changed here and no test in this PR pins either behaviour; it is left open.

## CI red

CI on head 096226d: `dogfood-guards` failed and `gate` failed. The failing guard inside `dogfood-guards` was not identified from CI, and the cause of the first red is not identified. On a merged tree cloned WITH history, all nine guard targets passed (below).

## Evidence (t2yoga, wrapper `fc-pre.sh pr-961`, tree cloned from a bundle WITH history, 2026-11-11/12)

- RED, commit 4ce10476: `cargo test --lib -- f852_state_dir_with_another_stacks_lock gh694_blank_completion_check`: 0 passed, 2 failed.
- plain main dbd1ec3e: `falsification_drift_scopes_to_the_config_it_was_given`: 6 passed, 0 failed. b23462d4: 5 passed, 1 failed.
- at c22b5be0 (round-1 fix; later heads add merges of main, 05414f90 and the follow-up below): `cargo fmt --all -- --check` rc=0; `cargo clippy --all-targets -- -D warnings` rc=0; `cargo test --lib`: 13794 passed, 0 failed, 4 ignored.
- every `falsification_drift_*` target (8 of 8, `ls tests/falsification_drift_*.rs`): declines_on_empty_scope 6, is_not_blind_to_task_guards 6, measures_a_broken_guard 4, names_its_lock 1, scopes_to_the_config_it_was_given 6, unmeasured_is_not_drift 4, verdict_reaches_the_exit_code 5, without_a_lock_measures_the_host 10 passed; 0 failed.
- the nine `dogfood-guards` targets: 9 of 9 ok, 0 failed.
- RED-before-fix of the original three code changes (#852, #613, #694): not measured.
- LAB reds on plain main that no CI job runs (forjar#858: `falsification_untyped_error_sites_ratchet`, `falsification_budget_dry_run`, `falsification_release_workflow_shape`): not run here.
- CB-2113 / pmat commit traceability: not run here.

IMPL-PMAT-912-RECEIPT-END
