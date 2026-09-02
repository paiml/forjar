# pmat MCP lane — #366 / #369

`analyze_vacuous_tests`: 17818 tests examined; 0 vacuous tests in any file
this branch touches (`src/core/parser/policy.rs`, `policy_coverage/*`,
`remediate/mod.rs`, `types/policy_rule_types.rs`, `compliance_gate.rs`,
`src/cli/infra_query.rs`, the four falsifier binaries).

Falsification, tests kept: the branch's own record — every hunk goes red
on its own when reverted; `falsification_policy_rule_identity` 5 of 7 RED
against unfixed code, library and shipped binary. Re-run for the receipt
with `src/core/parser/policy.rs`, `src/core/types/policy_rule_types.rs`,
`src/core/policy_coverage/mod.rs` and `src/cli/infra_query.rs` at the
merge-base.

Gates: the branch's own record `cargo test --lib 13404 passed; the four
policy suites 21 passed; fmt clean`; re-run counts in the receipt. Merged
`origin/main` cleanly so the receipt is anchored at 9e0815c0.
`policy_rule_types.rs` crossed the 500-line cap and its test module moved
to a `#[path]` sibling.
