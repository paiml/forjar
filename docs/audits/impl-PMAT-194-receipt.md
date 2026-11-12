# IMPL-PMAT-194 receipt

Ticket: PMAT-194 (forjar#456), PR #909, branch lane-909.

## What changed

- `build.rs`: both `contracts/` lookups read `std::env::var("CARGO_MANIFEST_DIR")` when the script
  runs, not `env!("CARGO_MANIFEST_DIR")` at the time the script was compiled. It also emits
  `cargo::rerun-if-env-changed=CARGO_MANIFEST_DIR`, so a second checkout sharing a target dir re-runs it.
- `tests/falsification_build_rs_manifest_dir_runtime.rs` (new): pins the run-time read, the absence of
  `env!`/`option_env!` of the variable, and the rerun directive.
- `tests/falsification_tool_jobs_run_where_the_tools_are.rs`: the unfiltered-lib-run detector treats a run as
  narrowed when it names packages (`-p <pkg>`, `--package <pkg>`, `--package=<pkg>`), none is the root crate
  `forjar`, and `--workspace` is absent (cargo runs the union of every `-p`). Detector cases added; no workflow is read
  differently by this change.
- `docs/roadmaps/roadmap.yaml`: PMAT-194 row labels. `CHANGELOG.md`: Unreleased entry.
- `.github/` is byte-identical to origin/main in this branch (`git diff origin/main --stat -- .github/` is empty).
  An earlier head added three steps to the `examples-validate` job (the `forjar-contracts` suite and two
  falsifiers, #452). That hunk is gate content and was removed from this PR by ruling; it becomes its own PR.
  **The contracts-crate CI step is NOT in this PR and is not measured by CI.** Neither is the build.rs falsifier:
  no CI job runs `falsification_build_rs_manifest_dir_runtime`.
- Moved out of the branch with the hunk (kept as `ci.yml.patch` and the test file under the cop's handoff
  directory `gate-pr-909/`): `tests/falsification_ci_runs_contracts_crate_tests.rs`, which asserts the removed step.

## Red on the PR and its root cause

`dogfood-guards` failed with `no test target named falsification_coverage_gate_mutation_scope`; `gate` only
aggregates it.
1. The job exited 101 on its first `cargo test --test` step.
2. cargo found no such test target in the checked-out tree.
3. The stale branch still carried the workflow step that names it.
4. origin/main removed the target and its step in #973.
5. A branch that does not merge the default branch keeps a step whose subject was deleted. Root cause is
   staleness, owned by the branch.
Fix: origin/main merged into the branch (clean, behind main = 0 at the last fetch). No reference to the deleted
target remains in the tree.

## Review round 1 findings handled

- Detector early-returned on the first non-root `-p` (`-p forjar-contracts -p forjar` read as narrowed) and did not
  parse `--package=<pkg>`: fixed in the detector; cases RED before, GREEN after.
- `build.rs` lacked the rerun directive: added; test RED before, GREEN after.
- The `option_env!` blind spot in the build.rs source check: fixed.
- The ci.yml gate-content finding: resolved by removing the hunk from this PR.

## Contract and falsifier test names

- Contract: forjar#456 — build.rs resolves `contracts/` at run time and re-runs when the manifest dir changes.
- `falsification_build_rs_manifest_dir_runtime::build_rs_has_no_compile_time_manifest_dir`
- `falsification_build_rs_manifest_dir_runtime::build_rs_reads_manifest_dir_at_run_time`
- `falsification_build_rs_manifest_dir_runtime::build_rs_reruns_when_the_manifest_dir_changes`
- `falsification_tool_jobs_run_where_the_tools_are::the_detector_does_not_pin_what_cannot_reach_the_test`

## Commands run and results

Host kfocus, through the capped wrapper `fc-pre.sh pr-909`, tree with `.github/` equal to origin/main:

- `cargo fmt --all -- --check`: rc=0
- `cargo clippy --all-targets -- -D warnings`: rc=0
- `cargo test --lib`: rc=0, 13785 passed, 0 failed, 4 ignored
- `cargo test --test falsification_tool_jobs_run_where_the_tools_are`: rc=0, 4 passed
- `cargo test --test falsification_build_rs_manifest_dir_runtime`: rc=0, 3 passed
- `cargo test --test falsification_version_matches_manifest`: rc=0, 4 passed
- `cargo test --test falsification_contract_citations_resolve`: rc=0, 7 passed
- `cargo test --test falsification_no_orphaned_source_files`: rc=0, 3 passed
- `cargo test --test examples_validate`: rc=0, 2 passed

RED before the review-round fixes (kfocus, commit with only the new cases): `build_rs_reruns_when_the_manifest_dir_changes`
FAILED and `the_detector_does_not_pin_what_cannot_reach_the_test` FAILED ("missed an unfiltered run:
cargo test --locked -p forjar-contracts -p forjar"); GREEN after the fix commit.

## Not covered

- The contracts-crate CI step and its falsifier (moved out of this PR); `forjar-contracts` unit tests were not
  run on the final tree and no CI job runs them for this change.
- Nine integration targets red on plain origin/main that no CI job runs (forjar#858), measured earlier on
  framework16 against both the PR tree and the origin/main tree, neither fixed nor hidden:
  `falsification_budget_dry_run` (3 tests), `falsification_comply_count_cannot_run_inside_itself`,
  `falsification_cron_check_reads_the_owner_crontab`, `falsification_e02_controlmaster_before_the_drift_gate`,
  `falsification_every_ci_job_runs_on_the_fleet`, `falsification_io_hash_answers_for_its_machine`,
  `falsification_release_workflow_shape`, `falsification_untyped_error_sites_ratchet`,
  `falsification_verb_cli_reader_parity` (2 tests).
- An all-targets integration run on the final tree, and a clean-room run: not measured.

verdict: PASS - the build.rs change and the detector change are measured on kfocus (fmt, clippy, lib, the targets this PR adds or edits); the contracts-crate CI step is not in this PR and is not measured by CI, and the reds no CI job runs are listed under Not covered
IMPL-PMAT-194-RECEIPT-END
