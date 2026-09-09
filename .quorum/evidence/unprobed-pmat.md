# Quorum evidence — PMAT-221 — pmat and the gates

- `pmat work add` → PMAT-221, `kind:code`, `orch:fable` with `orch-basis:M>=3` (five owning modules measured before the ticket was opened: core/types, core/planner, core/task, cli, mcp); `pmat work validate` passed.
- `kind-gate.sh`: kind=code. `model-gate.sh`: model=fable class=fable decision=admit basis=file, tier 1 meets tier 1. `target-guard.sh`: PASS.
- `pmat hooks install --strict --force`: every commit carries `Pmat-Ticket: PMAT-221`; the pre-commit hook ran fmt, complexity, TDG and clippy on every commit — one commit was refused by clippy (an `examples/` literal the first sweep missed) and fixed rather than bypassed.
- `pmat analyze vacuous-tests` over the touched test paths: see the receipt for the count.
- Gate: `cargo test --workspace` exit 0 (314 binaries, 19,603 passed); clippy `--all-targets -D warnings` exit 0; rustfmt stable check exit 0; gate G `scripts/dogfood/contracts.sh` PASS.
- Not measured on this host: gate F's mutation arm (`cargo mutants`, PMAT-216); no mutation-score figure is claimed. The discriminating mutations were run by hand instead: the RED commit 1102238b is the diff's own falsifier, observed failing at every disclosure assertion with every precondition passing.
