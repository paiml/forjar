# pmat MCP lane — #404

`analyze_vacuous_tests` over the whole worktree: 17827 tests examined; ZERO
vacuous tests and ZERO conditional skips in any path this branch touches
(`src/cli/apply.rs`, `src/cli/apply_drift.rs`, `src/cli/apply_mux.rs`,
`src/cli/apply_preflight.rs`, `src/core/executor/machine.rs`,
`src/transport/ssh_mux.rs`,
`tests/falsification_e02_controlmaster_before_the_drift_gate.rs`).

Falsification, per hunk, tests kept and production reverted:

- orphan-scope arm in `GateScope::covers` reverted → `gate_is_scoped_by_exclude`
  **RED** ("the gate probed alpha-b, which `--exclude alpha-b` excluded").
- `ssh_machines_in_scope` reverted to `-m`-only →
  `resource_and_tag_filters_narrow_the_fleet` **RED**.
- (from the first cut, re-run) group threading reverted → 1 red; `machine.rs`
  ownership hunk reverted → 1 red; all of `src/` reverted → 0 passed, 4 failed.

Full lib suite green; `cargo fmt --check` clean; `cargo clippy --all-targets
-D warnings`: 0 errors. Exact counts in the receipt's `gates`.
