# pmat lane — PMAT-160 — analyze_vacuous_tests

Tool: `pmat 3.37.0`, `pmat analyze vacuous-tests -f json`, run over the repository (the tool refuses a single path — no denominator) and filtered to the seven touched test paths.

## Result

```text
tests_examined    = 19347
files_parsed      = 2106
vacuous (repo)    = 360
conditional_skips = 4
in touched paths  = 4
```

Touched test paths:

- src/cli/tests_apply_scope.rs
- src/cli/tests_apply_selection_closure.rs
- src/cli/tests_check_cov.rs
- src/cli/tests_cov_apply_b.rs
- src/cli/tests_cov_dispatch_2.rs
- src/cli/tests_gh_dogfood_p1.rs
- tests/falsification_apply_filter_pipeline.rs

## The four hits, all pre-existing

- src/cli/tests_cov_apply_b.rs:332 `test_print_resource_report_empty` (no-failure-mode)
- src/cli/tests_cov_apply_b.rs:338 `test_print_resource_report_mixed` (no-failure-mode)
- src/cli/tests_cov_apply_b.rs:353 `test_print_timing_basic` (no-failure-mode)
- src/cli/tests_cov_apply_b.rs:362 `test_print_timing_zero_durations` (no-failure-mode)

All four are print-helper smoke tests in `src/cli/tests_cov_apply_b.rs` that exercise `print_resource_report` / `print_timing` with no assertion; they sit at lines 332-362, far from the nine lines the branch changed in that file (5b20cb41, the scoped dry-run test), and they predate the branch. They are named in the receipt's `pmat.accepted` with this reason rather than fixed here, because capturing stdout to assert on it is a separate change.

## Limit

The zero-for-the-branch is what the tool measured, not a claim that every touched test can fail. The orchestrator measured that separately: four mutations of the fix (no closure; validation after the prune; `-r`/`-g` re-applied downstream; the `--check` branch skipping the resolver) turn 6, 1, 1 and 5 tests red respectively, and the two tests added in 3fdae0c3 are red with their hunks reverted (`filter-pipeline-claims.md` F2; the receipt's `falsification` block).
