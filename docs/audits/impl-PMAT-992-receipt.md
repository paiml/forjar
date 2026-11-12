# Implementation receipt — PMAT-992 — `--resource-timeout` bounds the apply with no `kill` binary on the host

verdict: PASS — the timeout path signals the script's process group by syscall (`killpg`), so an apply is bounded by `--resource-timeout` on a host or image that has no external `kill` program.

## Identity

- ticket PMAT-992 (forjar#992, found by the infra clean-room on v1.34.0, gate B2); kind: code
- branch `fix/992-timeout-kill-by-syscall`, base `main` @ dbd1ec3e
- files: `src/transport/child_group.rs`, `src/transport/mod.rs`, `src/transport/pepita.rs`, `src/transport/tests_timeout.rs`, `src/transport/tests_dispatch.rs`, `tests/falsification_992_timeout_needs_no_kill_binary.rs` (new), `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`, `docs/roadmaps/roadmap.yaml` (row PMAT-992 added)

## Root cause (five whys)

1. In `rust:1.95-slim`, `test_resource_timeout_bounds_wave_apply_script` ran 30 s under `--resource-timeout 2`, twice on the same inputs.
2. The timeout error was returned at 2 s, but the script's process group was still alive, so `join()` waited for `sleep 30`.
3. `kill_process_group` killed the group by spawning the external program `kill -9 -- -<pgid>`.
4. That image has no `kill` program (procps is not installed; `kill` exists only as a shell builtin), so the spawn failed.
5. The spawn's result was discarded (`let _ = ... .status()`), so the failure was silent and no other kill was tried. The signal depended on a host tool that forjar never checked for.

## Contract and falsifier

Contract: `kill_process_group(pid)` delivers SIGKILL to process group `pid` with `killpg(2)` and spawns no process; it returns whether the signal was delivered; it refuses group 0 (the caller's own group) and any value that does not fit a `pid_t`. `kill_pid(pid)` does the same for one process and also refuses PID 1. No file under `src/` spawns a `kill` program.

Falsifiers:

- `tests/falsification_992_timeout_needs_no_kill_binary.rs::resource_timeout_bounds_the_apply_with_no_kill_binary_on_path` — the real binary runs `apply --resource-timeout 2` on a `sleep 30` task under a `PATH` that links every tool of the host except `kill`; it must end in under 15 s and fail the apply.
- `...::no_source_file_spawns_a_kill_binary` — walks `src/` and fails on any `Command::new("kill")`.
- `transport::tests_timeout::test_992_kill_process_group_kills_a_live_group_by_syscall` — a real `sleep 30` in its own group dies of signal 9.
- `transport::tests_timeout::test_992_kill_process_group_refuses_zero_and_reports_absent` — group 0, an out-of-range value and an absent group all report "not delivered"; `kill_pid` refuses 0 and 1.

`unsafe` is forbidden in this workspace, so the syscall goes through `nix` 0.31 (`signal` feature only), a crate already in `Cargo.lock`; the lock gains one edge and no new package. `test_165_kill_group_command_targets_group` asserted the argv of the removed `kill` command and is replaced by the two `test_992_*` unit tests.

## Evidence (rust 1.99.1, 2026-11-11)

RED, the new test file on unchanged `main` code (commit 345a470e), host t2yoga, `cargo test --test falsification_992_timeout_needs_no_kill_binary`:

    test no_source_file_spawns_a_kill_binary ... FAILED
    test resource_timeout_bounds_the_apply_with_no_kill_binary_on_path ... FAILED
    apply ran 30s with no kill binary on PATH; --resource-timeout 2 did not bound it
    test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 30.47s

GREEN, with the fix, host kfocus:

    test no_source_file_spawns_a_kill_binary ... ok
    test resource_timeout_bounds_the_apply_with_no_kill_binary_on_path ... ok
    test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.31s

- `cargo test --lib -- transport:: tests_resource_timeout`: 240 passed, 0 failed.
- `cargo test --workspace`, lib target: 13786 passed, 0 failed, 4 ignored.
- `cargo clippy --all-targets -- -D warnings`: rc=0.
- `cargo fmt --all -- --check`: rc=1 on the tree that was built (import order in `src/transport/mod.rs`); rc=0 after `cargo fmt --all`. The reformat changed import order only and was not re-compiled; the PR's CI is the check on the final tree.

Not measured here: the integration targets after `falsification_budget_dry_run`. `cargo test --workspace` stopped at that target (3 of 7 failed, "the granted pass must still reclaim"); it does not touch the transport, and other lanes measured the same target red on plain `main` on these hosts. The clean-room run in `rust:1.95-slim` (the environment of the original red) is the release gate and has not been run on this change yet.

IMPL-PMAT-992-RECEIPT-END
