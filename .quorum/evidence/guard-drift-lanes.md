# Quorum evidence — PMAT-215 — lane rulings

Three sandboxed review lanes on the diff at `39aa4a09`, each in its own `git clone --shared` copy. Verdicts 1 PASS, 2 FAIL. Both substantive findings were re-run by the orchestrator before anything was changed, and both reproduced.

## The census case was too weak, and the lane was right

Lane 2 refused `what_cannot_be_evaluated_is_still_not_counted_as_clean` because it asserts on `--no-task-checks` output. That is the operator DECLINING a measurement, not a resource that genuinely cannot be evaluated, and the acceptance criterion is about the second. It proposed a failed file or a task with no `completion_check` instead.

A fourth case now uses a task with no `completion_check` whose lock says `failed`. Nothing can evaluate it: the task path skips it without a census entry by design, because it has no assertion, and the state-query path still excludes `Failed` because a failed apply's digest is not a baseline. The case asserts it is named as unmeasured, not counted as inspected, and not invented as drift.

That case pins the NARROWNESS, which is the part most at risk. Mutating `src/tripwire/drift/mod.rs` to remove its `NotConverged` skip, as if the task-path reasoning applied there too, fails that one case and leaves the other three green.

The finding nearly did not survive its own transport. Lane 2 emitted its structured output twice and the second emission was truncated to an empty `findings` list, so the reducer scored the round with the one real dissent invisible; the substantive text lived only in the lane's `.response`. The delegate said so in its receipt rather than reporting a clean round.

## An out-of-scope file, and the lane was right again

Lane 3 refused the branch because `CLAUDE.md` was in the diff: nine lines of a `paiml-implement:orch-routing` block. It was staged in the working tree before this branch existed and rode along into the first commit. Reverted; the diff is now two files, the fix and its test.

Lane 3's other four findings and all five of lane 1's agreed the fix is sound: the assertion-versus-baseline distinction holds, no `ResourceStatus` makes a read-only check unsafe to run, and `SkipReason::NotConverged` being dead for tasks while live for file and image is consistent rather than an oversight.

## What the lanes did not judge

They were told the workspace gate is red on `forjar-contracts` `coverage_map_enrichment` and that it reproduces on `origin/main`, so none of them spent a finding on it. It is filed on forjar#452 and recorded in the receipt's jidoka section.
