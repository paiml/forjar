# pmat MCP lane — #449

`analyze_vacuous_tests`: counts in the receipt; the three cases assert bytes
at a path and directory entries, never a summary line.

Falsification, tests kept: with `src/cli/destroy.rs` at the merge-base,
`destroy_records_a_generation` and `destroy_then_undo_restores_the_managed_file`
are RED and the control stays green (measured before the fix was written —
TDD order, two commits).

Gates: `cargo test --lib -- destroy undo generation` 280 passed; clippy
`-D warnings` 0; fmt clean; full lib count in the receipt.
