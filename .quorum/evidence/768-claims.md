# Quorum evidence — #768 / PR 771 (fix/768-drop-x64) — adjudicated claims

Round design: 3 claim lanes (agy gemini-4-argon-high, agy gemini-3.8-flash-high, claude-haiku-4-5) produced the claims,
3 adversarial refuters (the same three models) tried to kill each one, 3 judges (the same three models) decided from the
code. One planted control claim (C9 below) was injected; all three refuters and all three judges killed it.
No Opus or Sonnet lane sat on this round: the commits are co-authored by both.

## CONFIRMED — 8 claims survived refutation

1. [design] (C1) `ARCH_NEUTRAL` lists exactly the 19 workflow jobs that lost the `X64` runner label, and every listed job exists in the workflow files and names neither `X64` nor `ARM64`.
   - evidence: the table opens at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:142 and the staleness loop at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:274 fails on a listed job that is missing or that carries an arch label; the test passes on the PR head (13 passed, 0 failed).

2. [design] (C2) `a_fleet_job_names_the_architecture_it_needs` fails for any self-hosted job that names no architecture and is not listed in `ARCH_NEUTRAL`; the failure message names the file and the job.
   - evidence: the job is collected into `unpinned` when it has `self-hosted`, no arch label and no exact (file, job) match at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:253, then `assert!(unpinned.is_empty())` fires at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:263.

3. [design] (C3) The guard is two-sided: a listed entry whose job was deleted, or whose job has gained an `X64` or `ARM64` label, fails the test with a message naming the entry.
   - evidence: tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:274 walks the table and pushes to `stale` on the missing case and on the has-arch case; the assertion that reports it follows at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:283.

4. [design] (C6) Artifact-producing jobs keep their chip pin: `binary-release.yml` job `build` and the Linux legs of `release.yml` job `build-binaries` still run on `[self-hosted, clean-room, X64]`, and neither is in `ARCH_NEUTRAL`.
   - evidence: the table at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:142 holds only chip-independent jobs (lint, classify, audit, doc tests, gate aggregators); a release job added to it would fail the has-arch branch at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:277.

5. [design] (C7) Every one of the 20 sites that lost `X64` (19 job-level `runs-on` lines and the two Linux `clippy` matrix legs, counted as 20 lines) keeps both `self-hosted` and `clean-room`, so none escapes the clean-room pool.
   - evidence: the test itself only inspects jobs that carry `self-hosted` (tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:247); all three judges read the 20 edited lines and found `[self-hosted, clean-room]` on each.

6. [design] (C8) Against the epic base, the PR's own delta is 12 files: 11 workflow files and the fleet test. Inside the workflows only runner-label lines change, so no job id, job name or step moves and no required check is renamed by this PR.
   - evidence: the delta was diffed against the epic base (12 files, +149/-40) and the workflow hunks are single-line label edits; the one Rust file is the table and guard from tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:142 onward. The gate's own diff against the default branch is wider (49 files) because the epic is not merged yet; one judge scored the claim on that wider diff, which is why the vote was 2 to 1.

7. [probe] (C11) The new guard is what turns the relaxed workflows green: the epic base's version of the test, run over these workflows, fails `a_fleet_job_names_the_architecture_it_needs`, and this PR's version passes.
   - evidence: measured. Epic-base test file restored: 12 passed, 1 failed, the failure is the architecture assertion in the epic-base test, naming audit, behavior, classify, doctests and others; PR head: 13 passed, 0 failed.

8. [design] (C12) The test reads the workflows from the checkout at run time, so a later edit to a runner label is seen by it rather than by a stored copy.
   - evidence: tests/falsification_every_ci_job_runs_on_the_fleet/scan.rs:27 reads the workflow directory with the standard library and parses each file; `runner_labels` is called on the parsed job at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:243.

## REFUTED — 4 claims killed

1. [design] refuted 3/3 — (C4) "The guard catches a matrix job where one leg names X64 and another leg names no architecture."
   - corrected: it does not. `runner_labels` flattens every matrix leg of a job into one list and `has_arch` is computed over that list (tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:245), so one `X64` leg marks the whole job as pinned and an unpinned sibling leg walks through. A real hole in the new guard, not introduced by the labels this PR drops; follow-up owner needed.

2. [design] refuted 3/3 — (C5) "The staleness check also fails when a listed ARCH_NEUTRAL job stops being self-hosted."
   - corrected: it does not. `seen` is pushed for every job before the `self-hosted` filter (tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:246), so a listed job moved to a GitHub-hosted runner stays present with no arch label and the entry never reads as stale. The list can outlive a job's leaving the fleet.

3. [probe] refuted 3/3 — (C9, PLANTED CONTROL) "ARCH_NEUTRAL admits an unlisted self-hosted job with no arch label, so long as its workflow file is in the list."
   - corrected: false by construction. The membership test requires BOTH the file and the job id to match (tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:255), so a new unlisted job in an already-listed workflow still lands in `unpinned` and fails the test.

4. [design] refuted 3/3 — (C10) "The diff guarantees that the gx10 ARM64 runners will pick up the relaxed jobs."
   - corrected: it only makes the 19 jobs label-eligible for the ARM64 runners. Whether gx10 takes them depends on runner-group access and scheduling outside the repository, which the test file's own comment records at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:224.
