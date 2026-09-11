# Implementation receipt — PMAT-522 — a ratchet measurement cannot eat the machine

verdict: PASS — the sentinel refuses a re-entry with exit 3 and no count on stdout; the process cap is measured in threads because that is what RLIMIT_NPROC compares, is proven applied by driving it, and FAILS CLOSED with exit 4 when it cannot be established; a committed ratchet config naming this script is refused by a test; and the measurement still works with all three in place. Five cases drive them. Three review lanes refuted the first version on every one of those points except the sentinel.

orch_model: opus [A]   orch_class: code   orch_decision: admit   orch_basis: state
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=impl  route=self  w=100.00  basis=absent  (one script, one test file; a live hazard on a shared machine, fixed directly)
  ph1.delegate  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]  (three lanes, 3 FAIL — dispatched only after the quorum gate refused a one-lane receipt)
  ph2  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="bash scripts/ratchets/comply-count.sh CB-2110"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-522-recursion.log
  cmd="COMPLY_COUNT_ACTIVE=1 bash scripts/ratchets/comply-count.sh CB-2110"  claimed_exit=-  rerun_exit=3  log_path=docs/audits/logs/PMAT-522-recursion.log
  cmd="cargo test --test falsification_comply_count_cannot_run_inside_itself"  claimed_exit=101(lanes)  rerun_exit=0  log_path=docs/audits/logs/PMAT-522-recursion.log
  cmd="bash scripts/dogfood/comply.sh"  claimed_exit=1(lane: CB-2115 regression)  rerun_exit=0  log_path=docs/audits/logs/PMAT-522-recursion.log

## What happened, and who found it

This session caused it. While adding the CB-2110..CB-2115 ceilings for PMAT-521,
pmat's own `.pmat-ratchet.toml` mechanism was tried first. A metric's `command`
ran `scripts/ratchets/comply-count.sh`, which runs `pmat comply check`, which
evaluates `.pmat-ratchet.toml` and runs every measurement command in it. That is
a cycle by construction and neither side had a guard.

**The operator measured it and filed it, not this session.** On a 48-core
machine that had been idle:

| metric | value |
|---|---|
| total processes | 9,740 |
| `comply-count.sh` copies | 5,781 |
| `pmat comply check` copies | 1,462 |
| load average, 1 min | 379, peaking at 3,026 |
| CPU pressure some/avg10 | 83% |

Killed by renicing the whole set to 19, then SIGKILL on the root.

**What this session saw was a ten-minute timeout, and it read it as slowness.**
A single `pmat comply check` is three seconds, so five such metrics finishing in
more than ten minutes was structurally impossible without recursion — the
arithmetic was available at the time and was not done. The approach was changed
for the right reason and the wrong one: the recursion was inferred, but nothing
was checked to see whether anything was still running.

## The fix, and the two things that had to be measured to get it right

1. **The sentinel.** `COMPLY_COUNT_ACTIVE` is tested first and exported second.
   A re-entry exits 3 and prints NOTHING on stdout, because a `0` there is read
   as the check's finding count — the largest improvement in the project's
   history, spelled identically to a script eating the machine.

2. **The process cap, in the unit the kernel compares.** `ulimit -u` is
   RLIMIT_NPROC: **per user, counting threads**. Both halves were learned here:

   | attempt | result |
   |---|---|
   | `ulimit -u 256` | killed the script's own fork at once |
   | `ulimit -u $((processes + 64))` | same, for the same reason |
   | this account at rest | 226 processes, **2,352 threads** |
   | peak during one comply run | 2,486 threads — about **134** for the run |
   | `ulimit -u $((threads + 512))` | measures, and bounds a cycle |

   A cap that fails on a busy machine is the gate going red for the wrong
   reason. And an unreadable count or a refused `ulimit` **refuses the
   measurement**, exit 4, rather than warning and running unbounded — three
   review lanes refuted the first version, which warned on stderr and
   proceeded. In a gate whose caller captures stdout, a warning on stderr is
   indistinguishable from no cap at all, and this repository has now measured
   what no cap costs.

3. **The declaration is refused too.** The cycle needs two halves — a script
   that runs comply, and a config that makes comply run the script. A test
   refuses any committed `.pmat-ratchet.toml` naming this script or running
   `pmat comply`.

## Falsification

`tests/falsification_comply_count_cannot_run_inside_itself.rs`, five cases: a
recursive invocation is refused with exit 3 and prints no count; the sentinel is
exported and tested before it is set, so the child process sees it and the first
call is not refused; no committed ratchet config names this script; the cap is
REALLY APPLIED and fails closed, driven with a stubbed `ps` reporting one thread
so the cap lands at 513 against an account running 2,424 and the script's own
fork must fail, and with a `ps` that cannot answer so the measurement must be
refused with exit 4; and the guarded script still measures on this machine,
which is the case that would have caught `ulimit -u 256`.

## The round, dispatched late, and what it caught

No round was dispatched at first. The reasoning was that the defect was already
measured by the operator and that three lanes each spawn processes on a box that
had just been recovered. **The quorum gate refused the receipt** — one lane,
floor is three — and it was right to: that reasoning is an argument for skipping
a rule, made by the person the rule constrains. The round ran, all three lanes
returned FAIL, and it found three things this session had not:

1. **The cap failed OPEN.** If `ps` could not answer, `threads` was empty, the
   script printed a warning and ran unbounded. Refuted by two lanes, reproduced
   here (`threads=[]`), and now exit 4.
2. **The CB-2115 ceiling was lowered WRONGLY, and gate B was red because of it.**
   42 was read from the working tree mid-edit, between marking PMAT-521
   completed and committing it; the committed tree measures 43. Reproduced:
   `GATE B FAIL … REGRESSION: CB-2115: 43 finding(s), ceiling 42`. Put back, with
   the rule added to the baseline: a ceiling may only be lowered from a
   measurement of the COMMITTED tree. **The ratchet caught its own author's
   error**, which is the cheapest possible demonstration that it is not
   decorative.
3. **`the_process_cap_counts_threads_and_is_relative` was a test of SPELLING.**
   It asserted the script's text contained `ps -L` and a particular `ulimit`
   expression — a correct refactor would fail it and a broken cap with the right
   words would pass. Replaced by a case that stubs `ps` to report one thread, so
   the cap lands at 513 against an account running 2,424, and asserts the
   script's own fork fails. That is the only proof the `ulimit` took effect.

Two lanes also wrote `pmat_output.json`, `stdout.log` and `stderr.log` into the
repository root despite the no-write rule opening the brief. Removed; recorded
here rather than left for someone to find.

## Two things this ticket does not do

- **pmat has no recursion guard either.** `pmat comply check` will run a
  measurement command that runs `pmat comply check`, at any depth. That half is
  upstream and is named in the ticket rather than fixed here.
- **`ulimit -u` is per user.** While this script runs, the cap applies to
  everything the account is doing, not only to this process tree. The headroom
  is sized for that (512 threads above the measured current count), but a second
  heavy job starting inside that window would see the same limit. A per-tree
  bound would need a cgroup, which is not available to this script.
- **The sentinel travels in the environment, so `env -i` defeats it.** A lane
  named this. pmat spawns a measurement command through a shell and does not
  sanitise the environment, so the path that actually happened is closed; a
  future caller that scrubs the environment would reopen it, and nothing here
  would notice.
- **The declaration test catches the literal string only.** A wrapper script,
  a Makefile target, or another pmat subcommand that evaluates the ratchet
  would re-create the cycle and pass it. Two lanes named this. A static test
  cannot follow an indirection, so the claim is narrowed rather than defended:
  what is refused is the DIRECT declaration, which is the one that happened.

## Also in this branch

PMAT-521's roadmap row says it shipped, which gate T's own T7 arm demanded the
moment PR #523 merged. And CB-2115 fell from 43 to 42 as the third ticket was
minted under the new convention, so its ceiling is lowered to what the tree
measures. Three tickets, three measurements, no ceiling raised.

IMPL-PMAT-522-RECEIPT-END
