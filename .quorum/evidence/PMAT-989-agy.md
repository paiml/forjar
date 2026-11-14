# PMAT-989 (`feat/989-image-base-and-exec`) — Independent AGY Stack Review (`Gemini 4 Carbon`, `LANES=3`)

- **Reviewer Stack**: Antigravity (`Gemini 4 Carbon`, `thinking=high`), 3 independent lanes (`Security & Base-Ref Input Validation`, `Parser Parity & Deterministic OCI Layer Capture`, `Contracts, Documentation & Tree Hygiene`), completed across 3 rounds (`FAIL @eef2c36`, `FAIL @2e61251`, `PASS @fa26fe210920448ff577b1b51da6711eb2fd77b2`).
- **Execution Hosts**: `kfocus` (`/var/tmp/agy-forjar-989r2`, `load1=6.84`, `nproc=32`, `/var/tmp` free `229.9 GiB`) and `gx10` (`RED` revert check `2 failed`, `lib` `13850 passed`).
- **Commands & Results (@ `fa26fe210920448ff577b1b51da6711eb2fd77b2`, code tree identical to `2e612515` minus removed `.orig` backup)**:
  1. `cargo fmt --all -- --check` and `cargo clippy --lib --test falsification_image_exec --test falsification_no_orphaned_source_files --features container-test -- -D warnings` on `kfocus` -> `PASS` (`0` warnings).
  2. `cargo test --lib -- tests_validation_image tests_build_image_base tests_build_image_exec tests_exec_layer` and `cargo test --test falsification_no_orphaned_source_files` (`3/3`) on `kfocus` -> `PASS`.
  3. `cargo test --features container-test --test falsification_image_exec -- --test-threads=1` on `kfocus` (`f1_the_same_spec_built_twice_has_the_same_manifest_digest` + `f2_the_declared_package_and_command_output_are_in_the_built_image`) -> `PASS` (`2/2` in `47.26s`).
  4. Revert perturbation of Round-1 fixes (`check_base_ref` in `src/cli/build_image_base.rs:24` and `validate_file` in `src/core/parser/resource_types.rs:85`) -> **FAILED** (`2 failed` on `gx10`, proving strict discrimination).
- **Overall Verdict**: `PASS @fa26fe210920448ff577b1b51da6711eb2fd77b2` (3/3 lanes PASS).


Round 4 delta review at `fa26fe21` (merge of `origin/main` `4a73376b` into `2be85af3`): `CHANGELOG.md:10` keeps both sides cleanly with zero source or test changes; 3/3 PASS.
