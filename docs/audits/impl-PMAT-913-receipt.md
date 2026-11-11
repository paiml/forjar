# Implementation receipt — PMAT-913 — CI and release gate hygiene (train)

verdict: PASS — PR #981: the arch-neutral gate jobs run on the X64 clean-room pool (three reviewed arch-neutral jobs on either pool) instead of the starved ARM64 pool; only `runs-on` lines change, and `examples-validate` stays on ARM64 as the aarch64 smoke.

## Identity

- ticket PMAT-913 (forjar#913, train "CI and release gate hygiene"); kind: ci
- PR #981, branch `fix/arm64-route-x64`, change head 3225566
- files: `.github/workflows/behavior.yml`, `ci.yml`, `convergence.yml`, `lint.yml`, `msrv.yml`, `proofs.yml` (+23 −20), `docs/roadmaps/roadmap.yaml` (row `GH-913` renamed to `PMAT-913`)

This receipt covers PR #981 only at this commit. PR #978 (same train) carries its own section of this file on its branch; whichever lands second merges the two sections.

## PR #981 — route arch-neutral gate jobs off the ARM64 pool

Root cause (five whys):

1. Twelve or more forjar PRs sat PENDING for hours on 2026-11-10 and 2026-11-11.
2. Their gate jobs were queued, not running.
3. Every gate job asked for `[self-hosted, clean-room, ARM64]`.
4. Six ARM64 clean-room runners were online (gx10) for 205 queued forjar jobs (measured about 18:26Z, 2026-11-10, in the PR body).
5. The 2026-10-27 move (#827) had pinned the jobs to ARM64 when that pool was the idle one, and nothing moved them back when the load reversed.

Contract: gate content is unchanged; only runner labels change. Eleven jobs move to X64, `lint.yml` clippy and file-health and `msrv.yml` take either pool (already reviewed as arch-neutral, forjar#722, cache keys carry `runner.arch`), `examples-validate` and the dispatch-only `quorum` stay on ARM64.

Falsifier: a job that is green on ARM64 and red on X64 goes back to ARM64 alone, with a ticket (the #827 falsifier, reversed). A `runs-on` revert takes minutes.

Evidence:

- CI on 3225566, run under the PR's own workflow files: rollup SUCCESS (read 2026-11-11 about 13:00Z), so every moved job ran green on its new pool.
- The diff against main at 3225566 changes `runs-on`/`runner`/`runs_on` lines and their comments only.

Known cost, from the PR body: the X64 clean-room pool was also full when measured (8 of 8 busy, about 69 queued), so the moved jobs compete there.

Not measured by forjar-cop on a build host: nothing was compiled for this receipt. The change is workflow YAML, and the PR's CI run is the execution of it.

IMPL-PMAT-913-RECEIPT-END
