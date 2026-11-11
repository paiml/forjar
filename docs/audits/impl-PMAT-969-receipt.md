# Implementation receipt — PMAT-969 — a timeout that fires before the child is spawned still kills it

verdict: PASS — when `exec_script_timeout`'s limit fires before the worker has published its child PID, the late child's process group is killed as soon as it is recorded, so the join is bounded by the limit and not by the script.

## Identity

- ticket PMAT-969 (forjar#969; found via paiml/infra#3590); kind: code
- PR #932, branch `f20-i6/timeout-spawn-race`, merge base with `main` @ 5b8f8123
- files: `src/transport/mod.rs`, `src/transport/child_group.rs` (new), `src/transport/tests_timeout.rs`, `docs/roadmaps/roadmap.yaml` (row PMAT-969 added)

## Root cause (five whys)

1. `test_resource_timeout_bounds_wave_apply_script` ran 30 s or more under `--resource-timeout 2` on loaded hosts.
2. The timeout fired, but no process was killed, so `join()` waited for `sleep 30`.
3. `kill_worker_child_group` kills only a PID it finds in the slot, and the slot was still empty.
4. Under load the worker thread had not yet spawned its child when the limit expired.
5. Nothing told the worker, once it did spawn, that its timeout had already fired.

## Contract and falsifier

Contract: if `kill_worker_child_group` finds the slot empty and the worker not done, it leaves `CANCELLED_PID` (0, never a real child, never passed to `kill`) in the slot; `record_child_pid`, on finding that marker, kills the new child's process group before recording its PID.

Falsifier: `transport::tests_timeout::a_timeout_before_the_child_spawns_still_kills_it` — the timeout path runs against an empty slot, then `exec_local("sleep 30")` must return in under 10 s without success.

## file-health

The fix grew `src/transport/mod.rs` from 497 to 513 lines (limit 500). The child process-group code (`ChildPidSlot`, `record_child_pid`, `configure_process_group`, `kill_group_command`, `kill_process_group`, `kill_worker_child_group`) moved unchanged to `src/transport/child_group.rs` (96 lines) and is re-exported under the same paths; `mod.rs` is now 424 lines. No behaviour change in the move.

## Evidence (kfocus, rust 1.99.1, `nice -n 19`, 2026-11-11)

RED, with the marker write removed (`None => {}`) and the test kept (`cargo test --lib transport::tests_timeout`):

    test transport::tests_timeout::a_timeout_before_the_child_spawns_still_kills_it ... FAILED
    test result: FAILED. 6 passed; 1 failed; 0 ignored; 0 measured; 13761 filtered out; finished in 30.01s

GREEN, after the move (`cargo test --lib transport::`):

    test result: ok. 238 passed; 0 failed; 0 ignored; 0 measured; 13530 filtered out; finished in 5.52s

`cargo clippy --lib --tests -- -D warnings` rc=0 on kfocus. `cargo fmt --all -- --check` was rc=1 on that tree (import order in `mod.rs`); after `cargo fmt --all` it is rc=0. The reformat changed import order only and was not re-compiled; the PR's CI is the check on the final tree.

IMPL-PMAT-969-RECEIPT-END
