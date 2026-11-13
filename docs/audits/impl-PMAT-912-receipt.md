# Implementation receipt — PMAT-912 — drift and plan compare the live version against the declaration (train)

verdict: PARTIAL — the code, the drift suites and the dogfood-guards are green on the build host at c22b5be0 (review round 1 fixed), but the branch's three original commits (ab3df95b, 4b8bb126, 5ff78c60) carry no `Pmat-Ticket:` trailer and are pushed, so CB-2113 on them is open, and the cause of the first CI red was never identified.

## Identity

- ticket PMAT-912 (forjar#912, train "drift/plan compare live against the declaration"); kind: code; covers #852, #613, #694
- PR #961, branch `cf/912-drift-train`, worked on local branch `lane-961`, merged with `origin/main` @ dbd1ec3e (0 commits behind main after the merge)
- files: `src/cli/drift.rs`, `src/cli/mod_test_decl.rs`, `src/cli/plan_version_pins.rs`, `src/cli/tests_drift_empty_state.rs` (new), `src/resources/package_check.rs`, `src/resources/tests_package_d.rs`, `CHANGELOG.md`, `docs/roadmaps/roadmap.yaml` (row `GH-912` renamed to `PMAT-912`, labels and acceptance criteria added, status inprogress), this receipt

## Contract

1. `drift` with a config and a state dir that holds NO lock for any machine (empty, or absent) takes the lockless path, so a declared pin is compared against the live binary, offline too (#852). A state dir that holds another stack's locks is not empty: a machine of this config that was never applied there keeps its old meaning, `no state yet`, and is not an error (forjar#488, `falsification_drift_scopes_to_the_config_it_was_given`). With no config an empty lock set keeps its old meaning.
2. `plan` names a live version that is not the pin (verdict DRIFTED, detail names the DOWNGRADE), discloses the skipped upstream leg when offline, and reads NOT CHECKED for an undeclared machine (#613).
3. `package_check::check_script` appends the declared `completion_check` as its own assertion for apt, cargo, uv and brew (#694). It is read by the check path only; apply does not run it, and an unsupported provider still reads diverged. A blank or whitespace-only `completion_check` adds no assertion (as an `if` condition it is a shell syntax error).

Falsifiers (all in the PR): `src/cli/tests_drift_empty_state.rs` (incl. `f852_state_dir_with_another_stacks_lock_is_not_lockless`, `f852_holds_no_lock_tells_empty_from_populated`), `src/resources/tests_package_d.rs::gh694_blank_completion_check_adds_no_assertion`, `src/cli/plan_version_pins.rs::tests` (three cases), `src/resources/tests_package_d.rs`.

## Review round 1 (three non-author lanes FAIL at b23462d4)

1. Lane 1 and 3, `src/cli/drift.rs:233`: the first fix routed EVERY empty scoped lock set to `scan_lockless`, so a declared machine never applied, beside a state dir holding another stack's locks, was inspected 0 of 1 and ended `declined` (exit 1). Measured on t2yoga: `cargo test --test falsification_drift_scopes_to_the_config_it_was_given` is RED on b23462d4 (`a_declared_machine_with_no_state_yet_is_not_an_error`, 5 passed 1 failed) and GREEN on plain main dbd1ec3e (6 passed). So this PR broke it. Five whys: the PR wanted "no lock to walk" to mean lockless (#852); `collect_machine_locks` returns an empty list both for an empty dir and for a populated dir whose locks are all out of scope; the fix keyed on the empty list; the two cases need opposite answers; the receipt ran 5 of the 8 `falsification_drift_*` targets and missed this one. Fix: `drift_state::holds_no_lock` (no `state.lock.yaml` for any machine dir) gates the lockless path. RED commit 4ce10476, fix c22b5be0.
2. Lane 2, `drift.rs`: with m1 applied and m2 never applied, `drift` and `drift -m m2` now agree (both walk locks and ignore m2; before the fix `-m m2` took the lockless path). The older blind spot, an unscoped `drift` never asking the host about an unapplied machine beside an applied one, exists on main and is NOT changed here. Closing it contradicts `a_declared_machine_with_no_state_yet_is_not_an_error`, so it is a design question left open (see below).
3. Lane 2, `package_check.rs`: a blank `completion_check` reached `verdict::assert_block` as an empty `if` condition (`if\n  \nthen`). Fixed: blank is dropped; `gh694_blank_completion_check_adds_no_assertion` was RED on 4ce10476 and is green on c22b5be0. (The lane's cited line 338 and signature `finish(lines, resource)` do not match this tree; the hazard did.) The task resource's own check does not trim either; not changed.
4. Lane 3, trailers: commits ab3df95b, 4b8bb126 and 5ff78c60 carry `Refs #` and no `Pmat-Ticket:` and are already pushed; fixing them needs a history rewrite, which this lane does not do. Open.

## CI red and root cause

CI on head 096226d: `dogfood-guards` failed (24 s) and `gate` failed (5 s) because it re-reads `dogfood-guards: failure` (read with `gh pr checks 961` and `gh run view 42866833348 --log-failed`, which printed only the gate step). The failing guard inside `dogfood-guards` was not identified from CI. On the merged tree, cloned WITH history, all nine guard targets pass (below). Cause of the first red: not identified; the likeliest explanation is the 12 main commits merged since, not measured. A run on a tree synced without `.git` fails `the_shipped_floor_is_the_commit_it_says_it_is`; that is a host artifact.

## Evidence (t2yoga, wrapper `fc-pre.sh pr-961`, tree cloned from a bundle WITH history, 2026-11-11/12)

- RED, commit 4ce10476: `cargo test --lib -- f852_state_dir_with_another_stacks_lock gh694_blank_completion_check`: 0 passed, 2 failed.
- plain main dbd1ec3e: `falsification_drift_scopes_to_the_config_it_was_given`: 6 passed, 0 failed. b23462d4: 5 passed, 1 failed.
- at c22b5be0: `cargo fmt --all -- --check` rc=0; `cargo clippy --all-targets -- -D warnings` rc=0; `cargo test --lib`: 13794 passed, 0 failed, 4 ignored.
- every `falsification_drift_*` target (8 of 8, `ls tests/falsification_drift_*.rs`): declines_on_empty_scope 6, is_not_blind_to_task_guards 6, measures_a_broken_guard 4, names_its_lock 1, scopes_to_the_config_it_was_given 6, unmeasured_is_not_drift 4, verdict_reaches_the_exit_code 5, without_a_lock_measures_the_host 10 passed; 0 failed.
- the nine `dogfood-guards` targets: 9 of 9 ok, 0 failed.
- RED-before-fix of the original three code changes (#852, #613, #694): not measured.
- LAB reds on plain main that no CI job runs (forjar#858: `falsification_untyped_error_sites_ratchet`, `falsification_budget_dry_run`, `falsification_release_workflow_shape`): not run here.
- CB-2113 / pmat commit traceability: not run here.

IMPL-PMAT-912-RECEIPT-END
