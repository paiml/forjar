# Claims — tool-requiring jobs on bare metal (PMAT-598)

Counts and heads live in `PMAT-598-lanes.md`; numbers here quote it.

The adjudicated claim set is the one rounds 2 and 3 reviewed, after round 1 had
already forced the scope to widen from three jobs to five:

- **C1** exactly five jobs re-routed — proofs/kani, proofs/lean, coverage, stress, mutation
- **C2** the change only narrows the eligible runner set
- **C3** `[self-hosted, clean-room, X64, intel]` matches only intel bare metal, and is non-empty
- **C4** name-filtered and targeted test runs are correctly left unpinned
- **C5** the test discovers the jobs from parsed YAML, and every declared mutation reddens exactly its own arm
- **C6** the note on `ci / test` is an honest limit, not a claim of coverage

## What the three rounds did

Round 1 widened the fix. Round 2 and round 3 each found a false negative in the
detector — and round 3's first finding was in round 2's own fix. Round 3 also
produced a hand-trace, from a lane, that was wrong: it reported a `&&` chain as
classified correctly when it was not, and that was caught only because every
trace in the test is backed by an asserted case.

The detector's final two refinements (libtest value flags after `--`, and shell
separators) were made in response to round 3 and no lane has reviewed them.
They are covered by asserted cases on both edges and by the RED proofs.
