# PMAT-522 — the claims, and where each is anchored

No review round was dispatched for this ticket, and that is recorded rather
than papered over: the finding was a LIVE HAZARD on a shared machine, filed by
the operator after killing 9,740 processes, and the fix went in directly. What
stands in for the round is that every claim below is driven by a case that runs
the real script on the real machine, and that the operator's own measurements
are the input rather than this session's.

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
4 REFUTED** — the refutations are this session's own three failed attempts at
the cap plus its misreading of the timeout, adjudicated against the operator's
measurements rather than against a lane's opinion.
