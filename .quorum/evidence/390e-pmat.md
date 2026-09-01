# pmat MCP lane — #390-E

`analyze_vacuous_tests` over the whole repo:

- tests_examined: **17797** (up 6 from the 1.24.0 baseline of 17791 — exactly the
  six tests added by this branch)
- files_parsed: 1874
- unmeasured_tests: 0

`tests/falsification_390e_nested_shell_strictness.rs` does **not** appear in the
vacuous list. Neither do the two updated files
(`src/core/codegen/tests_sudo.rs`, `src/resources/tests_task.rs`).

The repo-wide vacuous population is unchanged from 1.24.0 and remains tracked
separately — this branch neither adds to it nor is asked to fix it.

Falsification check (the rule with teeth): reverting both generator hunks to their
pre-fix shape turns **4 of 6** new tests red. The two that stay green are the
deliberate controls — `the_same_command_without_timeout_already_failed` (asserts
the un-wrapped path was ALREADY strict, which is the premise) and
`the_delimiter_is_deterministic` (holds either way by construction).
