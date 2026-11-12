# Claims — `stress.yml` fits its 30-min timeout with `fail-fast: false` & `SEC005` token-boundary regression guard (PMAT-673 / forjar#914)

Extracted from PR `#914` (`s-forjar-2/pack696b`, head `7c609178`) and attacked across all 3 review lanes:

- **C1 (`stress.yml` pass count fits 30-minute step budget)**: `.github/workflows/stress.yml` reduces `seq 1 10` (~30 min on contended shared runners, exceeding 20 min) to `seq 1 3` under `timeout-minutes: 30` (`tests/falsification_stress_workflow_fits_its_timeout.rs:18`).
- **C2 (`stress.yml` matrix isolation via `fail-fast: false`)**: `.github/workflows/stress.yml` sets `strategy.fail-fast: false` so a failure in one stress leg (`determinism`, `property-chaos`, `concurrency-race`) does not cancel sibling legs (`tests/falsification_stress_workflow_fits_its_timeout.rs:34`).
- **C3 (`SEC005` `sk-` token-boundary falsification test)**: `tests/falsification_sec005_needs_a_token_boundary.rs:10` verifies words ending in `sk` (`task-1`, `disk-budget`, `mask-secret`) are not misflagged as `sk-` API keys while real `sk-...` literals are flagged.
- **C4 (`CHANGELOG.md` & `PMAT-673` roadmap/receipt traceability)**: `CHANGELOG.md:8`, `docs/roadmaps/roadmap.yaml`, and `docs/audits/impl-PMAT-673-receipt.md` record `PMAT-673` cleanly after merging `origin/main` (`#982`).
- **R1 (Overloaded-host `LLVM linker crash` & home-relative path in `impl-PMAT-673-receipt.md`)**: `docs/audits/impl-PMAT-673-receipt.md` recorded `verdict: partial` from an overloaded first run where doctests crashed the linker.
- **R2 (Undocumented second ticket test `falsification_sec005_needs_a_token_boundary.rs` in PR title/summary)**: PR `#914` ships both `PMAT-673` (`stress.yml`) and `SEC005` token-boundary regression tests (`tests/falsification_sec005_needs_a_token_boundary.rs:10`) under a single pack branch.
