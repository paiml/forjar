# PMAT-522 — adjudicated claims

**No agy round was dispatched.** The defect was a live hazard on a shared
machine, already measured and filed by the operator, and the fix was written
against those measurements directly. The adjudication below is this session's
own claims, each tested against what the machine actually did — which is the
stronger instrument here than three lanes reading a diff, and is recorded as
what it is rather than dressed up as a quorum.

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
   - corrected: nothing in the code. Recorded here because the fix addresses
     the cycle and not the reading of the signal, and the second is the part
     that let it run.

2. [cap] That `ulimit -u 256` bounds a runaway cycle.
   - evidence: it killed the script's own fork immediately. `ulimit -u` is
     RLIMIT_NPROC, which on Linux is PER USER and counts THREADS. Measured on
     this account at the moment of the attempt: 226 processes, **2,352
     threads**. A fixed cap below the account's standing thread count is a gate
     that fails on any busy machine.
   - corrected: the cap is relative to a measured count.

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
   - corrected: a test refuses any committed ratchet config that names this
     script or runs `pmat comply`, so the loop cannot be re-declared.
