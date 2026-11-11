# Claims — route arch-neutral CI jobs off starved ARM64 pool (PMAT-913 / forjar#981)

1. K1: `.github/workflows/{ci,proofs,behavior,convergence}.yml` route their non-prover arch-neutral gate jobs to `[self-hosted, clean-room, X64]` without altering gate commands, `needs:` edges, or bare-metal `, intel` prover labels.
2. K2: `.github/workflows/{lint,msrv}.yml` drop `ARM64` to `[self-hosted, clean-room]` safely because every `target/` cache key in `lint.yml` and `msrv.yml` includes `runner.arch`.
3. K3: `.github/workflows/ci.yml:120-122` keeps `examples-validate` on `[self-hosted, clean-room, ARM64]` as the `aarch64` build-and-integration smoke, cutting explicit per-PR `ARM64` demand from 17 legs to 1.
4. K4: All 20 workflows satisfy `scripts/lint-rust-cache-guard.sh` private-`CARGO_HOME` isolation and YAML validity, and PR `#981` aligns with `PMAT-913` Gate A/E requirements once `forjar-cop`'s held docs commit lands.
5. K5: Commit `535b2280` accurately recorded the timestamp of the 205-job `ARM64` queue measurement.
6. K6: PR `#981` body under `**What.**` accurately counts `(11 jobs)` and `(14 runs-on lines)` moved to `X64`.
