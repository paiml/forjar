# pmat MCP lane — #364

`analyze_vacuous_tests`: counts in the receipt; the falsifier's two cases
assert on parsed lockfile entries and on the workflow text, neither of
which is a constant.

Falsification, tests kept: with `Cargo.lock` at the merge-base,
`cargo_lock_pins_no_known_yanked_release` is RED (spin 0.9.8 named); with
`.github/workflows/audit.yml` at the merge-base,
`audit_workflow_denies_yanked_crates` is RED (flag absent).

Gates: lockfile resolves `--locked`; clippy/fmt untouched by this diff
(no Rust source changes outside the new test).
