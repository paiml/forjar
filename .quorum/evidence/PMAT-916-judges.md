# Judges — ResourceType Display spelling & cron block guards (PMAT-916 / forjar#943)

Round count and heads: Round 1 `a4e28c3b` (`0/3 PASS`), Round 2 `f7525fb6` (`3/3 PASS`); see `PMAT-916-lanes.md`. Every claim `C1..C4` and `R1..R2` met 3 independent refuters.

## CONFIRMED

1. [code] C1 — `src/cli/validate_advanced.rs:120` and 41 other CLI diagnostic modules format `ResourceType` via `Display` (`snake_case`) while `src/cli/lineage.rs:168` and `src/cli/provenance.rs:199` keep `Debug` (`{:?}`) for BLAKE3 digest stability.
   - evidence: `tests/resource_type_spelling.rs:40` (`5/5`) and `tests/resource_type_cli_output.rs:30` (`10/10`) passed on `<host>`, and reverting `src/cli/validate_advanced.rs` to `origin/main` failed `cli_never_debug_formats_resource_type` (`FJ_943_REVERT_RC=101`).
2. [code] C2 — `src/resources/cron.rs:37` (`crontab_check_guard`) distinguishes non-interactive `sudo -n` absence or refusal (`FORJAR_SKIP`) from a genuinely absent user crontab (`missing:<name>`).
   - evidence: `src/resources/tests_cron_nosudo.rs:20` passed on `<host>`, confirming unprivileged `check` runs do not misreport `sudo` permission denials as missing cron resources.
3. [code] C3 — `src/resources/cron.rs:51` (`intact_block_guard`) rejects truncated 3-line `# forjar:<name>` blocks (`exit 3`, aborting `apply` before `delete_block` runs) when the entry line is EOF, blank, commented, or a sibling marker.
   - evidence: `src/resources/tests_cron_exec.rs:400` (`fj445`, `10/10` passed on `<host>`) verified both mid-crontab truncated blocks and intact schedule/env cron entries.
4. [doc] C4 — `CHANGELOG.md:8`, `docs/roadmaps/roadmap.yaml:7110`, and `docs/audits/impl-PMAT-916-receipt.md:20` document `#433`, `#848`, and `#445` including the residual two-adjacent-bare-lines boundary.
   - evidence: `cargo fmt --all -- --check` (`rc=0`) and `cargo clippy --all-targets -- -D warnings` (`rc=0`) passed cleanly on `<host>`.

## REFUTED

1. [code] R1 — Round-1 `intact_block_guard` in `src/resources/cron.rs:62` used `d==2{d=0;next}`, accepting a deleted 3rd entry line whenever followed by `# forjar:<other>`, `# comment`, or blank line and causing `delete_block` (`src/resources/cron.rs:95`) to delete that line.
   - corrected: Commit `81fcf1ee` tightened `d==2` to flag `^[ 	]*(#|$)` as `bad=1` (`exit 3`), with regression tests in `src/resources/tests_cron_exec.rs:400` verified green (`10/10`) on `<host>`.
2. [code] R2 — Round-1 `src/cli/validate_deep.rs:106` and `src/cli/validate_quality.rs:16` replaced `format!("{:?}", ...)` with `res.resource_type.to_string() == "Unknown"`, creating dead branches on a closed `snake_case` enum and bumping `CB-200` to `652 > 651`.
   - corrected: Round 2 (`f7525fb6`) removed both dead loops (`serde_yaml` already rejects unknown `type:` variants at parse time) and restored `CB-200` compliance.
