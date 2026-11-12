# Judges — Empty-state lockless drift, plan version-pin tests & package `completion_check` (PMAT-912 / forjar#961)

Round count and heads: Round 1 `b23462d4` (`0/3 PASS`), Round 2 `a7c9ce1c` (`3/3 PASS`); see `PMAT-912-lanes.md`. Every claim `C1..C4` and `R1..R2` met 3 independent refuters.

## CONFIRMED

1. [code] C1 — `src/cli/drift_state.rs:75` (`holds_no_lock`) and `src/cli/drift.rs:236` treat an existing empty `state_dir` as lockless (`scan_lockless`) only when zero `state.lock.yaml` files exist in `state_dir`, keeping multi-stack scoping intact.
   - evidence: `src/cli/tests_drift_empty_state.rs:25` (`4/4`) and all `8/8` `falsification_drift_*` targets (`42/42`) passed on `<host>`, and perturbing `holds_no_lock` failed `3/4` tests (`FJ_961_REVERT_RC=101`).
2. [test] C2 — `src/cli/plan_version_pins.rs:60` exercises `forjar plan` live version-pin mismatch (`DRIFTED` / `DOWNGRADE`), offline upstream leg disclosure, and undeclared machine `NOT CHECKED` reporting (`forjar#613`).
   - evidence: `cargo test --lib plan_version_pins` passed (`rc=0`) on `<host>` with `cargo clippy --all-targets -- -D warnings` clean (`rc=0`).
3. [code] C3 — `src/resources/package_check.rs:199` (`finish`) appends non-blank `completion_check` assertions across every package provider (`apt`, `cargo`, `uv`, `brew`) while ignoring empty/whitespace strings.
   - evidence: `src/resources/tests_package_d.rs:90` (`gh694_*` suite) passed on `<host>` and RED commit `4ce10476` failed `gh694_blank_completion_check_adds_no_assertion` (`rc=101`) prior to `c22b5be0`.
4. [doc] C4 — `CHANGELOG.md:8` and `docs/audits/impl-PMAT-912-receipt.md:15` record `#852`, `#613`, `#694`, the Round-1 scoping regression, and remaining multi-machine follow-ups.
   - evidence: Verified on `<host>` that all Gate A documentation files (`CHANGELOG.md:8`, `docs/roadmaps/roadmap.yaml:7110`, `docs/audits/impl-PMAT-912-receipt.md:15`) are present and accurate.

## REFUTED

1. [code] R1 — Round-1 `src/cli/drift.rs:236` routed every empty `collect_machine_locks` result into `scan_lockless`, causing unapplied configs sharing a `state_dir` with another stack to fail with `declined` (`exit 1`).
   - corrected: Commit `c22b5be0` added `holds_no_lock(state_dir)` in `src/cli/drift_state.rs:75` and `f852_state_dir_with_another_stacks_lock_is_not_lockless` in `src/cli/tests_drift_empty_state.rs:25`, restoring all `42/42` drift integration tests.
2. [code] R2 — Round-1 `src/resources/package_check.rs:199` appended whitespace-only `completion_check` values without trimming, emitting invalid POSIX `sh` syntax in generated check scripts.
   - corrected: Commit `c22b5be0` guarded `completion_check` on `!cmd.trim().is_empty()`, verified by `gh694_blank_completion_check_adds_no_assertion` in `src/resources/tests_package_d.rs:90`.
