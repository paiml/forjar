# Implementation receipt — PMAT-996 — `apply --check` resolves secrets through the manifest provider

verdict: PARTIAL — the check, the lockless check and the `--refresh` check now resolve `{{secrets.*}}` through the manifest `secrets:` block; other read-only callers of the env-only resolver are listed below and unchanged.

## Identity

- ticket PMAT-996 (forjar#996; reported as `forjar apply --check` 1.34.0 failing with "secret not found" on a manifest whose `apply` works); kind: code
- branch `fix/check-secrets-provider`, base `main` @ c4ce3a2c
- files: `src/cli/check.rs`, `src/cli/check_run.rs`, `src/cli/mod.rs`, `src/core/executor/lockless_check.rs`, `src/core/executor/refresh.rs`, `src/cli/tests_check_secrets.rs`, `src/core/executor/tests_lockless_secrets.rs`, `src/core/executor/tests_refresh_secrets.rs`, `src/cli/mod_test_decl.rs`, `CHANGELOG.md`, `docs/roadmaps/roadmap.yaml`

## Root cause (five whys)

1. `apply --check` failed with `secret '<name>' not found (set env var FORJAR_SECRET_<NAME> or use a secrets file)` on a manifest that declares a secrets provider.
2. The resolver looked the secret up in the environment, not in the declared provider.
3. `cmd_check` called `resolve_resource_templates(resource, params, machines)`, which resolves with `SecretsConfig::default()` (provider `env`).
4. The secrets-aware entry point `resolve_resource_templates_with_secrets` was wired into the apply path only; the check paths kept the old call.
5. No test ran a check over a manifest with a non-env provider, so the gap was silent. The same call sat in `host_check_passes` (lockless check) and `refresh_check_fails` (`--refresh`), where the failed resolution was read as "could not look".

## Contract and falsifier

Contract: every check that runs a resource's check script resolves its templates with the manifest's `secrets:` block, the same one `apply` uses. A check never falls back to `FORJAR_SECRET_*` when the manifest names another provider.

Falsifiers (each runs a real check on localhost with `provider: file` and a key no host variable carries):

- `cli::tests_check_secrets::check_resolves_a_secret_through_the_manifest_file_provider`
- `cli::tests_check_secrets::check_fails_on_the_resolved_value_not_on_a_missing_env_var`
- `core::executor::lockless_check::tests_lockless_secrets::lockless_check_reads_the_secret_from_the_declared_provider`
- `core::executor::refresh::tests_refresh_secrets::refresh_check_observes_divergence_behind_a_provider_secret`
- controls, green before and after: `lockless_check_still_fails_when_the_resolved_value_differs`, `refresh_check_passes_when_the_provider_secret_matches`

## Evidence (2026-11-12)

RED, the test files on unchanged `main` code (commit e682a19b), host gx10, `cargo test --lib -- <module>` for each of the three modules:

    tests_check_secrets:    0 passed; 2 failed
    tests_lockless_secrets: 1 passed; 1 failed
    tests_refresh_secrets:  1 passed; 1 failed

GREEN, with the fix (commit 9240fe36), host t2yoga:

    cargo fmt --all -- --check                      rc=0
    cargo clippy --all-targets -- -D warnings       rc=0
    tests_check_secrets:    2 passed; 0 failed
    tests_lockless_secrets: 2 passed; 0 failed
    tests_refresh_secrets:  2 passed; 0 failed
    cargo test --lib        13817 passed; 0 failed; 4 ignored

Not measured: the reporter's own manifest and provider (the tests use `provider: file`); the `sops` and `op` providers under a check; integration test targets under `tests/`; the released binary.

## Open

- Other callers of the env-only `resolve_resource_templates` are unchanged and may show the same defect for their verb: `src/cli/check_test_runners.rs`, `src/cli/validate_deep.rs`, `src/cli/validate_core.rs`, `src/cli/coverage_promote.rs`, `src/cli/show.rs`, `src/cli/build_image.rs`, `src/cli/contracts.rs`, `src/mcp/handlers.rs`. Each needs its own decision: some of these verbs should not run a subprocess provider at all.
- A check now runs the declared provider, including `sops` and `op` subprocesses. The unattended surface already swaps those for a declining provider on the config it is handed; not re-measured here for the check verb.

## Review round 1 (FAIL @717244c, three lanes) and what changed

- `src/cli/check.rs` had grown 513 -> 517 lines, past the 500-line growth ratchet of the `file-health` job. The single-check runner moved to the new `src/cli/check_run.rs` (113 lines); `check.rs` is now 462 lines (`wc -l`).
- A check script now carries resolved secret values, and a failing script could echo them into the detail that text mode prints and `--json` serializes. `check_run::Redaction` takes the policy `apply` uses for transcripts from the UNRESOLVED declaration: named secret values are struck out of the detail; a `sensitive` resource, or one carrying ciphertext, has its detail withheld whole.
- New tests in `cli::tests_check_secrets`: `a_failing_check_does_not_keep_the_secret_it_echoed`, `a_sensitive_resource_has_its_check_output_withheld`, and `check_of_a_file_resource_with_secret_content_uses_the_provider` (the reported `file` resource shape; it asserts only that a failure is not the missing-env-var error).

Measured on host gx10, round 2:

    redaction line reversed, cargo test --lib -- tests_check_secrets:  3 passed; 2 failed (the two redaction tests)
    restored:
    cargo fmt --all -- --check                      rc=0
    cargo clippy --all-targets -- -D warnings       rc=0
    tests_check_secrets:    5 passed; 0 failed
    tests_lockless_secrets: 2 passed; 0 failed
    tests_refresh_secrets:  2 passed; 0 failed
    cargo test --lib        13820 passed; 0 failed; 4 ignored

Still not measured: `apply --check` through its own argument struct (it calls the same `cmd_check_selected`), and the `file-health` job itself (no PR exists yet, so CI has not run).

IMPL-PMAT-996-RECEIPT-END
