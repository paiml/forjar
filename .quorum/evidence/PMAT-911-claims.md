# Claims — refresh-only selector scoping, canary pre-checks & plan drift disclosure (PMAT-911 / forjar#941)

1. C1: `apply --refresh-only` resolves all apply selectors (`-r`, `--resource-filter`, `-t/--tag`, `--only-machine`, `--exclude-machine`, `-m`) before refreshing lock entries (`src/cli/dispatch_apply_check.rs:55`, `src/cli/apply_variants.rs:375`, `src/cli/tests_refresh_only_selection.rs:145`).
2. C2: `apply --canary-machine` runs `apply_pre_checks(args)` once prior to canary and fleet execution (`src/cli/dispatch_apply_b.rs:180`, `tests/falsification_canary_runs_prehooks.rs:18`).
3. C3: `apply --plan` preflight discloses `DRIFT_GATE_NOT_RUN` on `stderr` without polluting `--json` `stdout` (`src/cli/apply_from_plan_gates.rs:84`, `src/cli/tests_apply_from_plan.rs:35`).
4. C4: Formatting, clippy, `file-health` (`<= 500` lines), and `PMAT-911` Gate A receipt (`docs/audits/impl-PMAT-911-receipt.md`) pass cleanly.
5. R1: Round 1 (`@a7a77e0`) claimed `resolve_selection` alone scoped `--refresh-only -t <tag>` without explicit post-selection tag retention.
6. R2: `dispatch_apply_check` doc comment above `select_for_refresh` claimed `tag` is not a `Selectors` field.
