# PMAT-660 (`forjar#934`) — Extracted Claim Manifest (`C1..C3`, `R1..R2`)

Base commit: `5b8f8123be40731340e2a74a6e4ae365648cab45`
Head commit: `8b70633cd59aa07500fa017c239dbdbaa674b55f`
Branch: `reaper-w/forjar-660`

1. **C1 (`[code]`)**: `cmd_show` (`src/cli/show.rs:36`) calls `resolver::resolve_data_sources(&mut config)?` after `parse_and_validate` and before `resolve_resource_templates`, resolving `{{data.*}}` across YAML/JSON and `-r` filtered output.
2. **C2 (`[code]`)**: Failing `data:` sources propagate `Err` via `?` and abort `cmd_show` (`show_reports_a_failing_data_source`, `src/cli/tests_cov_show2.rs:182`).
3. **C3 (`[test]`)**: `show_resolves_data_sources` (`src/cli/tests_cov_show2.rs:175`) fails immediately when `src/cli/show.rs` is reverted to `main` (`FJ934_REV_KILLED` on `t2fw13`).
4. **R1 (`[doc]`)**: `docs/audits/impl-PMAT-660-receipt.md:21` claims every manifest `plan` resolves is also resolved by `show`, whereas `show` still omits `prepare_config` (`--workspace`, `--env-file`).
5. **R2 (`[test]`)**: `show_resolves_data_sources` asserts `.is_ok()` on `cmd_show` without capturing stdout to assert the rendered string directly.
