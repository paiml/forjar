# PMAT-996 (`fix/check-secrets-provider`) — Judges Claims Digest

### CONFIRMED

1. [code] `forjar check` resolves `{{secrets.*}}` via the manifest `secrets:` provider (`C1`)
   - evidence: `src/cli/check.rs:395` switches `cmd_check_selected` from env-only `resolve_resource_templates` to `resolve_resource_templates_with_secrets(resource, &config.params, &config.machines, &config.secrets)`, verified by `src/cli/tests_check_secrets.rs:10` and `CHANGELOG.md:10`.

2. [security] Failing check diagnostics redact resolved secret values and withhold sensitive/ciphertext resources (`C2`)
   - evidence: `src/cli/check_run.rs:18` builds `Redaction::for_resource` and `src/cli/check_run.rs:89` applies `redaction.apply(&detail)` prior to printing text output or serializing `--json` rows, masking secret values as `***` and replacing `sensitive: true` or ciphertext failure details with `[withheld: sensitive resource]`, pinned by `src/cli/tests_check_secrets.rs:60`.

3. [code] Lockless check and `--refresh` check share the manifest `secrets:` provider path (`C3`)
   - evidence: `src/core/executor/lockless_check.rs:45` and `src/core/executor/refresh.rs:75` pass `&cfg.config.secrets` into `resolve_resource_templates_with_secrets`, verified by `src/core/executor/tests_lockless_secrets.rs:10` (`2/2`) and `src/core/executor/tests_refresh_secrets.rs:10` (`2/2`).

4. [test] End-to-end binary falsifiers prove `provider: file` resolution and output redaction with dual revert kills (`C4`)
   - evidence: `tests/falsification_check_secrets_provider.rs:70` and `tests/falsification_check_secrets_provider.rs:87` execute `CARGO_BIN_EXE_forjar` against a `provider: file` fixture across default and `--json` flags; reverting `src/cli/check.rs:395` fails both tests (`rc=101`, `secret not found`) and reverting `src/cli/check_run.rs:89` fails `falsify_996_failing_check_does_not_print_the_secret_it_echoed` (`rc=101`, `secret leaked`).

### REFUTED

1. [scope] External `sops` and `op` CLI backends are not executed against live vaults in clean-room CI (`R1`)
   - evidence: `docs/audits/impl-PMAT-996-receipt.md:1` and `docs/audits/impl-PMAT-996-receipt.md:45` record that neither `sops` age keys nor `op` service-account credentials exist in hermetic tests, while `tests/falsification_check_secrets_provider.rs:38` exercises `provider: file`.
   - corrected: `docs/audits/impl-PMAT-996-receipt.md:3` explicitly records `verdict: PARTIAL` rather than claiming live `sops`/`op` vault transcripts, while `src/cli/check.rs:395` and `CHANGELOG.md:10` route all three check call-sites through the single `SecretsConfig` dispatcher.

2. [security] Value-level `***` scrubbing covers `{{secrets.*}}` keys referenced on the checked resource, not arbitrary undeclared host strings (`R2`)
   - evidence: `src/cli/check_run.rs:18` harvests secret values only for `{{secrets.<name>}}` templates present in the resource fields before `src/cli/check_run.rs:89` scrubs `detail`.
   - corrected: `CHANGELOG.md:14` states accurately that `"the value of every secret the resource names is printed as ***"`, and resources reading secret files indirectly can set `sensitive: true` to force full `[withheld: sensitive resource]` suppression (`src/cli/check_run.rs:28`).
