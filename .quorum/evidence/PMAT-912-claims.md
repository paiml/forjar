# Claims — Empty-state lockless drift, plan version-pin tests & package `completion_check` (PMAT-912 / forjar#961)

Extracted from PR `#961` (`cf/912-drift-train`, Round 1 `b23462d4` -> Round 2 `a7c9ce1c`) and attacked across 3 lanes:

- **C1 (Empty state dir is lockless, while multi-stack state dirs stay scoped)**: `src/cli/drift_state.rs:75` (`holds_no_lock`) and `src/cli/drift.rs:236` route `forjar drift` into `scan_lockless` when `state_dir` holds zero `state.lock.yaml` files anywhere, comparing declared pins (`#852`) while preserving `no state yet` beside another stack's locks (`src/cli/tests_drift_empty_state.rs:25`).
- **C2 (`forjar plan` version-pin mismatch, offline leg disclosure & undeclared machine notice)**: `src/cli/plan_version_pins.rs:60` verifies live version mismatch (`DRIFTED` / `DOWNGRADE`), offline upstream disclosure, and undeclared machine `NOT CHECKED` (`#613`).
- **C3 (`package` `check_script` runs non-blank `completion_check` across providers)**: `src/resources/package_check.rs:199` appends non-empty `completion_check` assertions across `apt`, `cargo`, `uv`, and `brew` (`src/resources/tests_package_d.rs:90`, `#694`).
- **C4 (Gate A receipt honesty & revert discrimination)**: `CHANGELOG.md:8` and `docs/audits/impl-PMAT-912-receipt.md:15` record the Round-1 regression, fix, and open follow-ups, and perturbing `holds_no_lock` fails `3/4` tests (`FJ_961_REVERT_RC=101`).
- **R1 (Round-1 `b23462d4` broke multi-stack scoping in `falsification_drift_scopes_to_the_config_it_was_given`)**: Routing all `locks.is_empty()` cases to `scan_lockless` turned never-applied configs into `declined` (`exit 1`) when `state_dir` held another stack's locks.
- **R2 (Round-1 `b23462d4` emitted invalid POSIX `sh` on whitespace-only `completion_check`)**: `finish` in `src/resources/package_check.rs:199` previously appended unguarded whitespace `completion_check` strings.
