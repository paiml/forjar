# Quorum evidence — #791 (fix/restore-x64-runs-on) — adjudicated claims

Round design: 3 claim lanes (claude-opus-5-5, three review roles), 3 adversarial refuters per claim and 3 judges, all claude-opus-5-5 because the author is Sonnet 5.5. One planted control (C4) was injected and killed 3 of 3.

## CONFIRMED — 3 claims survived refutation

1. [design] (C1) Every self-hosted clean-room job in .github/workflows now runs on `[self-hosted, clean-room, X64]`, restoring the label PMAT-756 dropped (R1: restore X64, smallest diff).
   - evidence: the guard loop at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:149 collects any self-hosted job without an arch label, and the test at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:202 passes on the PR head (13 passed, 0 failed).

2. [design] (C2) `ARCH_NEUTRAL` is empty, so `a_fleet_job_names_the_architecture_it_needs` fails any self-hosted job that names no architecture; the failure text names the file and the job.
   - evidence: the table is empty at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:145 and the assertion that reports unpinned jobs sits at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:204. With the base workflows restored the test fails (12 passed, 1 failed).

3. [design] (C3) The extraction of `arch_scan` and `stale_arch_neutral` changes no behaviour; the staleness check still fails a listed entry that is gone or that carries an arch label, and both assert messages are unchanged.
   - evidence: tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:149 returns the unpinned list and the seen records before any filter, tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:175 reports stale entries, and the test body at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:215 asserts on them with the same message as before.

## REFUTED — 2 claims did not survive

4. [planted] (C4) ARCH_NEUTRAL admits any unpinned job whose workflow file name is listed.
   - corrected: false. The table is empty at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:145 and the neutral match at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:162 requires file and job both to match an entry.

5. [design] (C5) The diff guarantees the gx10 ARM64 runners will never pick up these jobs.
   - corrected: the diff is label-only. Runner groups, labels and access are configured outside the repository, and reusable-workflow callers such as the jobs near .github/workflows/ci.yml:38 carry no guarantee from this file alone; the diff makes the jobs label-ineligible for ARM64, nothing more.
