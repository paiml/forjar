# pmat lane — PMAT-161 — analyze_vacuous_tests

Tool: `pmat 3.39.0`, `pmat analyze vacuous-tests -f json`, run over the branch at HEAD and filtered to the touched test paths.

## Result

```text
tests_examined    = 19407
files_parsed      = 2120
vacuous (repo)    = 360
conditional_skips = 4
in touched paths  = 4
```

Touched test paths:

- src/cli/tests_cov_apply_b.rs
- src/cli/tests_generation.rs
- src/cli/tests_helpers_state_b.rs
- src/core/state/stamp/tests_file_identity.rs
- src/core/state/tests_basic.rs
- src/core/state/tests_edge.rs
- src/core/state/tests_global_lock.rs
- src/core/state/tests_outputs.rs
- src/core/state/tests_stack_stamp.rs
- src/core/state/tests_state_cov.rs
- src/core/types/tests_proptest_resource.rs
- src/core/types/tests_state.rs
- tests/common/stack_stamp_harness.rs
- tests/common/stamp_gc_cases.rs
- tests/common/stamp_rename_cases.rs
- tests/common/stamp_wrong_file_cases.rs
- tests/falsification_state_integrity.rs
- tests/falsification_state_stamp_per_name.rs
- tests/falsification_undo_state_dir_interlock.rs

## Hits in touched paths

- src/cli/tests_cov_apply_b.rs:341 `test_print_resource_report_empty` (no-failure-mode)
- src/cli/tests_cov_apply_b.rs:347 `test_print_resource_report_mixed` (no-failure-mode)
- src/cli/tests_cov_apply_b.rs:362 `test_print_timing_basic` (no-failure-mode)
- src/cli/tests_cov_apply_b.rs:371 `test_print_timing_zero_durations` (no-failure-mode)

## Limit

The count is what the tool measured. The orchestrator measured discrimination separately: at 5044ab1f three mutations of the guard turned 4, 1 and 1 tests red, and at HEAD the disabled threshold turns the tests named in the receipt's falsification block red; the two review-driven suites (rename cases, wrong-file cases, gc cases, replay fidelity) were each committed RED before their fix.
