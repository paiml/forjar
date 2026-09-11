# PMAT-522 — the claims, and where each is anchored

Six claims, one round of three sandboxed lanes, all three FAIL, five of the six
REFUTED. The round was nearly skipped and the quorum gate refused that; see
`comply-count-recursion-lanes.md`. Every claim below is anchored in a file at
the branch tip, and every refutation was re-run before it was accepted.

1. **A re-entry is refused, exits non-zero, and prints no count.**
   `tests/falsification_comply_count_cannot_run_inside_itself.rs:1`. Measured:
   exit 3, stdout empty, stderr naming the refusal.

2. **The sentinel reaches the child process.** It is exported, and it is tested
   BEFORE it is set — otherwise the first call would refuse itself.
   `tests/falsification_comply_count_cannot_run_inside_itself.rs:1`.

3. **The process cap is measured in threads.** `ulimit -u` is RLIMIT_NPROC:
   per user, counting threads. Measured on this account: 226 processes and
   2,352 threads at rest, 2,486 at the peak of one `pmat comply check`.
   `tests/falsification_comply_count_cannot_run_inside_itself.rs:1`.

4. **No committed ratchet config can re-open the loop.** The cycle needs two
   halves; this refuses the declaration.
   `tests/falsification_comply_count_cannot_run_inside_itself.rs:1`.

5. **The guard does not break the measurement.** The fourth case runs the real
   script with the cap on and asserts it prints a count.
   `tests/falsification_comply_count_cannot_run_inside_itself.rs:1`.

The adjudicated tally in `comply-count-recursion-judges.md` is **1 CONFIRMED,
7 REFUTED**: this session's own three failed attempts at the cap and its
misreading of the timeout, plus the three the lanes found — the cap failing
open, a ceiling lowered to a number the committed tree does not measure, and a
test that asserted on spelling rather than behaviour.
