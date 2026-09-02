# pmat MCP lane — #403

`analyze_vacuous_tests` over the whole worktree: 17825 tests examined; ZERO
findings in any path this branch touches (`src/core/planner/hashing.rs`,
`src/core/planner/tests_hash_completeness.rs`, `src/core/planner/tests_hash.rs`,
`src/core/observe/classify_e01.rs`,
`tests/falsification_e01_hash_the_whole_resource.rs`).

Falsification, run against the reverted production hunk:

- `hashing.rs` restored to main: **8 of 10 RED** (every `*_moves_the_hash`
  case), 2 green by design (the `*_does_not_move_the_hash` guards hold on both
  trees and are the other half of the contract).
- tag length-prefix reverted in isolation: `tagged_input_values_do_not_collide`
  **RED**; restored: green. The FIRST version of that test stayed green on the
  reverted code because its counterexample was wrong — recorded as a refuted
  claim rather than quietly fixed.

Full lib suite: 13408 passed, 0 failed, 4 ignored. `cargo fmt --check` clean.
`cargo clippy --all-targets -D warnings`: 0 errors.
