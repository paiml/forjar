# pmat MCP lane — #390-C

`analyze_vacuous_tests`: tests/falsification_390c_json_failure_detail.rs does not
appear in the vacuous list. Repo-wide population unchanged from the 1.24.0 baseline.

Falsification, run TWICE because the first run exposed the suite rather than the fix:

- **First attempt: only 1 of 3 red.** Two tests were vacuous — a permissive `||`
  disjunction, and an `if let` that swallowed a shape mismatch. Both searched a
  top-level `resource_reports` that does not exist.
- **After correction: 3 of 3 red** against reverted code.

Full suite: 13404 passed, 0 failed. clippy `-D warnings`: 0 errors.
