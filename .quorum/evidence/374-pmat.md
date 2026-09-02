# pmat MCP lane — #374

`analyze_vacuous_tests`: 17818 tests examined; 0 vacuous tests in any file
this branch touches (`src/cli/apply_variants.rs`, `dispatch_apply_b.rs`,
`tests_apply_variants.rs`, `tests_cov_apply.rs`, `tests_cov_apply_modes.rs`,
`tests_cov_fleet_c.rs`, `tests_cov_fleet_run.rs`,
`tests/falsification_canary_apply_is_authorized.rs`).

Falsification, tests kept: with `src/cli/dispatch_apply_b.rs` and
`src/cli/apply_variants.rs` reverted to the merge-base, the gate cases fail
on the first assertion of `refused()` (exit 0, fleet converged) and
`canary_apply_does_not_imply_yes` fails on `prod_file()` existing; the
control and the two `a_listed_operator_still_gets_*` cases stay green,
which is the regression guard doing its job.

Gates: clippy `-D warnings` 0; fmt clean; `cargo test --lib` counts in the
receipt. Merged `origin/main` (CHANGELOG conflict: both Unreleased blocks
kept) so the receipt is anchored at 9e0815c0.
