# Quorum evidence — #791 round 2 (fix/restore-x64-runs-on) — adjudicated claims

Round design: 3 claim lanes (claude-opus-5-5, roles code, workflow side, gate weakening), each also refuting every claim, then 3 independent claude-opus-5-5 judges. The author is Sonnet 5.5, so no lane used its model. One fresh planted control (C4) was injected and refuted 3 of 3 by lanes and 3 of 3 by judges. This round replaces the stale round-1 binding after the receipt commit changed the gate hash.

## CONFIRMED — 3 claims survived refutation

1. [design] (C1) Every self-hosted runs-on that a workflow declares now names X64: the diff turns 18 bare `[self-hosted, clean-room]` lines into `[self-hosted, clean-room, X64]`, and the matrix legs already carried X64.
   - evidence: the guard at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:165 pushes any self-hosted job without an arch label, and the live test passes on the PR head (13 passed, 0 failed, run by judge 2 and judge 3). Qualifier recorded: ci.yml, nightly-bench.yml and pr-gate.yml hand their runner to a reusable workflow and declare no runs-on here.

2. [design] (C2) ARCH_NEUTRAL is empty, so a_fleet_job_names_the_architecture_it_needs fails any self-hosted job that names no architecture, and the failure line names the file and the job.
   - evidence: the table is empty at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:145, the failure line is built at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:166 from the file, the job name and the labels, and the assert that reports the list sits at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:204.

3. [design] (C3) Extracting arch_scan and stale_arch_neutral changes no behaviour: seen is still pushed before the self-hosted filter, the three old continues equal one conjunction, and both assert messages are unchanged context lines.
   - evidence: tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:161 pushes every job to seen, tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:165 holds the single condition, and tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:175 keeps the stale-entry branches with byte-identical strings.

## REFUTED — 2 claims did not survive

4. [planted] (C4) The diff modifies production Rust under src/ in addition to the workflows and the test.
   - corrected: false. The saved diff has 12 file headers, 11 workflow files plus tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:145 side of the test, and none is under src/. The src/ changes seen against origin/main belong to the epic commits below the PR base epic/forjar-img-001.

5. [design] (C5) The diff guarantees the gx10 ARM64 runners can never pick up these jobs.
   - corrected: the diff only adds a label. Exclusion depends on how the runners are registered, outside the repo, and the reusable-workflow jobs at .github/workflows/ci.yml:54 declare no runs-on here, so nothing in this file constrains them; the guard note at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:190 records the same limit.
