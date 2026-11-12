# PMAT-913 (`forjar#972`) — Extracted Claim Manifest (`C1..C4`, `R1..R2`)

Base commit: `c4ce3a2c5a4d770a79d4dfdcb6295cd438a9e7da`
Head commit: `4689c9832fc9254c07ece51e7c7e309bcd69ca63`
Branch: `chore/runner-smells-baseline`

1. **C1 (`[code]`)**: `26` non-reusable jobs across `11` workflow files (`audit.yml`, `bench.yml`, `binary-release.yml`, `ci.yml`, `lint.yml`, `msrv.yml`, `nightly.yml`, `proofs.yml`, `quorum.yml`, `release-goal.yml`, `stress.yml`) gain explicit `timeout-minutes` bounds (`15` to `120` minutes), documented in `CHANGELOG.md:10-13`, with all `20` workflow YAML files parsing cleanly on `yoga`.
2. **C2 (`[code]`)**: `.github/workflows/ci.yml:34-37` and `.github/workflows/lint.yml:27-30` (`CHANGELOG.md:11-12`) declare workflow-level `CARGO_PROFILE_DEV_DEBUG: 0`, `CARGO_PROFILE_TEST_DEBUG: 0`, and `CARGO_INCREMENTAL: 0`, merging cleanly with per-job `CARGO_HOME` env maps without altering any job's `if:` or `needs:` graph.
3. **C3 (`[test]`)**: Round-2 telemetry fixes bump `.github/workflows/release-goal.yml:35` from `15` to `60` minutes (`2.8x` above `21.4 min` green runs) and `.github/workflows/stress.yml:17` from `60` to `120` minutes (`1.26x` above `95.2 min` bare-metal loaded runs on `intel`), eliminating false-timeout kills on green gates.
4. **C4 (`[doc]`)**: `CHANGELOG.md:10-13` and `docs/audits/impl-PMAT-913-receipt.md:103-135` reconcile the exact `26`-job / `11`-workflow counts, include the measured max-duration table for every touched workflow, preserve `.github/workflows/pr-gate.yml` byte-identical to `origin/main`, and maintain `IMPL-PMAT-913-RECEIPT-END`.
5. **R1 (`[scope]`)**: Round 1 (`8fb8f9f0`) set `release-goal.yml` to `15` minutes and `stress.yml` to `60` minutes without measuring non-`ci.yml` workflows (`impl-PMAT-913-receipt.md` at `8fb8f9f0`), which all 3 review lanes refuted using historical GitHub Actions durations (`18.1–21.4m` and `74.3–95.2m`) and `4689c983` corrected to `60` and `120`.
6. **R2 (`[scope]`)**: `.github/workflows/release.yml` (`7` jobs) was dropped from `#972` in Round 2 (`docs/audits/impl-PMAT-913-receipt.md:110`, `CHANGELOG.md:11` narrowed from `33` to `26` jobs) because its last 5 runs on `main` failed and `v1.34.1` is in flight, leaving `release.yml` timeouts for a post-release slice.
