# pmat MCP lane — #407

`analyze_vacuous_tests` over the whole worktree: 17816 tests examined; ZERO
vacuous tests and ZERO conditional skips in any path this branch touches
(`src/mcp/handlers.rs`, `handlers_drift.rs`, `mod.rs`, `registry.rs`,
`types.rs`, `tests_drift_e05.rs`, `tests_drift_adversarial.rs`,
`src/tripwire/drift/census.rs`, `src/verb/registry.rs`,
`tests/falsification_e05_verb_drift_contacts_the_host.rs`).

Falsification, tests kept and production mutated, one line at a time:

- `detect_drift_full_reported(&lock, machine, …)` → `None`:
  `drift_over_an_unreachable_machine_must_not_answer_clean` **RED**
  (`drifted: false`, no `unchecked`, the controller's copy hashed).
- `run_task_checks: false` → `true`:
  `the_readonly_verb_neither_runs_the_completion_check_nor_hides_that`
  **RED** (the trap file exists).
- `the_verb_discloses_how_much_it_inspected` is RED on main by construction
  (no `census` key existed) and green from the first #407 commit on; it is
  the guard for the denominator, not a falsifier of the second commit.

`falsification_e05_verb_drift_contacts_the_host`: 3 of 3 green on the branch.
Full lib suite green; clippy `-D warnings` 0; fmt clean — counts in `gates`.
