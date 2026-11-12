# Judges — Job Timeouts and Leaner CI Env (PMAT-913 / forjar#972)

Round count and heads: see `PMAT-913-972-lanes.md`. Every claim `C1..C4` and `R1..R2` met 3 independent refuters across Workflow Semantics, Empirical Actions Telemetry, and Contract Reconciliation.

## CONFIRMED

1. [code] C1 — `26` non-reusable jobs across `11` workflow files gain bounded `timeout-minutes` (`CHANGELOG.md:11`) with valid GitHub Actions YAML structure.
   - evidence: `git diff origin/main...4689c9832fc9254c07ece51e7c7e309bcd69ca63 -- .github/workflows` adds exactly `26` `timeout-minutes` lines across `11` workflows (`CHANGELOG.md:11`), omits `timeout-minutes` on `uses:` caller jobs, and passes `yaml.safe_load` across all `20` workflows on `yoga` (`/var/tmp/agy-fj972`).

2. [code] C2 — Workflow-level `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, and `CARGO_INCREMENTAL=0` (`CHANGELOG.md:12`) propagate without clobbering per-job `CARGO_HOME`.
   - evidence: `.github/workflows/ci.yml:34-37` and `.github/workflows/lint.yml:27-30` (`CHANGELOG.md:11-13`) declare the three Cargo variables at workflow scope, merging with job-level `env:` (`CARGO_HOME`) while leaving `if:`, `needs:`, and `runs-on:` graphs untouched (`CHANGELOG.md:13`).

3. [test] C3 — Every added `timeout-minutes` ceiling holds positive empirical headroom above measured green and loaded GitHub Actions durations.
   - evidence: `.github/workflows/release-goal.yml:35` (`60` min vs `18.1–21.4` min across green runs `34750974335`, `34685068751`, `34584275279`) and `.github/workflows/stress.yml:17` (`120` min vs `26` min unloaded and `74.3–95.2` min loaded on `intel`) match `.github/workflows/ci.yml:293` (`dogfood-surface: 60` vs `28` min) with zero false-timeout risk.

4. [doc] C4 — `CHANGELOG.md:10` and `docs/audits/impl-PMAT-913-receipt.md:103` reconcile exact job/file counts and restore `.github/workflows/pr-gate.yml` to `origin/main`.
   - evidence: `CHANGELOG.md:10-13` and `docs/audits/impl-PMAT-913-receipt.md:108-135` both record `26` jobs across `11` workflow files with a full measured-duration table, `.github/workflows/pr-gate.yml` has zero diff against `c4ce3a2c`, and `IMPL-PMAT-913-RECEIPT-END` closes the receipt cleanly.

## REFUTED

1. [scope] R1 — Round 1 (`8fb8f9f0`) assigned `timeout-minutes: 15` to `release-goal.yml` and `timeout-minutes: 60` to `stress.yml` from unmeasured estimates (`CHANGELOG.md:11` at `8fb8f9f0`).
   - corrected: All 3 review lanes refuted those two ceilings using GitHub Actions REST telemetry (`release-goal` green runs took `18.1–21.4` min once Gate T passed; `stress` legs took `74.3–95.2` min under `intel` contention), and commit `4689c983` raised them to `60` and `120` minutes (`docs/audits/impl-PMAT-913-receipt.md:122-123`).

2. [scope] R2 — Round 1 (`8fb8f9f0`) included `7` unmeasured timeouts on `.github/workflows/release.yml` (`33` jobs across `13` workflows).
   - corrected: Because `release.yml`'s last 5 runs on `main` failed and `v1.34.1` is actively in flight, commit `4689c983` restored `.github/workflows/release.yml` and `.github/workflows/pr-gate.yml` byte-identical to `origin/main`, narrowing `CHANGELOG.md:11` from `33` to `26` jobs (`docs/audits/impl-PMAT-913-receipt.md:110`).
