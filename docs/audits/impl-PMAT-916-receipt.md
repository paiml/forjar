# Implementation receipt — PMAT-916 — ResourceType serde spelling in the CLI; cron check guard (PR #943)

verdict: PARTIAL — merged with main, fmt/clippy/lib and the touched integration targets are green on kfocus; the clean-room run, the required CI jobs on the merged head, and the ticket-id fix for the earlier PMAT-198 commits are open.

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

IMPL-PMAT-916-RECEIPT-END
