# Judges — `check` unmeasured exit 4 & `lock-prune` stack scoping (PMAT-659 / forjar#902)

Round count and heads: Round 1 `2a3ac2e3` (`3/3 PASS`); see `PMAT-659-lanes.md`. Every claim `C1..C4` and `R1..R2` met 3 independent refuters across all 3 lanes.

## CONFIRMED

1. [code] C1 — `src/cli/check.rs:140` classifies SSH `exit_code == 255` (guarded by `transport::is_ssh_transport(machine)`) and transport spawn `Err(_)` as `"unmeasured"`, while local exit `255` and non-255 remote check failures remain `"fail"`.
   - evidence: `tests/falsification_659_check_unreachable_is_unmeasured.rs:45` passed `4/4` on `<host>`, and perturbing `"unmeasured"` to `"fail"` in `src/cli/check.rs:140` turned it RED (`rc=101`).
2. [code] C2 — `src/cli/check_json.rs:28` reports `"unmeasured"` counts with `"all_passed": total_fail == 0 && unmeasured == 0` (`"detail"` key for unmeasured entries), and `src/core/error.rs:12` maps `CHECK_UNMEASURED_MARKER` to exit code `4` only when `total_fail == 0`.
   - evidence: `unreachable_host_json_status_is_unmeasured` in `tests/falsification_659_check_unreachable_is_unmeasured.rs:62` verified both JSON schema fields and exit-code precedence (`1` over `4` over `0`).
3. [test] C3 — `src/cli/lock_core.rs:368` and `src/cli/lock_prune_scope.rs:15` scope `lock-prune -f` to machines declared in the target config and refuse shared-stack machines before mutating any lock on disk.
   - evidence: `tests/falsification_661_lock_prune_scoped_to_config.rs:42` passed `2/2` on `<host>`, and reverting `src/cli/lock_core.rs` to `origin/main` in `<SCRATCH>` failed `prune_from_one_stack_leaves_the_other_stacks_locks_alone` (`FJ_902_REVERT_RC=101`).
4. [doc] C4 — `src/cli/commands/apply_args.rs:342` updates `--check` CLI help (`exit 1 = changes needed, exit 4 = a host could not be reached`) and `CHANGELOG.md:8` documents both `forjar#659` and `forjar#661`.
   - evidence: `cargo fmt --all -- --check` (`rc=0`), `cargo clippy --all-targets -- -D warnings` (`rc=0`), and all 9 `dogfood-guards` targets passed on `<host>`.

## REFUTED

1. [doc] R1 — `docs/audits/impl-PMAT-659-receipt.md` recorded `verdict: PARTIAL` over 10 failing full-suite binaries without separating host-state artifacts (`/tmp` at 95%, non-root uid, `ulimit -u`) from repo-level failures.
   - corrected: Verified on `<host>` that all PR-touched targets (`falsification_659_check_unreachable_is_unmeasured` `4/4`, `falsification_661_lock_prune_scoped_to_config` `2/2`, and all 9 `ci.yml` `dogfood-guards` binaries) pass `rc=0`.
2. [test] R2 — Dropping only `.filter(|n| config.machines.contains_key(n))` in `src/cli/lock_prune_scope.rs:25` leaves `tests/falsification_661_lock_prune_scoped_to_config.rs:42` green (`rc=0`) because `refuse_if_owned_by_other` aborts on `shared` before `only-a` is reached.
   - corrected: Measured on `<host>` (`<SCRATCH>`) that reverting `src/cli/lock_core.rs` to `origin/main` (`! cmp -s` verified) drives `prune_from_one_stack_leaves_the_other_stacks_locks_alone` RED (`FJ_902_REVERT_RC=101`), proving end-to-end discrimination of the `#661` fix.
