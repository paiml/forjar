# pmat MCP lane — #390-A/B

`analyze_vacuous_tests`: tests/falsification_390ab_parallel_wave_parity.rs does not
appear in the vacuous list. Repo-wide population unchanged from the 1.24.0 baseline.

Falsification (the rule with teeth), run TWICE because the first run was wrong:

- **First attempt: 4/4 passed against fully-reverted code.** Not a fix verification —
  a vacuous suite. Cause: one-resource fixtures never reach the wave path
  (`machine.rs:183` needs >1 change).
- **After correction: 3 of 5 go red.** The 2 that stay green are deliberate guards
  (`the_wave_path_is_actually_taken`, `a_converging_task_still_converges_under_parallel`)
  which must hold either way.

Full suite: 13404 passed, 0 failed. clippy `-D warnings`: 0 errors.
