# PMAT-522 — adjudicated claims

One round of three sandboxed agy quorum lanes, 3/3 FAIL. The round was nearly
skipped — see `comply-count-recursion-lanes.md` — and the quorum gate refused
the one-lane receipt that would have shipped without it.

The digest carries two kinds of refutation and marks which is which: the ones
this session found by measuring against the machine before any lane ran, and
the ones the lanes found that this session had not. Every refutation of either
kind was re-run by the orchestrator before it was acted on.

## CONFIRMED

1. [cycle] That a ratchet measurement command running `pmat comply check` is a
   cycle by construction, with no guard on either side.
   - evidence: the operator's process tree, captured while it was running:
     `pmat comply ratchet` → `comply-count.sh CB-2110` → `pmat comply check` →
     `comply-count.sh CB-2110` → `pmat comply check` → … Every level repeats,
     and each fans out roughly fourfold because five metrics were declared.
   - evidence: 9,740 total processes, of which 5,781 were the script and 1,462
     were `pmat comply check`; 1-minute load average 379 with a peak of 3,026;
     CPU pressure some/avg10 at 83%; on a 48-core machine that had been idle.
     Killed by renicing the set to 19 and SIGKILL on the root.

## REFUTED

1. [timeout] That the ten-minute timeout this session saw was slowness.
   - evidence: a single `pmat comply check` takes 3 seconds, measured directly.
     Five such metrics cannot take more than ten minutes without recursion, and
     that arithmetic was available at the time. The approach was changed for
     the right reason — the recursion WAS inferred — but nothing was checked to
     see whether anything was still running, and it was.
   - corrected: nothing in the code, and nothing can be. Recorded here because
     the fix addresses the cycle and not the reading of the signal, and the
     second is the part that let it run. The nearest mechanical guard is the
     refusal at
     tests/falsification_comply_count_cannot_run_inside_itself.rs:59, which
     turns the re-entry into an exit 3 a reader cannot mistake for slowness.

2. [cap] That `ulimit -u 256` bounds a runaway cycle.
   - evidence: it killed the script's own fork immediately. `ulimit -u` is
     RLIMIT_NPROC, which on Linux is PER USER and counts THREADS. Measured on
     this account at the moment of the attempt: 226 processes, **2,352
     threads**. A fixed cap below the account's standing thread count is a gate
     that fails on any busy machine.
   - corrected: the cap is relative to a measured count, and
     tests/falsification_comply_count_cannot_run_inside_itself.rs:209 asserts
     the guarded script still measures on this machine — the case a fixed cap
     would fail.

3. [unit] That a cap derived from the PROCESS count is the right relative one.
   - evidence: `processes + 64` failed identically, because the kernel compares
     the limit against threads and the account was at 2,352 of them against 226
     processes — a factor of ten.
   - corrected: `ps -u <user> -L --no-headers`, the thread count, plus 512.
     Measured: one `pmat comply check` costs about 134 threads (2,352 at rest,
     2,486 at the peak), so 512 leaves room for the measurement and kills a
     cycle in its first level or two.

4. [sufficiency] That the sentinel alone closes this.
   - evidence: the sentinel stops the second ENTRY into the loop. It does not
     stop the DECLARATION that opens it, and the declaration is the half a
     future contributor writes. A `.pmat-ratchet.toml` naming this script would
     be committed, reviewed as config, and the guard would then turn every
     ratchet run into an exit 3 that reads as a broken gate rather than as a
     refused cycle.
   - corrected: tests/falsification_comply_count_cannot_run_inside_itself.rs:106
     refuses any committed ratchet config that names this script or runs
     `pmat comply`, so the loop cannot be re-declared directly.

5. [fail-open] That the process cap bounds a cycle whatever happens.
   - evidence: **refuted by the lanes, not by this session.** The script runs
     under strict mode, and there the capture `threads="$(ps … | wc -l)"`
     followed by `|| threads=""` leaves `threads` EMPTY when `ps` cannot answer, and the script then printed a warning on
     stderr and ran the measurement unbounded. Reproduced directly:
     `threads=[]`, and with a stubbed `ps` that exits 1 the old script measured
     anyway. In a gate whose caller captures stdout, a warning on stderr is
     indistinguishable from no cap at all — and this repository has now
     measured what no cap costs.
   - corrected: an unreadable count or a refused `ulimit` REFUSES the
     measurement, exit 4, saying it is refusing rather than proceeding, driven
     at tests/falsification_comply_count_cannot_run_inside_itself.rs:141 with a
     stubbed `ps` that exits 1. A red gate is the cheaper of the two failures
     by a factor of 9,740.

6. [ceiling] That CB-2115's ceiling could be lowered from 43 to 42.
   - evidence: **refuted by a lane**, and gate B was RED on this branch because
     of it: `GATE B FAIL … REGRESSION: CB-2115: 43 finding(s), ceiling 42`. The
     42 was read from the WORKING TREE mid-edit, between marking PMAT-521
     completed and committing it — a transient state no one else would ever
     see. The committed tree, which is the tree the gate reads, measures 43.
   - corrected: put back to 43, with the rule written into the baseline: a
     ceiling may only be lowered from a measurement of the COMMITTED tree. The
     ratchet catching its own author's error inside a day is the cheapest
     possible demonstration that it is not decorative.

7. [spelling] That the cap's falsifier tested the cap.
   - evidence: **refuted by all three lanes.** The case named for the cap
     asserted that the script's TEXT contained a `ps -L` invocation and a
     particular `ulimit` expression. A correct refactor would have failed it; a
     broken cap with the right words would have passed. That is the vacuity
     this repository's whole falsification discipline exists to refuse, written
     into a file named `falsification_*`.
   - corrected: the cap is DRIVEN at
     tests/falsification_comply_count_cannot_run_inside_itself.rs:141. A stubbed
     `ps` reporting one thread puts the cap at 513 against an account running
     2,424, and the case asserts the script's own fork fails — the only proof
     the `ulimit` took effect. A third case at
     tests/falsification_comply_count_cannot_run_inside_itself.rs:209 runs the
     real script on the real machine and asserts it still measures, which is
     what would have caught `ulimit -u 256`.
