# Implementation receipt — PMAT-913 — CI and release gate hygiene (train)

verdict: PASS — two PRs of the train, one section each. PR #981: the arch-neutral gate jobs run on the X64 clean-room pool (three reviewed arch-neutral jobs on either pool) instead of the starved ARM64 pool; only `runs-on` lines change, and `examples-validate` stays on ARM64 as the aarch64 smoke. PR #978: the `examples-validate` job runs the same 20 non-sudo integration test targets, in the same order, in one `cargo test` invocation instead of 18 steps; no target is dropped and the live-sudo step stays separate.

## Identity

### PR #981 (merged first, squash 305ba0ae8)

- ticket PMAT-913 (forjar#913, train "CI and release gate hygiene"); kind: ci
- PR #981, branch `fix/arm64-route-x64`, change head 3225566
- files: `.github/workflows/behavior.yml`, `ci.yml`, `convergence.yml`, `lint.yml`, `msrv.yml`, `proofs.yml` (+23 −20), `docs/roadmaps/roadmap.yaml` (row `GH-913` renamed to `PMAT-913`)

### PR #978

- ticket PMAT-913 (forjar#913, train "CI and release gate hygiene"; work requested under paiml/infra#3828); kind: ci
- PR #978, branch `chore/ci-dedupe-cargo`, change commit acc83726
- files: `.github/workflows/ci.yml` (+32 −123), `docs/roadmaps/roadmap.yaml` (row `GH-913` renamed to `PMAT-913`)

This receipt covers PR #981 and PR #978. Other PRs under the same train add their own section below when they land.

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

## PR #978 — one `cargo test` invocation for the non-sudo targets

Why: each of the 18 non-sudo steps paid its own cargo resolve and link plan on a shared runner pool that was already queued.

Contract: the set of `--test` targets executed by `examples-validate` is unchanged, and so is the order of the 20 non-sudo targets among themselves; `falsification_version_matches_manifest`, formerly the last step, now runs inside the merged invocation and so before the live-sudo target instead of after it; `falsification_sudo_transport_survives_closefrom` remains its own step gated on `FORJAR_REQUIRE_SUDO_TESTS == "1"`; `--locked` and the feature set are unchanged.

Falsifier: dropping or reordering one `--test` target fails the step-target bijection check (exit 1).

Evidence:

- Independent review, agy (Gemini 4 Carbon), run on t2fw13: `PASS @acc837269db9f9a1f6699f2376b8a28c30225451`. It parsed the job before (c5b7e4f) and after (acc8372), found the 20 targets 1:1 in order, the sudo step preserved, and killed the target-drop falsifier. File: `handoff/agy/review-forjar-978-acc8372.md` in the fleet inbox.
- Three-lane review at 20c4883 (agy; quorum receipt `.quorum/chore-ci-dedupe-cargo.json`, written by agy and committed unedited): all three PASS. Two claims of this receipt were refuted as written and are corrected above: the step count (18 non-sudo steps covering 20 targets, plus 1 live-sudo step, not 19 merged steps) and the order claim (the version-manifest target moved ahead of the live-sudo target).
- CI on acc83726: rollup SUCCESS, including `examples-validate`, `dogfood-guards` and `gate`. The merged step ran every target on the PR's own run.
- Counts from the PR body (non-comment lines of ci.yml, lint.yml, audit.yml): `cargo test` 21 → 4; `cargo clippy` and `cargo build/check/install` unchanged.

Not measured by forjar-cop on a build host: nothing was compiled for this receipt. The change is workflow YAML, and the PR's CI run is the execution of it.

IMPL-PMAT-913-RECEIPT-END
