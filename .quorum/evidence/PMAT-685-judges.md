# PMAT-685 & PMAT-848 (`forjar#903`) — Judges Claims Digest

### CONFIRMED

1. [code] Secret-placeholder apply codegen for lint and quality_gate (`C1`)
   - evidence: `src/core/resolver/resource.rs:315` and `src/core/codegen/dispatch.rs:350` replace `{{secrets.<key>}}` with synthetic `__FORJAR_SECRET_<key>__` tokens strictly on the read-only lint path (`src/cli/lint.rs:331` and `src/core/quality_gate/checks.rs:33`), keeping production `apply_script` fail-closed on unresolved secrets.

2. [code] Explicit reporting of ungenerated secret-bearing apply scripts in cli::lint (`C2`)
   - evidence: `src/cli/lint.rs:341` checks `Err(e) if kind == "apply" && resolver::has_unresolved_secret(resource)` and records `"bashrs: {id}/apply not linted: {e}"` instead of silently skipping ungenerated secret-bearing apply scripts, verified by `src/cli/tests_lint_secret.rs:45`.

3. [code] Command-scoped sudoers probe in crontab_check_guard (`C3`)
   - evidence: `src/resources/cron.rs:32` replaces `sudo -n true` with `LC_ALL=C sudo -n crontab -u {u} -l 2>&1 >/dev/null` and exits `2` (`FORJAR_SKIP`) only when stderr starts with `sudo:`, allowing `NOPASSWD: /usr/bin/crontab` hosts to observe real crontab state instead of falsely skipping.

4. [test] Hermetic revert-killing tests and zero-error bashrs output (`C4`)
   - evidence: `src/cli/tests_lint_secret.rs:15`, `src/core/quality_gate/tests_secret_apply.rs:15`, and `src/resources/tests_cron_exec.rs:385` pin both fixes hermetically; on `t2fw13`, `crontab_check_guard` produced `0` `bashrs` errors and both `FJ903_REV1_KILLED` and `FJ903_REV2_KILLED` failed on revert.

### REFUTED

1. [doc] Receipt overclaim that quality_gate reports ungenerated apply scripts (`R1`)
   - evidence: `docs/audits/impl-PMAT-685-receipt.md` line `3` asserts that both `forjar lint` and the quality gate report an apply script that still cannot be generated, whereas `src/core/quality_gate/checks.rs:33` retains `if let Ok(text) = result` and silently drops `Err(_)` results.
   - corrected: Ungenerated secret-bearing apply script reporting is implemented in `src/cli/lint.rs:341` (`forjar lint`), while `src/core/quality_gate/checks.rs:33` gains secret-placeholder linting for all dispatchable resource types without emitting a separate ungenerated-script diagnostic.

2. [doc] Comment in lint.rs:340 vs non-dispatchable secret-bearing resources (`R2`)
   - evidence: `src/cli/lint.rs:340` comments `"Other refusals (a type with no apply) stay quiet"`, whereas a non-dispatchable type (`Model`, `Gpu`, `Dataset`, `Recipe`) carrying `{{secrets.x}}` returns `Err(NOT_DISPATCHABLE)` and satisfies `has_unresolved_secret(resource)` at `src/cli/lint.rs:341`.
   - corrected: Non-dispatchable resources without secrets stay quiet (`Err(_) => {}`), while any resource containing literal `{{secrets.` that still fails `apply_script_for_lint` surfaces `bashrs: {id}/apply not linted` as a conservative fail-loud warning.
