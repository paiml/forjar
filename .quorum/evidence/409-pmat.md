# pmat MCP lane — #409 / #410

`analyze_vacuous_tests`: counts in the receipt; the E06 falsifier asserts
codegen equality and a parsed composite from the shipped command; the E07
falsifiers assert the refusal text by name and the plan's commands.

Falsification, tests kept: with `src/core/store/{purity,repro_score}.rs`
at the merge-base, `two_byte_identical_apply_scripts_score_equal` and
`test_e06_store_flag_does_not_affect_score` are RED (68 vs 38; Pure vs
Pinned); with `sandbox_run.rs` and `sandbox_exec.rs` at the merge-base,
`test_e07_execute_sandbox_plan_returns_honest_error` and
`the_plan_names_the_steps_it_cannot_run` are RED. Counts in the receipt.

Gates: clippy `-D warnings`, fmt, `cargo test --lib`, the three derivation
suites — counts in the receipt. Merged `origin/main` (baseline: theirs);
the lane's worktree had been created on an older base (f0cbf635), so the
merge is what makes the footprint honest (0 deletions vs main).
