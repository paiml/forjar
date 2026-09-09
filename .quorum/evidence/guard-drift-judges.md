# Quorum evidence — PMAT-215 — adjudicated claims

## CONFIRMED

1. [assertion] TASK-CHECK-NEEDS-NO-BASELINE — a `completion_check` asks the host a question and reads the answer, so the lock's status cannot make it unanswerable, and the task path is right to ignore that status where the hash paths are right to honour it.
- evidence: the exclusion this fix removes is the one still standing at src/tripwire/drift/mod.rs:289 at the merge base, whose own comment says Failed and Unknown stay out because their recorded hash records an apply that did not complete. That sentence is about a BASELINE, and a check has none to be missing. Two review lanes reached the same conclusion independently and neither could name a status for which running a read-only check is wrong.

2. [measurement] GUARD-NOW-REPORTED — a guard whose assertion is false is reported as drift and exits `--tripwire` non-zero, where before it was skipped and the run printed a clean verdict.
- evidence: measured end to end with the release binary against a temp state dir. Before: `skipped 1: not converged in the lock 1` and `No drift detected.` After: `DRIFTED: runner-registered on box (completion_check fails on box: task=pending)` and `--tripwire` exit 1. The row names its machine because of the change at src/cli/drift.rs:115, which printed the resource without it at the merge base.

3. [narrowness] HASH-PATHS-UNTOUCHED — the file, image and state-query paths still exclude `Failed`, and that is pinned rather than promised.
- evidence: the skip at src/tripwire/drift/file.rs:145 and the one at src/tripwire/drift/mod.rs:289 both resolve at the merge base and neither is in this diff. Mutating the second one away fails exactly one case in tests/falsification_drift_measures_a_broken_guard.rs:261, a file this branch adds, and leaves the other three green.

4. [scope] ONE EARLY RETURN — the whole behaviour change is the removal of a single early return and its now-unused parameter, in one file.
- evidence: `git diff --stat origin/main...HEAD` reports two files, src/tripwire/drift/task_check.rs and the new test. The function it changes is the `skip_reason` whose pre-fix body sat at src/tripwire/drift/task_check.rs:128 at the merge base, and the remaining reasons keep their original order so `lifecycle.ignore_drift` and `--no-task-checks` still surface as themselves.

## REFUTED

5. [census] DECLINED-IS-NOT-IMPOSSIBLE — the case pinning acceptance criterion 2 tested the operator declining a measurement, not a resource that cannot be measured.
- corrected: a fourth case now uses a task with no `completion_check` whose lock says `failed`. It has no assertion for the task path and no baseline for the state-query path, so nothing can evaluate it, and it must still be named as unmeasured. The earlier case is kept, because "declined" and "impossible" are different things and both must stay named; what changed is that the criterion is now pinned by the one it is actually about. The new case is at tests/falsification_drift_measures_a_broken_guard.rs:200, a file this branch adds.

6. [scope] OUT-OF-SCOPE FILE — nine lines of `CLAUDE.md` were in the diff and the ticket does not ask for them.
- corrected: the block was staged in the working tree before the branch existed and rode along into the first commit. Reverted in its own commit rather than quietly amended away, so the record shows it happened.

7. [process] A COMPARISON THAT MEANT NOTHING — the workspace gate's red was first attributed to this branch on evidence that could not support it.
- corrected: `coverage_map_enrichment` early-returns unless a sibling `aprender` is visible from `CARGO_MANIFEST_DIR/../..`, so running the branch in `~/src/forjar` and main in a worktree under `/tmp` compared a real run against a vacuous one. Re-running `origin/main` from a path where the corpus is visible reproduced the failure with none of this branch's changes present. Filed on forjar#452.
