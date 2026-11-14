# Implementation receipt — PMAT-913 — CI and release gate hygiene (train)

verdict: PASS — PRs of the train, one section per change. PR #981: the arch-neutral gate jobs run on the X64 clean-room pool (three reviewed arch-neutral jobs on either pool) instead of the starved ARM64 pool; only `runs-on` lines change, and `examples-validate` stays on ARM64 as the aarch64 smoke. PR #978: the `examples-validate` job runs the same 20 non-sudo integration test targets, in the same order, in one `cargo test` invocation instead of 18 steps; no target is dropped and the live-sudo step stays separate. Folded into PR #978 on 2026-11-11: the reusable workflow's jobs go back to the ARM64 clean-room pool, and the mutation job reads the clock when it starts and does nothing outside 00:00-05:59 UTC (review of these two changes is requested at the new head and is not part of the PASS above until it lands).

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

## PR #978 (folded in) — reusable workflow's jobs back to the ARM64 clean-room pool

A separate PR for this was refused by the open-PR cap (25 open, cap 12), so the change rides on PR #978, same ticket.

Root cause (five whys):

1. After #981, `ci / lint`, `ci / security`, `ci / provenance` and `ci / gate` ask for `[self-hosted, clean-room, X64]`.
2. This repository is public.
3. The X64 clean-room runner group does not serve public repositories.
4. So the X64 jobs of `ci.yml` share one seat (infra-cop's measurement, 2026-11-11, relayed by fleet-sup 17:35Z; not re-measured by forjar-cop).
5. #981 moved the jobs on queue length alone and did not check which runner groups serve this repository.

Contract: the `runs_on` input of the `ci` job is `["self-hosted","clean-room","ARM64"]`; no job content changes; the jobs defined in `ci.yml` itself stay where #981 put them.

Falsifier: on this PR's CI run the four reusable-workflow jobs must be taken by an ARM64 clean-room runner and finish green. Green on X64 and red on ARM64 reverts the line.

## PR #978 (folded in) — mutation job: earlier cron, night window read at job start

Root cause (five whys):

1. A mutation run sat queued on `intel` through the working day of 2026-11-11 (run 43009168778, cancelled 17:18Z).
2. It was a `schedule` run created at 11:06 UTC.
3. The cron was `17 3 * * *`: GitHub delivered the 03:17 slot about 7 h 50 min late.
4. Nothing in the workflow looked at the clock, so a late delivery ran, or queued, by day.
5. #973 made the workflow schedule-only but took the schedule's delivery time on trust.

Change: cron `17 0 * * *`; a first step reads the UTC hour when the job starts and, from 06:00 on, every later step is skipped and the job ends with a notice.

Contract: started at an hour below 6 UTC, the job runs as before; otherwise no step after the first runs. Triggers stay `schedule` only.

Falsifier: the hour test, run by hand for 0, 5, 6, 11 and 23, gave run, run, stop, stop, stop; `date -u +%-H` for 08:05 printed `8` (no octal reading). The parsed workflow shows the condition on every step after the first.

Not covered:

- The job stays pinned to `intel`. The ARM64 clean-room containers were checked as a second home (gx10-pool1, gx10-pool3, gx10-light1, read-only): `cargo-mutants` is present, `rsync` is absent, and the unfiltered lib tests need rsync (#598). `tests/falsification_tool_jobs_run_where_the_tools_are.rs` pins the label for that reason. Moving the job needs rsync in that image first.
- No run of the workflow has exercised the new step: it is schedule-only, so the first measurement is the next night's run.
- A late delivery now costs that day's mutation run; nothing retries it.
- Nothing was compiled for these two sections; the repository's workflow tests run in this PR's CI.


## PR #972 — job timeouts and no-debug-info env (reduced scope)

Not yet merged; the verdict above covers the merged PRs only. Scope after the coordinator ruling of 2026-11-12 10:59Z:
this PR keeps only what does not change which jobs run or what a gate accepts.

- In: `timeout-minutes` on 26 jobs in 11 workflow files; `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_INCREMENTAL=0` as workflow env in `ci.yml` and `lint.yml`.
- Out, kept on the local branch `chore/runner-smells-job-and-draft-skip` (5f5112c3) for a separate PR with a proof run: the `runner-smells` called workflow and `.github/runner-smells.baseline`; the draft-PR skip and `ready_for_review` trigger on `audit.yml` and `lint.yml`; the `concurrency` group on `pr-gate.yml`.
- Out, not measurable: the 7 timeouts on `release.yml`. Its last 5 runs all failed, so no green duration exists, and a release is in flight. `release.yml` and `pr-gate.yml` are byte-identical to main.
- `git diff origin/main -- .github` adds only `timeout-minutes` lines (26) and the two env blocks; every workflow file parses (`yq -e .jobs`); `ci.yml` has 0 non-reusable jobs without a timeout.

### Timeouts against measured durations

Measured on lambda through the REST API: for each touched workflow, the longest successful job in its last 3 successful runs (nightly: 1 run per build leg).

| workflow | job | longest green | timeout |
|---|---|---|---|
| ci.yml | dogfood-surface | 28 min | 60 (was 30 in this PR's first version) |
| ci.yml | examples-validate | 26 min | 60 (was 30) |
| ci.yml | every other job | 5 min or less | 15 to 30 |
| release-goal.yml | release-goal | 21 min | 60 (was 15; review round 1) |
| stress.yml | stress (3 legs) | 26 min; the review cites 74 and 95 min under host load (runs 40300791720, 42617061589), not re-measured here | 120 (was 60; review round 1) |
| bench.yml | benchmark | 21 min | 60 |
| binary-release.yml | build (4 targets) | 12 min | 120 |
| nightly.yml | build (7 legs) | 15 min | 120 |
| proofs.yml | kani | 10 min | 150 (unchanged from main) |
| msrv.yml | msrv | 11 min | 60 |
| lint.yml | clippy | 4 min | 60 |
| audit.yml, quorum.yml | audit, quorum receipt | 3 and 6 min | 30 and 15 |

Every job with a timeout in this PR had at least one successful run in the sample; jobs whose longest run was under 3 minutes are not listed. Not measured: CI on this head.

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
