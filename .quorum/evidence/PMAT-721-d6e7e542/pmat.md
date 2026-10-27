# pmat vacuous-test result for epic/forjar-697-v1-35-0-lookahead

Command: `pmat analyze vacuous-tests -p . -f json` (examined 20,020 tests in 2,244 files;
432 flags repo-wide). Intersected with the 134 .rs files this branch touches against base
493ccc36:

- Test files the branch wrote (e.g. tests/falsification_plan_file_lockless_record_in_scope.rs): 0 findings.
- Pre-existing files with flags, all kind no-failure-mode, 72 in total:
  src/cli/tests_cov_args_extra_b.rs 41 (branch diff +1 line), src/cli/tests_cov_args_extra_c.rs 25 (+1),
  src/cli/tests_cov_apply_b.rs 4 (4 lines changed), src/cli/tests_show_1.rs 1 (1 line changed),
  src/cli/tests_diff_cmd.rs 1 (1 line changed).
- Every edit in those five files is a mechanical follow of a signature change (an added
  `offline: false` field, an added argument, `exclude: None` to `exclude: vec![]`). None of the
  flagged tests was written by this branch. They are recorded as accepted, not fixed.
