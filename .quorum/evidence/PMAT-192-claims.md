# Claims — Refuse contradictory machine flags in apply / apply --check (PMAT-192 / forjar#950)

1. C1: `check_existence` (`src/cli/apply_selection/closure.rs:189`) refuses disjoint `--only-machine X -m Y` and identical `--exclude-machine X -m X` by name while allowing `--only-machine X -m X`.
2. C2: `Selectors.machine` (`src/cli/apply_selection/closure.rs:55`) never prunes `config.resources`, preserving `-m` as an executor/lock filter downstream.
3. C3: Both `forjar apply` (`src/cli/apply.rs:93`) and `forjar apply --check` (`src/cli/dispatch_apply_check.rs:57`) wire `-m` into `Selectors` and fail closed before lock writes (`tests/falsification_apply_machine_contradiction.rs:28`).
4. C4: Formatting, clippy, `file-health`, and `PMAT-192` Gate A traceability pass cleanly on `t2fw13`.
5. R1: Round 1 wired a dead `machine` field into `cmd_check` (`src/cli/check.rs`), violating `file-health`.
6. R2: `src/cli/apply_selection/closure.rs:28` and `src/cli/apply_selection/closure.rs:55` doc comments state `machine` is `"checked against --only-machine only"`.
