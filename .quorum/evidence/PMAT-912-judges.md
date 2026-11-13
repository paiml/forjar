# Judges — PMAT-912 / forjar#961

Records of past review rounds. Round 1 at `b23462d4` was 0/3 PASS; a later round was 3/3 PASS (see `PMAT-912-lanes.md`). Later heads were re-reviewed separately.

- C1 (empty state dir lockless): measured on a build host, `tests_drift_empty_state` 4/4 and the 8 `falsification_drift_*` targets 42/42 passed; perturbing `holds_no_lock` failed 3 of 4 tests.
- C2 (plan version pins): `cargo test --lib plan_version_pins` passed.
- C3 (completion_check): RED commit `4ce10476` failed `gh694_blank_completion_check_adds_no_assertion`; it passes from `c22b5be0`.
- Round-1 finding R1: at `b23462d4`, `falsification_drift_scopes_to_the_config_it_was_given` was RED; fixed by `holds_no_lock` plus `f852_state_dir_with_another_stacks_lock_is_not_lockless`.
- Round-1 finding R2: a blank `completion_check` was reported as an empty `if` condition; fixed by the trim guard plus `gh694_blank_completion_check_adds_no_assertion`.
