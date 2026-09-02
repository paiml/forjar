# pmat MCP lane — #406

`analyze_vacuous_tests` over the whole worktree: 17834 tests examined. Three
`no-failure-mode` findings sit in files this branch touches
(`src/cli/tests_apply_helpers.rs::test_fj225_run_notify_failure_silent`,
`src/core/executor/tests_run_capture.rs::capture_output_nonexistent_dir_noop`
and `::update_meta_missing_dir_noop`); all three are PRE-EXISTING "does not
panic on a missing directory" smoke tests in files the branch extended, named
and accepted in the receipt. ZERO findings in the falsification file, in
`tests_redaction.rs`, or in any production file the branch touches.

Falsification (tests kept, production reverted), re-run for this receipt:
see the receipt's `falsification` block for the exact red set. Both
schedulers were exercised: `no_state_file_contains_the_resolved_secret` and
`..._under_parallel` go red together when the redaction is removed, which is
the E09 drift class caught one path early.

`falsification_e04_run_log_secrets`: 6 of 6 green on the branch. Full lib
suite green; clippy `-D warnings` 0; fmt clean — counts in `gates`.

Jidoka (recorded in .pmat/jidoka.jsonl): while this receipt was being
re-anchored, the gate's own `cargo test` run — inside the pre-push hook, with
GIT_DIR exported — let this branch's falsifier commit INTO the branch: its
`git add -A && git commit -m legacy` in a tempdir acted on the worktree
repository and removed 2,556 tracked files. Recovered from the last good
commit without `--hard`. The gate now strips GIT_* from the test environment
and the test helper scrubs it too; proven by re-running the case with GIT_DIR
pointed at the worktree and asserting HEAD unchanged.
