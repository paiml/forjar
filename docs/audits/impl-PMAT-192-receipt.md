# Implementation receipt — PMAT-192 — contradictory machine selectors on `apply` are refused by name

verdict: PASS — `apply` and `apply --check` exit non-zero and name both flags when `--only-machine X` meets `-m Y` (X ≠ Y) or `--exclude-machine X` meets `-m X`; before the change each converged nothing and exited 0.

## Identity

- ticket PMAT-192 (forjar#471); kind: code
- PR #950, branch `mail-ana/471`, merge base with `main` @ 5b8f8123
- files: `src/cli/apply_selection/closure.rs`, `src/cli/apply.rs`, `src/cli/dispatch_apply_check.rs`, `src/cli/tests_apply_scope.rs`, `tests/falsification_apply_machine_contradiction.rs`

## Root cause (five whys)

1. `forjar apply --only-machine a -m b` ran nothing and exited 0.
2. `--only-machine` pruned the resource set to machine `a`; `-m` then picked executor `b`, which had no resources left.
3. The two flags were resolved in different places: `resolve_selection` saw `--only-machine`, the executor saw `-m`.
4. `Selectors` did not carry `-m`, so the resolver could not compare them.
5. An empty result was treated as "nothing to do", not as a contradiction.

## Contract and falsifier

Contract: `resolve_selection` returns `Err` naming both flags when `only_machine` and `machine` are both set and differ, or when `exclude_machine` and `machine` are both set and equal. `machine` never prunes the resource set.

Falsifiers: `tests/falsification_apply_machine_contradiction.rs` (3 tests, run the built binary) and `src/cli/tests_apply_scope.rs`.

## file-health

The PR's first head added `machine: machine_filter` to the `Selectors` literal in `cmd_check` (`src/cli/check.rs`, 513 → 514 lines; the gate refuses growth of a file over 500). `cmd_check` sets neither `only_machine` nor `exclude_machine`, so that field could never trip either refusal there. The line is removed; `check.rs` is back to main's 513 lines and behaviour is unchanged. Splitting `check.rs` under 500 is not part of this ticket.

## Evidence (kfocus, rust 1.99.1, `nice -n 19`, 2026-11-11)

RED, with both refusals removed from `resolve_selection` and the tests kept (`cargo test --test falsification_apply_machine_contradiction`):

    test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out

GREEN, on this tree:

    cargo test --test falsification_apply_machine_contradiction: test result: ok. 3 passed; 0 failed
    cargo test --lib apply_scope: test result: ok. 13 passed; 0 failed; 13756 filtered out

Checks on the same tree: `cargo fmt --all -- --check` rc=0; `cargo clippy --lib --tests -- -D warnings` rc=0.

IMPL-PMAT-192-RECEIPT-END
