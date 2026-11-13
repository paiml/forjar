# Claims — PMAT-912 / forjar#961

Each claim is stated only with the test that pins it.

- **C1**: an empty state dir (no `state.lock.yaml` for any machine) takes the lockless path, so a declared pin is compared; a state dir holding another stack's locks is not empty and a never-applied machine is not an error. Pinned by `src/cli/tests_drift_empty_state.rs` and `falsification_drift_scopes_to_the_config_it_was_given` (its fixture creates the state dir that main's GH-880 requires; removing the `!holds_no_lock` arm fails `a_declared_machine_with_no_state_yet_is_not_an_error`).
- **C2**: `plan_version_pins::measure` reports DRIFTED with the DOWNGRADE detail, discloses the offline skipped leg, and reads NOT CHECKED for an undeclared machine. Pinned by the three tests in `src/cli/plan_version_pins.rs`. The `forjar plan` CLI wiring is not pinned.
- **C3**: the package check script includes the presence assertions and `completion_check` for apt, cargo, uv and brew, and a blank or whitespace `completion_check` adds no assertion. Pinned by `gh694_completion_check_reaches_the_check_script` and `gh694_blank_completion_check_adds_no_assertion`.
- **C4**: `verify_against_host` asks the same check script. Pinned by `a_failing_package_completion_check_fails_the_post_apply_verification`.
