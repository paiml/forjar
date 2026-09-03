# pmat MCP lane — #360 / #362

`analyze_vacuous_tests`: counts in the receipt (run on the rebuilt branch).

Falsification, tests kept: with `src/core/executor/resource_ops.rs`,
`src/tripwire/drift/mod.rs`, `src/cli/apply_variants.rs` and
`src/core/observation_mask.rs` at the merge-base the mask cases are RED
(the whole observation is suppressed, or drift is reported on the ignored
field); with `src/resources/cron.rs` at the merge-base the cron exec cases
are RED (the old job survives the re-apply; the sibling is orphaned).
Counts in the receipt.

Gates: clippy `-D warnings`, fmt, `cargo test --lib` — counts in the
receipt. Rebuilt from the holding commit's own diff on main after #412
(the branch touches the executor's record path, which #412 moved).
