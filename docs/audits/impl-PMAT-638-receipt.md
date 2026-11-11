# impl-PMAT-638 receipt

Ticket: PMAT-638 (forjar#638), PR #906, branch lane-906 (remote s-forjar-1/638).

## What changed

- `src/resources/file.rs`, `src/core/shell_escape.rs`: the file resource's atomic
  replace stages at `<path>.forjar-new.$$` (per-run) instead of the fixed
  `<path>.forjar-new`; `decode_written_file` follows the new name.
- `src/resources/package_check.rs`: a declared `completion_check` is appended to
  the package check verdict (`finish`), for the apt, cargo, uv and brew arms.
- Tests: `src/resources/tests_file_staging.rs` (new), `tests_file.rs`,
  `tests_file_b.rs`, `tests_package_d.rs`, and the generated-script lint corpus
  in `src/transport/tests_generated_scripts_lint.rs` (now covers `state: file`
  with `content` and with `source`).
- `CHANGELOG.md`: Unreleased entry. `docs/roadmaps/roadmap.yaml`: PMAT-638 row.
- Accepted residue, stated in the code: a PID is guessable and an apply that
  dies before the `mv -f` leaves its `.forjar-new.<pid>` behind.

## Why PR #906 was red (five whys)

1. CI showed `dogfood-guards` and `gate` red. The `dogfood-guards` job failed
   with `error: no test target named falsification_coverage_gate_mutation_scope`.
2. The job's `cargo test --test ...` list named a target that did not exist in
   the merge ref the job built.
3. The branch was cut before main changed that workflow list, so the job
   (workflow from one side, `tests/` from the other) disagreed with itself.
4. Nothing in the PR touches `.github/workflows/` or `tests/falsification_*`.
5. Root cause: a stale branch, not a defect in the change. Merging origin/main
   (commit 905fd5af) aligned the two; every `--test` target the current
   `dogfood-guards` job names exists in the tree (checked by listing them
   against `tests/`). `gate` is the aggregate of that job and follows it.

No code in the PR's owning modules needed a fix: fmt, clippy and all 13789 lib
tests were already green on the merged tree.

## Failures that are not this PR

The full `cargo test --no-fail-fast` on host t2yoga reports 10 failing
integration targets. The same 10 fail on plain origin/main (dbd1ec3e)
exported without the PR, run on the same host:
`falsification_budget_dry_run`, `falsification_comply_count_cannot_run_inside_itself`,
`falsification_cron_check_reads_the_owner_crontab`,
`falsification_e02_controlmaster_before_the_drift_gate`,
`falsification_every_ci_job_runs_on_the_fleet`,
`falsification_io_hash_answers_for_its_machine`,
`falsification_release_workflow_shape`, `falsification_replace_running_binary`,
`falsification_untyped_error_sites_ratchet`, `falsification_verb_cli_reader_parity`.
Causes seen in their output: host properties of t2yoga (root-owned `ulimit -u`,
a filesystem that lists directories sorted, no reachable remote, `cp` over a
running binary succeeding), the repo's own workflow pins (`@<sha>` where the
test expects `@main`, tag pushes in bench/ci/coverage), and
`untyped_error_fns = 790` against a ceiling of 778 on main itself. They are
outside the PR's diff and were not changed. One earlier run also showed
`fj446_doctor_machine_reports_path_and_disk` red because the host disk was 99%
full at the time; it passes with the disk at 45%.

## Contract and falsifier tests

- Contract bindings: build reports `CONTRACT: 45/45 bindings bound`.
- Falsifiers (all green): `resources::tests_file_staging::test_638_staging_name_is_per_run`,
  `resources::tests_file::fj634_a_parked_script_finishes_on_its_old_bytes`,
  `resources::tests_file::fj634_an_unreadable_source_leaves_no_staged_file`,
  `resources::tests_package_d::gh694_completion_check_reaches_the_check_script`,
  `gh694_no_completion_check_adds_nothing`,
  `gh694_failing_completion_check_diverges_a_present_package`,
  `transport::tests_generated_scripts_lint::every_generated_apply_script_survives_the_i8_gate`
  (with the new `file/state-file-content` and `file/state-file-source` corpus rows).

## Commands run (host t2yoga, ~/src/fc-pre/pr-906)

- `cargo fmt --all -- --check`: exit 0
- `cargo clippy --all-targets -- -D warnings`: exit 0
- `cargo test --no-fail-fast` (PR tree): exit 101, 10 targets failed, identical
  set to plain origin/main; lib 13789 passed, 0 failed
- `cargo test --no-fail-fast --test <the 11 targets>` on origin/main export: exit 101, 10 failed
- `cargo test --test falsification_release_changelog_counts_the_window --test falsification_version_matches_manifest --test falsification_crux_gate_reads_the_release_section --test falsification_crux_audit_shape` (after the CHANGELOG edit): exit 0
- `cargo test --test falsification_446_exec_facts_doctor`: exit 0

verdict: PARTIAL - PR change is green (fmt, clippy, 13789 lib tests, its falsifiers); CI red was a stale branch, cleared by the main merge; 10 integration targets stay red on origin/main itself and are outside this PR

IMPL-PMAT-638-RECEIPT-END
