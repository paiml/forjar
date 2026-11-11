# PMAT-660 (`forjar#934`) — Judges Claims Digest

### CONFIRMED

1. [code] Pre-template data-source resolution in cmd_show (`C1`)
   - evidence: `src/cli/show.rs:36` invokes `resolver::resolve_data_sources(&mut config)?;` right after `parse_and_validate(file)?` and before `resolver::resolve_resource_templates`, populating `config.params` so `{{data.*}}` templates resolve in both YAML and `--json` output (`src/cli/tests_cov_show2.rs:175`).

2. [code] Fail-closed error propagation on broken data sources (`C2`)
   - evidence: `src/cli/show.rs:36` propagates `resolve_data_sources` errors via `?` prior to any `println!`, and `src/cli/tests_cov_show2.rs:182` (`show_reports_a_failing_data_source`) verifies non-zero `type: command` data sources return `Err` naming the failing key.

3. [test] Revert-killing unit test coverage in tests_cov_show2 (`C3`)
   - evidence: `src/cli/tests_cov_show2.rs:175` exercises both unfiltered YAML and `-r greeting --json` paths; restoring `src/cli/show.rs` from `main` on `t2fw13` failed `show_resolves_data_sources` immediately (`FJ934_REV_KILLED`).

### REFUTED

1. [doc] Receipt claim that show resolves every manifest plan resolves (`R1`)
   - evidence: `docs/audits/impl-PMAT-660-receipt.md` line `21` states `"every manifest that plan resolves, show resolves too"`, whereas `cmd_show` (`src/cli/show.rs:30`) still calls `parse_and_validate` directly without `prepare_config` (`--workspace` and `--env-file`).
   - corrected: `forjar show` now matches `validate` and `plan` on `data:` source resolution (`#660`), while `--workspace` (`{{params.workspace}}`) and `--env-file` parameter injection remain explicitly outside `cmd_show`'s CLI signature (`impl-PMAT-660-receipt.md` line `41`).

2. [test] Stdout value assertion granularity in show_resolves_data_sources (`R2`)
   - evidence: `src/cli/tests_cov_show2.rs:175` checks `.is_ok()` on `cmd_show` (which prints to stdout via `println!`) rather than capturing stdout to assert `"hello-from-data"` appears in the rendered text.
   - corrected: Template value substitution from `resolve_data_sources` is already pinned by `core::resolver::tests_data` (`9` unit tests), while `show_resolves_data_sources` cleanly falsifies removal of the `resolve_data_sources` call (`FJ934_REV_KILLED`).
