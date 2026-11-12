# Judges — Refuse contradictory machine flags in apply (PMAT-192 / forjar#950)

Round count and heads: see `PMAT-192-lanes.md`. Every claim met 3 independent refuters across Correctness, Failure Modes, and Contract/Falsifiers on `t2fw13`.

## CONFIRMED

1. [diff] C1 — `check_existence` in `src/cli/apply_selection/closure.rs:189` refuses disjoint `--only-machine X -m Y` and identical `--exclude-machine X -m X` with actionable diagnostics while admitting agreeing `--only-machine X -m X`.
   - evidence: `13/13` `apply_scope` unit tests in `src/cli/tests_apply_scope.rs:240` passed on `t2fw13`, and reverting either guard in `src/cli/apply_selection/closure.rs:189` turned the suite RED (`FJ950_REV1_KILLED`, `FJ950_REV2_KILLED`).
2. [diff] C2 — `Selectors.machine` in `src/cli/apply_selection/closure.rs:55` is read exclusively inside `check_existence` and never prunes `config.resources` during dependency closure resolution.
   - evidence: full call-site inspection across `src/cli/apply_selection/closure.rs:55` confirmed `sel.machine` participates only in contradiction checks after `check_machine` existence validation, leaving downstream executor/lock filtering unchanged.
3. [diff] C3 — Both `cmd_apply_scoped` (`src/cli/apply.rs:93`) and `selectors_of` (`src/cli/dispatch_apply_check.rs:57`) populate `Selectors.machine`, refusing contradictions across `apply`, `apply --check`, and `apply --refresh-only` (`tests/falsification_apply_machine_contradiction.rs:28`).
   - evidence: `3/3` binary tests in `tests/falsification_apply_machine_contradiction.rs:28` plus a hostile 4th CLI case (`forjar apply --check --exclude-machine local -m local`) exited non-zero on `t2fw13` with zero `state/` directory created.
4. [diff] C4 — `cargo fmt --check` and `cargo clippy --lib --tests -- -D warnings` pass on `t2fw13`, all touched `.rs` files remain `<= 492` lines (`src/cli/apply.rs:490`), and `docs/audits/impl-PMAT-192-receipt.md` satisfies Gate A.
   - evidence: `t2fw13` measured `apply.rs = 492`, `closure.rs = 429`, `dispatch_apply_check.rs = 84`, `tests_apply_scope.rs = 291`, and `falsification_apply_machine_contradiction.rs = 75` lines (`<= 500`).

## REFUTED

1. [diff] R1 — Round 1 added `machine: machine_filter` to `cmd_check` (`src/cli/check.rs`), which accepts neither `--only-machine` nor `--exclude-machine`, creating dead code and pushing `check.rs` to `514` lines (`> 500` ceiling).
   - corrected: refuted in Round 1 and dropped in `7b335a09` (`git diff 5b8f8123..7b335a09 -- src/cli/check.rs` is empty), while `forjar apply --check` remains covered through `src/cli/dispatch_apply_check.rs:57`.
2. [doc] R2 — Doc comments at `src/cli/apply_selection/closure.rs:28` and `src/cli/apply_selection/closure.rs:55` claimed `-m` is `"checked against --only-machine only"`, omitting the `--exclude-machine` check right below at `src/cli/apply_selection/closure.rs:196`.
   - corrected: runtime code at `src/cli/apply_selection/closure.rs:196` and `src/cli/dispatch_apply_check.rs:46` checks both `--only-machine` and `--exclude-machine`; comment inaccuracy queued for fix by `forjar-cop`.
