# Judges — route arch-neutral CI jobs off starved ARM64 pool (PMAT-913 / forjar#981)

Round count and heads: see `PMAT-913-981-lanes.md`. Every claim `K1..K6` met 3 independent refuters across Correctness, Failure Modes, and Contract/Guards.

## CONFIRMED

1. [diff] C1 — `.github/workflows/{ci,proofs,behavior,convergence}.yml` route 13 non-prover gate jobs to `[self-hosted, clean-room, X64]` while preserving `gate.needs`, `proofs.needs`, and `, intel` bare-metal prover pinning (`proofs.yml:96,220`).
   - evidence: AST diff on `t2fw13` across all 20 workflows (`YAML_PARSE_OK 20`) verified only `runs-on`/`runs_on` labels and comments changed, `kani`/`lean`/`coverage`/`mutation`/`stress` still carry `intel`, and GitHub CI rollup on `32255669` is `SUCCESS`.
2. [test] C2 — `.github/workflows/lint.yml:42,44,149` and `.github/workflows/msrv.yml:16` safely drop `ARM64` to `[self-hosted, clean-room]` without cross-architecture ELF `target/` cache poisoning.
   - evidence: `bash tests/cache-key-arch-guard.sh` passed on `t2fw13` (`ok` on `lint.yml` and `msrv.yml`), and mutating `lint.yml` in `<SCRATCH>` to strip `-${{ runner.arch }}` turned `cache-key-arch-guard.sh` RED (`FJ981_CACHE_ARCH_GUARD_RED_OK`).
3. [diff] C3 — `.github/workflows/ci.yml:120-122` retains `examples-validate` on `[self-hosted, clean-room, ARM64]`, keeping a real `aarch64` build and integration guard smoke while cutting explicit per-PR `ARM64` legs from 17 to 1.
   - evidence: `.github/workflows/ci.yml:122` preserves `runs-on: [self-hosted, clean-room, ARM64]` gated by `needs.classify.outputs.code == 'true'`, and `.github/workflows/quorum.yml:37` remains `workflow_dispatch`-only so it adds zero PR queue load.
4. [test] C4 — All 20 workflow files preserve private `CARGO_HOME` isolation against the shared registry reaper (`infra#430` / `forjar#447`) and align with `PMAT-913` Gate A traceability once `forjar-cop` commits `docs/audits/impl-PMAT-913-receipt.md` + `docs/roadmaps/roadmap.yaml`.
   - evidence: `bash scripts/lint-rust-cache-guard.sh` (`5 $CARGO_HOME cache uses in 46 self-hosted jobs across 20 workflows`) and `--selftest` (`12/12` cases) passed on `t2fw13`, and PR `#981` body names `Ticket: PMAT-913` with zero conflicting commit trailers.

## REFUTED

1. [doc] R1 — Commit `535b2280` claimed `(18:35Z)` in `.github/workflows/ci.yml:59` and its commit message for the 205-job `ARM64` queue snapshot when the actual measurement was `~18:26Z`.
   - corrected: commit `d4a6693d` (`ci: correct the measurement time in the route-back comment`) updated `.github/workflows/ci.yml:59` to `2026-11-10 ~18:26Z`, leaving `(18:35Z)` only in the historical commit body of `535b2280`.
2. [doc] R2 — PR `#981` body under `**What.**` stated `"Moved to X64 (11 jobs) ... (That is 14 runs-on lines.)"`, miscounting both the jobs moved to `X64` and the modified runner lines.
   - corrected: the diff moves 13 jobs and 13 `runs-on`/`runs_on` lines to `X64` (`ci.yml`: 8, `proofs.yml`: 3, `behavior.yml`: 1, `convergence.yml`: 1) plus 4 arch-less lines in `lint.yml` (3) and `msrv.yml` (1) = 17 runner lines total; workflow YAML is exact.
