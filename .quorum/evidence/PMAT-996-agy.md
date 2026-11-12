# PMAT-996 (`fix/check-secrets-provider`) — Independent AGY Stack Review (`Gemini 4 Carbon`, `LANES=3`)

- **Reviewer Stack**: Antigravity (`Gemini 4 Carbon`, `thinking=high`), 3 independent lanes (`CLI Check & Redaction Security`, `Lockless & Refresh Executor Parity`, `End-to-End Binary Falsifiers, Revert Kills & Receipt Audit`), completed across 2 rounds (`FAIL @35ea039`, `PASS @2186e37ada1ec4d02b3181f95e83af30d43636a9`).
- **Execution Host**: `yoga` (`<SCRATCH>/agy-fj999`, `load1=0.92`, `nproc=22`, `/var/tmp` free `377 GiB`).
- **Commands & Results (`@2186e37ada1ec4d02b3181f95e83af30d43636a9`, base `4a73376b450aae881151a3f87abd103b186ffc19`)**:
  1. `cargo fmt --all -- --check` and `cargo clippy --locked --lib --bins --test falsification_check_secrets_provider -- -D warnings` on `yoga` -> `PASS` (`0` warnings).
  2. `cargo test --locked --lib -- tests_check_secrets tests_lockless_secrets tests_refresh_secrets` (`9/9`) and `cargo test --locked --test falsification_check_secrets_provider` (`2/2`) on `yoga` -> `PASS`.
  3. **Revert Kill 1 (Provider Wiring)**: Reverting `resolve_resource_templates_with_secrets` to `resolve_resource_templates` in `src/cli/check.rs:408` turns `cargo test --locked --test falsification_check_secrets_provider` RED (`rc=101`, `2 failed`, `secret not found`).
  4. **Revert Kill 2 (Redaction Wiring)**: Removing `redaction.apply(&detail)` in `src/cli/check_run.rs:89` turns `falsify_996_failing_check_does_not_print_the_secret_it_echoed` RED (`rc=101`, `secret leaked` in both text and `--json`).
