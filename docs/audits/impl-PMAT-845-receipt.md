# Implementation receipt — PMAT-845 — `apply` refuses a config checkout whose HEAD is not origin/main

verdict: PASS — a real clone one commit behind its origin/main is refused by `forjar apply` with a non-zero exit, the target file is never created and the state dir stays empty; the same sandbox at origin/main converges; a stale remote-tracking ref is fetched before comparing; a named PR worktree (`FORJAR_PR_WORKTREE`) is the one exception and is printed. Bypassing the check, or deleting its call site, turns the falsifier RED.

## Identity

- ticket PMAT-845 (forjar#845); kind: code
- branch `c05-n4/apply-origin-main-guard-845`, from `main` @ 269e180c
- files: `src/cli/apply_checkout_guard.rs` (new), `src/cli/tests_apply_checkout_guard.rs` (new), `tests/falsification_apply_refuses_a_stale_checkout.rs` (new), `src/cli/dispatch_apply_b.rs`, `src/cli/mod.rs`, `CHANGELOG.md`

## Where the check sits

`dispatch_apply_cmd`, inside the same `!is_read_only_apply_mode` block as the
forjar#374 operator gate, after the GH-211 inert-flag and header checks and
before `apply_early_exits`, `apply_pre_checks` (pre-scripts), backups and
`apply_execute`. So every apply exit that can touch a host (`--plan-file`,
`--canary-machine`, `--refresh-only`, a plain apply) passes it; `plan`, `drift`
and the read-only apply modes never call it.

## Policy

| checkout state | result |
|---|---|
| no `.git` in any ancestor of the config's directory | allowed (not a checkout) |
| git repo with no `origin` remote (e.g. an `--auto-commit` state repo) | allowed (not a clone) |
| `FORJAR_PR_WORKTREE=<N>` or `#<N>` | allowed, printed with HEAD sha |
| `FORJAR_PR_WORKTREE=<branch>` and HEAD is on that branch | allowed, printed with HEAD sha |
| `FORJAR_PR_WORKTREE=<branch>` and HEAD is on another branch | refused |
| `git fetch origin main` fails, or HEAD / origin/main / rev-list do not resolve | refused (unmeasured) |
| HEAD == origin/main after the fetch | allowed |
| HEAD != origin/main | refused: HEAD sha, origin/main sha, behind and ahead counts |

Every git call goes through the `GitProbe` seam; `SystemGit` scrubs an inherited
`GIT_DIR`/`GIT_WORK_TREE`/`GIT_INDEX_FILE` so a hook environment cannot point
the measurement at another repository.

## Evidence (rust 1.93.0, gx10)

- `cargo test --lib apply_checkout_guard`: 21 passed, 0 failed.
- `cargo test --test falsification_apply_refuses_a_stale_checkout`: 6 passed, 0 failed.
- `cargo test --lib cli::`: 8346 passed, 0 failed (before the last 3 coverage tests were added).
- `falsification_e04_run_log_secrets` (auto-commit into a git repo without origin), `falsification_apply_refuses_without_provenance`, `falsification_plan_file_scopes_the_apply`, `falsification_yes_is_not_an_integrity_override`, `doc_cli_parity`: all passed.
- `cargo llvm-cov` over the unit and falsifier tests: `cli/apply_checkout_guard.rs` 100.00% regions (208/208), functions (21/21), lines (121/121); the call site in `dispatch_apply_b.rs` executed 5 times.
- `cargo fmt --all -- --check` rc=0; `cargo clippy --all-targets -- -D warnings` rc=0.

## Mutants

| mutant | falsifier result |
|---|---|
| `check_apply_checkout` returns `Ok(())` first | 4 of 6 FAILED, incl. `apply_from_a_checkout_one_commit_behind_origin_main_touches_nothing` at the exit-code assertion |
| the call in `dispatch_apply_cmd` deleted | `apply_from_a_checkout_one_commit_behind_origin_main_touches_nothing` FAILED |

## Not covered

- `forjar make`, drift auto-remediation and other internal callers of `cmd_apply` do not pass through `dispatch_apply_cmd` and are not guarded; the ticket names `forjar apply` only.
- The exception is recorded on stderr, not in the per-machine event log.

IMPL-PMAT-845-RECEIPT-END
