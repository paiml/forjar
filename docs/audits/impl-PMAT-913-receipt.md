# Implementation receipt — PMAT-913 — CI and release gate hygiene (train)

verdict: PASS — PR #981: the arch-neutral gate jobs run on the X64 clean-room pool (three reviewed arch-neutral jobs on either pool) instead of the starved ARM64 pool; only `runs-on` lines change, and `examples-validate` stays on ARM64 as the aarch64 smoke.

## Identity

- ticket PMAT-913 (forjar#913, train "CI and release gate hygiene"); kind: ci
- PR #981, branch `fix/arm64-route-x64`, change head 3225566
- files: `.github/workflows/behavior.yml`, `ci.yml`, `convergence.yml`, `lint.yml`, `msrv.yml`, `proofs.yml` (+23 −20), `docs/roadmaps/roadmap.yaml` (row `GH-913` renamed to `PMAT-913`)

This receipt covers PR #981 only at this commit. PR #978 (same train) carries its own section of this file on its branch; whichever lands second merges the two sections. PR #904 (a dependabot action bump filed under this train) has its own section below.

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

- Three-lane review of PR #981 at 3225566 (agy, Gemini 4 Carbon): 3/3 PASS, 2 prose claims refuted (a timestamp already fixed in d4a6693d; the PR body said 11 jobs where the diff moves 13, body corrected). Quorum receipt: `.quorum/fix-arm64-route-x64.json`, written by agy and committed unedited.

## PR #904 — softprops/action-gh-release 3.0.2 to 3.0.4

Change: one line, `.github/workflows/nightly.yml:247` (`Create nightly release`), the pinned commit `3d0d9888cb7fd7b750713d6e236d1fcb99157228  # v3.0.2` becomes `8a0cd651e5b2365bd640854a3fa5dfc70e4875f3  # v3.0.4`. `git diff origin/main --shortstat` on the PR branch merged with main at dbd1ec3e: 1 file changed, 1 insertion, 1 deletion. It is the only `softprops/action-gh-release` call site in the file.

Contract: the nightly release step runs the v3.0.4 action; no `with:` input changes.

Evidence (read 2026-11-11, `gh api`, 2 read-only calls):

- `repos/softprops/action-gh-release/git/ref/tags/v3.0.4` is an annotated tag (object 28ab305d0ffa26d60ec4bd614126329e26c7a0ec); `git/tags/28ab305d...` dereferences it to commit 8a0cd651e5b2365bd640854a3fa5dfc70e4875f3, which is the SHA the PR pins. The pin is the commit, not the tag object.
- Pin style matches the file: every other `uses:` in `nightly.yml` is a 40-character commit SHA with a `# vX.Y.Z` comment (`actions/checkout`, `upload-artifact`, `download-artifact`); the one exception, `dtolnay/rust-toolchain@stable` (line 129), was already unpinned on main.

Not measured: the nightly release step itself was not run on a host; the PR's CI run does not exercise it (it runs on schedule). Nothing was compiled for this section. Whether v3.0.4 changes release behaviour was not read from the upstream changelog.

CHANGELOG: no Unreleased entry. The file lists no individual GitHub Action bump (the 1.34.0 "CI and release engineering" line names gate changes, not action pins), and `[Unreleased]` on main is empty.

Quorum receipt: not written here. `.quorum/<slug>.json` is the reviewer's file, and the review round for #904 is the place it comes from.

IMPL-PMAT-913-RECEIPT-END
