# Implementation receipt — PMAT-913 — CI and release gate hygiene (train)

verdict: PARTIAL — PR #981 is PASS: the arch-neutral gate jobs run on the X64 clean-room pool (three reviewed arch-neutral jobs on either pool) instead of the starved ARM64 pool; only `runs-on` lines change, and `examples-validate` stays on ARM64 as the aarch64 smoke. PR #972 is PARTIAL: it merges with main and the 30 integration targets CI runs pass on gx10, but the runner-smells job and the gate-content sign-off are not done (see the PR #972 section).

## Identity

- ticket PMAT-913 (forjar#913, train "CI and release gate hygiene"); kind: ci
- PR #981, branch `fix/arm64-route-x64`, change head 3225566
- files: `.github/workflows/behavior.yml`, `ci.yml`, `convergence.yml`, `lint.yml`, `msrv.yml`, `proofs.yml` (+23 −20), `docs/roadmaps/roadmap.yaml` (row `GH-913` renamed to `PMAT-913`)

This receipt covers PR #981 and PR #972 at this commit. PR #978 (same train) carries its own section of this file on its branch; whichever lands second merges the two sections.

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

## PR #972 — runner-smells baseline and cheap workflow fixes (infra#3828)

- PR #972, branch `chore/runner-smells-baseline`; files: `.github/runner-smells.baseline` (new) and 13 workflow files, +66 −0 against main (`git diff --stat origin/main`, lane worktree), plus one Unreleased line in `CHANGELOG.md`; no Rust source changes.
- Merge with main: 11 commits behind, 4 conflicted files (`ci.yml` 2 hunks, `lint.yml`, `msrv.yml`, `proofs.yml` 1 hunk each), every hunk a `timeout-minutes:` line against a `runs-on:` line. Kept main's `runs-on` (from #981) and this PR's `timeout-minutes` and draft-skip `if:` lines; nothing else edited. After the merge the PR is still +66 −0 across the same 14 files, so no hunk is a duplicate of main.

Measured, host gx10, `fc-pre.sh pr-972`, tree synced with `.git` from a clone of the merged commit abe6f6a6:

- `cargo fmt --all -- --check`: rc 0.
- `cargo test --locked --no-fail-fast` over the 30 `--test` targets named in `.github/workflows/*.yml`: rc 0, 30 targets, 0 failed. This includes the targets that read workflow files (`falsification_683_cross_mounts_under_work_root`, `falsification_722_cache_key_arch`, `falsification_hosted_jobs_do_not_cache_target`, `falsification_nightly_gate_is_tag_vs_head`, `falsification_nightly_windows_openssl_perl`, `falsification_pr_lane_runs_what_the_change_can_break`, `falsification_pr_lane_selects_the_gate_the_change_can_move`, `falsification_quorum_gate_reads_the_pushed_ref`, `falsification_quorum_gate_reads_the_receipt_store`, `falsification_version_matches_manifest`).
- Re-run after the CHANGELOG entry was added: `falsification_version_matches_manifest`, `falsification_crux_gate_reads_the_release_section`, `falsification_pr_lane_runs_what_the_change_can_break`, `falsification_gate_a_the_pr_and_its_commits_name_one_ticket`, `falsification_dogfood_harness_and_quorum`: rc 0, 0 failed.

Not measured:

- clippy and `cargo test --lib` (no Rust source changes); the known LAB reds (forjar#858).
- The `runner-smells` scanner run by this lane; a reviewer lane (agy, round 1 at 2fe54ffb) reports it matches `.github/runner-smells.baseline` (RS-05 1, RS-07 1, RS-08 11, RS-09 2, RS-10 1).

Open:

- Round 1 review (agy, 3 lanes at 2fe54ffb) returned FAIL: the PR changes when or whether jobs run (`runner-smells` job outside `gate.needs`, `timeout-minutes`, draft-skip `if:`, `ready_for_review` trigger type, a `pr-gate.yml` concurrency group), which is gate content and needs a gate sign-off; three branch commits carry no `Pmat-Ticket:` trailer and two carry `PMAT-1`.
- The PR title and body name no `PMAT-<n>`; Gate A files a merged PR under a ticket named in its branch, title or body (`scripts/dogfood/harness.sh`).

IMPL-PMAT-913-RECEIPT-END
