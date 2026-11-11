# Judges — refresh-only selector scoping & canary pre-checks (PMAT-911 / forjar#941)

Round count and heads: see `PMAT-911-lanes.md`. Every claim met 3 independent refuters across Correctness, Failure Modes, and Contract/Falsifiers on `t2fw13`.

## CONFIRMED

1. [diff] C1 — `apply --refresh-only` resolves all resource/machine selectors and narrows `config.resources` by `-t/--tag` before `refresh_machine_lock` hashes state (`src/cli/dispatch_apply_check.rs:55`, `src/cli/apply_variants.rs:375`, `src/cli/tests_refresh_only_selection.rs:145`).
   - evidence: `5/5` `refresh_only` unit tests in `src/cli/tests_refresh_only_selection.rs:145` passed on `t2fw13`, proving unselected and untagged lock entries remain byte-for-byte untouched while unknown selectors fail closed before any lock write.
2. [diff] C2 — `apply --canary-machine` executes `apply_pre_checks(args)` once inside `apply_early_exits` before canary or fleet convergence begins (`src/cli/dispatch_apply_b.rs:180`, `tests/falsification_canary_runs_prehooks.rs:18`).
   - evidence: `3/3` integration tests in `tests/falsification_canary_runs_prehooks.rs:18` passed on `t2fw13`, and reverting `apply_pre_checks(args)?` in `src/cli/dispatch_apply_b.rs:180` turned the binary test RED (`FJ941_REV2_KILLED`).
3. [diff] C3 — Saved-plan apply preflight (`run_plan_apply_gates`) emits `DRIFT_GATE_NOT_RUN` on `stderr` rather than `stdout`, preserving valid `--json` output (`src/cli/apply_from_plan_gates.rs:84`, `src/cli/tests_apply_from_plan.rs:35`).
   - evidence: `34/34` `apply_from_plan` unit tests passed on `t2fw13`, and replacing `DRIFT_GATE_NOT_RUN` in `src/cli/apply_from_plan_gates.rs:84` broke `plan_gate_drift_note_names_what_did_not_run` (`FJ941_REV3_KILLED`).
4. [diff] C4 — All touched Rust files stay within the 500-line `file-health` ceiling (`src/cli/dispatch_apply_b.rs:495`, `src/cli/apply_variants.rs:430`), `cargo fmt --check` and `cargo clippy --lib --tests -- -D warnings` pass on `t2fw13`, and `PMAT-911` traceability is complete.
   - evidence: `t2fw13` measured `dispatch_apply_b.rs = 496` lines and `apply_variants.rs = 471` lines, `clippy` exited `0`, and `docs/audits/impl-PMAT-911-receipt.md` terminates with `IMPL-PMAT-911-RECEIPT-END`.

## REFUTED

1. [diff] R1 — Round 1 (`@a7a77e0`) claimed calling `resolve_selection` in `cmd_apply_refresh_only` already honoured `-t/--tag`, whereas `resolve_selection` existence-checks `Selectors.tag` without pruning `config.resources` (`src/cli/dispatch_apply_check.rs:55`).
   - corrected: `agy` Round-1 review refuted `@a7a77e0` with a concrete reproducer refreshing untagged resources under `-t alpha`; fixed at `78b309bc` via `select_for_refresh` and pinned by `src/cli/tests_refresh_only_selection.rs:145` (`FJ941_REV1_KILLED` on revert).
2. [doc] R2 — The new doc comment above `select_for_refresh` stated `"The tag is not a Selectors field"` even though `selectors_of` (`src/cli/dispatch_apply_check.rs:55`) populates `Selectors.tag` for `check_existence`.
   - corrected: `Selectors.tag` DOES exist for unknown-tag existence checking, while `select_for_refresh` retains matching tagged resources afterwards because normal apply filters tags inside the executor loop; comment queued for precision fix by `forjar-cop`.
