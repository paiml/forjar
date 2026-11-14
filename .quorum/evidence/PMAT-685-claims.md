# PMAT-685 & PMAT-848 (`forjar#903`) — Extracted Claim Manifest (`C1..C4`, `R1..R2`)

Base commit: `5b8f8123be40731340e2a74a6e4ae365648cab45`
Head commit: `765f9493b8898d7b40d143b2e8f1edfac2c974c5`
Branch: `forjar-cop/packC`

1. **C1 (`[code]`)**: `with_secret_placeholders` (`src/core/resolver/resource.rs:315`) and `apply_script_for_lint` (`src/core/codegen/dispatch.rs:350`) substitute `__FORJAR_SECRET_<key>__` on the lint/quality-gate path while keeping `apply_script` fail-closed.
2. **C2 (`[code]`)**: `lint_scripts` (`src/cli/lint.rs:341`) emits `bashrs: {id}/apply not linted: {e}` when a secret-bearing apply script still fails codegen.
3. **C3 (`[code]`)**: `crontab_check_guard` (`src/resources/cron.rs:32`) probes `LC_ALL=C sudo -n crontab -u {u} -l 2>&1 >/dev/null` instead of `sudo -n true`, eliminating false exit-`2` `SKIP` under `crontab`-only `sudoers` rules.
4. **C4 (`[test]`)**: `tests_lint_secret.rs`, `tests_secret_apply.rs`, and `tests_cron_exec.rs` falsify regressions on both fixes (`FJ903_REV1_KILLED` and `FJ903_REV2_KILLED` on `t2fw13`), and `crontab_check_guard` introduces `0` `bashrs` errors.
5. **R1 (`[doc]`)**: `docs/audits/impl-PMAT-685-receipt.md:3` claims `quality_gate` also reports ungenerated apply scripts, whereas `src/core/quality_gate/checks.rs:33` silently ignores `Err(_)`.
6. **R2 (`[doc]`)**: `src/cli/lint.rs:340` claims non-apply-type refusals stay quiet, whereas non-dispatchable types carrying `{{secrets.x}}` emit `not linted`.
