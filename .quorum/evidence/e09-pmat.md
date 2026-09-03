# pmat MCP lane — #412

`analyze_vacuous_tests`: counts recorded in the receipt after the last commit;
the two parity binaries compare artefacts to each other after scrubbing only
run ids and timestamps, and the harness (`tests/common/scheduler_parity.rs`)
is shared by `#[path]` so both binaries read the same fixtures.

Falsification, tests kept: `src/core/executor` reverted to main → the first
binary is **0 passed, 4 failed** (hooks twice, lock differs, events differ,
failure attributed to the wrong resource). A per-hunk revert table was NOT
produced within the worker's turn budget and is recorded as a gap.

Gates: falsifiers 4 + 4 green; `cargo test --lib`, clippy and fmt counts in
the receipt's `gates`. The TDG hook: `machine_b.rs` measures 94.0 (A) against
a 95.7 (A+) baseline whose 5.0 entropy the recompute omits (pmat #1162); its
cognitive complexity fell (27 → 17) and the baseline was re-recorded with the
hook's own update before the commit [A].
