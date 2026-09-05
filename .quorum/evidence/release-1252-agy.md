# The independent review — /teamwork-preview

One agy /teamwork-preview run, briefed on the same four questions as the claim lanes.
TWO CAVEATS, both measured. First, it spawned NO child agents: the brain directory
count went from 503 to 504 across the whole wave, a delta of one, which is the run
itself, so this was a single-agent review wearing the teamwork name. Second, it ran in
wave 1, against the PRE-REFACTOR branch, so its file and line references predate the
move of the four gate cases into their own file and it never saw the two named helper
functions. Its reasoning about the anchor rule survives that move; its citations do not.

Verdict returned: do-not-implement-as-written

Summary: The branch introduces critical weaknesses to the quorum gate's forgery defense, adds an inert test that will never execute its assertions in CI, and places a new CI step where it can be silently skipped. It must be reworked.

## Findings

- Q1: Widening the anchor rule severely weakens the forgery defense. By allowing citations to added files, a pusher can fabricate claims anchored entirely in a new file they authored, defeating the requirement that anchors resolve against a tree the pusher did not author. Furthermore, the file-level rule makes the 33% floor trivially satisfiable for release branches—since Cargo.lock is touched, citing `Cargo.lock:1` counts as an anchor even though the branch didn't change that line. Citations to files outside the diff and past the end of new files are still refused.

- Q2: The four new cases are real and discriminating. They assert against exit codes and specific string outputs (e.g., '0/4', '9999'). They drive `quorum_evidence.py` directly with the exact 6 arguments used by `quorum-gate.sh`. The banner's measurements are mathematically consistent: both reverted ignores added files (0/4, 0%), reverting only CIT_RE anchors the added file (1/4, 25%), and reverting the added-file rule silently ignores out-of-range citations instead of explicitly refusing them (0/4, 0%).

- Q3: It is dishonest to call this test a 'message-bearing witness'. As admitted in the documentation, under `cargo test --locked`, a stale lockfile causes `cargo` to fail at dependency resolution. The test code never compiles or runs, and its carefully written assertion message is never seen in CI. An inert test that relies entirely on the build tool failing before execution is not a valid witness.

- Q4: The step does not weaken existing checks, add network access, unpinned actions, secrets, host writes, or change job configurations. However, the `examples-validate` job itself has NO `if:` or `needs:` clause that would cause it to be skipped. Instead, the step is placed at the end of 7 preceding `cargo test` steps; because it lacks an `if: always()` condition, any failure in a prior step will cause it to be silently skipped due to implicit step conditions.

## How the panel scored it

The independent review returned do-not-implement-as-written while all three judges
returned PASS. The divergence is real and is not resolved by majority: on its Q1 and
Q3 it agrees with the panel on the FACTS — the added-file rule does surrender the
authorship-independence property, and the message-bearing-witness phrase is
self-flattering — and differs only on whether those are blockers. On its Q4 it is
wrong on a checkable point: it argues the new CI step can be silently skipped because
an earlier step in the job may fail, which is ordinary fail-fast behaviour that turns
the job RED rather than a silent skip, and the panel did not adopt it.
