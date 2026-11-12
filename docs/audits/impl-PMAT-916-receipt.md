# Implementation receipt — PMAT-916 — ResourceType serde spelling in the CLI; cron check guard (PR #943)

verdict: PARTIAL — merged with main, round-1 review defects (truncated-block guard, dead "Unknown" comparisons) fixed with red-then-green tests and green on kfocus (fmt, clippy, lib, 17 integration targets); CB-200 reads 652 on plain main and on this branch against ceiling 651 (not caused by this PR, not fixed here), and the CB-2113 trailers of the earlier commits, CI on the new head and the clean-room run are open.

## Identity

- ticket PMAT-916 (roadmap row added in this commit; the issue number 916 is assumed from the PMAT-n/#n pairing and was not read from GitHub); kind: code
- PR #943, branch `fix/f433-resourcetype-display`, lane branch `lane-943`; closes #433, #848, #445
- merged `origin/main` @ dbd1ec3e into the branch (merge commit 7b0a4c6b); no rebase, nothing pushed

## Merge conflicts (2 files, both from main's own #848 fix)

- `src/resources/cron.rs`: main replaced the const `CRONTAB_CHECK_GUARD` with a `crontab_check_guard(u)` that probes the read and skips on a line starting `sudo:`. This branch carries the same probe plus a missing-sudo form (`sh: sudo: not found`, `bash: line N: sudo: command not found`), `Sorry, user ...`, and the #445 intact-block guard. Resolved by keeping the branch side: it is a superset of main's behaviour (main's `sudo:` refusal is still a SKIP).
- `src/resources/tests_cron_exec.rs`: main's three `fj848_*` tests (`g848a/b/c`) cover the same three cases as this branch's `fj848_*` tests (`t848a/b/c`, the refusal test now also covers the `Sorry, user` form), plus the #445 tests. Resolved by keeping the branch side. One main assertion is not carried over: `!stdout.contains("FORJAR_SKIP")` in the no-crontab test; `contains("missing:backup")` in the branch test implies it, because a SKIP exits before the verdict.

## CI red on head 37e4d34 (five whys)

1. `gate` failed because `dogfood-guards` failed (`cargo test --locked --test ...` exit 101, cargo's "available test targets" list in the log tail).
2. cargo listed targets because one `--test` name in the job did not exist in the tree it ran on.
3. main's `.github/workflows/ci.yml` still named a falsification test that main's #973 had deleted.
4. main fixed that in #982 (5b8f8123); the run was made before #982.
5. The branch was not merged with main afterwards, so it kept the red. Not a defect of this branch's code: `5b8f8123` is an ancestor of the merged head (`git merge-base --is-ancestor 5b8f8123 HEAD`, rc 0), and all nine targets named by the job exist and pass (below).

I read the log of the failed job once with `gh run view` (3 of 3 allowed `gh` calls); I did not see the name of the missing target in the part I read, so step 3 is inferred from the target list and from #982's title and diff, not read from the log.

## Measured on kfocus (rust toolchain of the host, wrapper `fc-pre.sh pr-943`, tree = merged head 7b0a4c6b)

- `cargo fmt --all -- --check`: rc 0
- `cargo clippy --all-targets -- -D warnings`: rc 0
- `cargo test --lib`: 13791 passed, 0 failed, 4 ignored (rc 0)
- `cargo test --test resource_type_cli_output --test resource_type_spelling`: 10 passed and 3 passed, rc 0
- dogfood-guards command (9 targets, `--locked --no-fail-fast`), tree with a full git clone: all 9 `test result: ok`, rc 0
- same 9 targets on a tree synced without `.git`: 8 ok, 1 failed (`falsification_gate_a_the_pr_and_its_commits_name_one_ticket::the_shipped_floor_is_the_commit_it_says_it_is`, "does not resolve in a repository that is NOT shallow"). An artefact of the missing `.git`, not of the change; it passes in the clone run above.
- touched integration targets, rc 0: `falsification_cron_absent_failclosed` 15, `falsification_390e_nested_shell_strictness` 6, `falsification_mutation_observability` 37, `falsification_query_runlog` 28, `falsification_scoring_static` 21, `falsification_unobservable_sentinel_survives_i8` 5; all passed, 0 failed.
- flips: none observed (no test was run twice with differing verdicts).

## Not measured

- `cargo test --test '*'` over the whole `tests/` directory; the LAB reds named in forjar#858 (`falsification_untyped_error_sites_ratchet`, `falsification_budget_dry_run`, `falsification_release_workflow_shape`) were not run.
- the required CI jobs on the merged head (nothing is pushed), and the clean-room run.
- red-then-green for this branch's tests against pre-fix code: not re-run in this lane; the RED/GREEN history is in the branch commits.

## Open items

- Commits `86fca4be` and `37e4d34d` carry `Pmat-Ticket: PMAT-198`; this commit carries PMAT-916. Gate A reads the squash message of the merged PR, so whoever squashes must make the PR title/body claim one ticket. History was not rewritten.
- the issue number of PMAT-916 should be confirmed against GitHub.

## Round 1 (three non-author reviews of a4e28c3, all FAIL)

1. `intact_block_guard` accepted a block whose entry line was deleted or commented out whenever another line followed the cmd marker, so `delete_block` dropped that line (a sibling's `# forjar:other`, a comment, a blank). Red first (commit fa756480, kfocus, `cargo test --lib fj445`): 5 failed (sibling marker, comment, indented comment, blank line, absent-state with sibling), 5 passed. Fix (81fcf1ee, `src/resources/cron.rs`): the entry line must not be blank or start with `#`. Green: the same 10 pass. A guard that must not widen is pinned by `fj445_entry_lines_of_every_ordinary_shape_stay_intact` (schedule line, `@reboot`, indented line, `MAILTO=` line). Limit: a truncated block followed directly by another real job line cannot be told from the entry and is treated as intact.
2. `res.resource_type.to_string() == "Unknown"` in `validate_deep.rs` and `validate_quality.rs` was never true (Display is snake_case; the enum has no such variant, and the Debug form on main was never true either). Red first: `tests/resource_type_spelling.rs::cli_never_compares_resource_type_text_to_a_literal_display_cannot_produce` named both lines. Fix: the dead branches are removed; behaviour is unchanged (both checks always passed). Green: 5 of 5 in that target.
3. CB-200: measured with `bash scripts/cb200-ratchet.sh` on kfocus (pmat 3.42.0, tree with .git, fresh index): plain `origin/main` @ dbd1ec3e reads 652; the branch at a4e28c3b reads 652; the branch at 81fcf1ee reads 652. The ceiling in `scripts/ratchets/cb200-baseline.json` is 651 and was recorded with pmat 3.39.0, so the 652 on main may be the instrument (3.42.0) or main's own growth; this PR adds no function below grade A. The ceiling and the baseline were not edited. `falsification_cb200_ratchet_measures_this_tree` passes (9 of 9 in the dogfood-guards group).
4. CB-2113: not fixed, history not rewritten. Open: `2c677009` and `8b9ae178` have no `Pmat-Ticket`, `86fca4be` and `37e4d34d` claim PMAT-198, `37e4d34d` and `a4e28c3b` have a blank line before `Co-Authored-By`. The commits added in this round carry `Pmat-Ticket: PMAT-916` in the last paragraph.

Measured on kfocus at 81fcf1ee: `cargo fmt --all -- --check` rc 0; `cargo clippy --all-targets -- -D warnings` rc 0; `cargo test --lib` 13797 passed, 0 failed, 4 ignored; the nine dogfood-guards targets plus `resource_type_cli_output`, `resource_type_spelling`, `falsification_cron_absent_failclosed`, `falsification_390e_nested_shell_strictness`, `falsification_mutation_observability`, `falsification_unobservable_sentinel_survives_i8`, `falsification_query_runlog`, `falsification_scoring_static`: all ok, rc 0.

Not measured: the coverage-gap note of lane 2 (`state_dir()` in `tests/resource_type_cli_output.rs` does not populate a `package` resource, so a revert of that arm in `status_cost.rs` stays green); CI on the new head; the clean-room run.

File-health (500-line limit, `.github/workflows/lint.yml`): `src/resources/tests_cron_exec.rs` read 576 lines (430 on main); the #848/#445 guard tests moved unchanged to `src/resources/tests_cron_exec_guard.rs`, leaving 389 and 193 lines (`wc -l`); 24 `#[test]` before and 24 after (`grep -c`). On kfocus at d469f070: `cargo fmt --all -- --check` rc 0, `cargo clippy --all-targets -- -D warnings` rc 0, `cargo test --lib` 13797 passed, 0 failed, 4 ignored (same count as before the split). No other .rs file changed by this PR grows past its size on main (checked against origin/main 03375fcc with `git diff --name-status` and `wc -l`).

IMPL-PMAT-916-RECEIPT-END
