# Implementation receipt — PMAT-522 — a ratchet measurement cannot eat the machine

verdict: PASS — the sentinel refuses a re-entry with exit 3 and no count on stdout, the process cap is measured in threads because that is what RLIMIT_NPROC compares, a committed ratchet config naming this script is refused by a test, and the measurement still works with all three in place. Four cases drive them.

orch_model: opus [A]   orch_class: code   orch_decision: admit   orch_basis: state
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=impl  route=self  w=100.00  basis=absent  (one script, one test file; a live hazard on a shared machine, fixed directly)
  ph2  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="bash scripts/ratchets/comply-count.sh CB-2110"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-522-recursion.log
  cmd="COMPLY_COUNT_ACTIVE=1 bash scripts/ratchets/comply-count.sh CB-2110"  claimed_exit=-  rerun_exit=3  log_path=docs/audits/logs/PMAT-522-recursion.log
  cmd="cargo test --test falsification_comply_count_cannot_run_inside_itself"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-522-recursion.log

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
   reason. An unreadable count or a refused `ulimit` is ANNOUNCED rather than
   swallowed: the measurement still runs and the reader is told it is unbounded.

3. **The declaration is refused too.** The cycle needs two halves — a script
   that runs comply, and a config that makes comply run the script. A test
   refuses any committed `.pmat-ratchet.toml` naming this script or running
   `pmat comply`.

## Falsification

`tests/falsification_comply_count_cannot_run_inside_itself.rs`, four cases: a
recursive invocation is refused with exit 3 and prints no count; the sentinel is
exported and tested before it is set, so the child process sees it and the first
call is not refused; no committed ratchet config names this script; and the cap
counts threads, is relative, and does not stop the measurement it guards — the
last of which runs the real script on the real machine.

## Two things this ticket does not do

- **pmat has no recursion guard either.** `pmat comply check` will run a
  measurement command that runs `pmat comply check`, at any depth. That half is
  upstream and is named in the ticket rather than fixed here.
- **`ulimit -u` is per user.** While this script runs, the cap applies to
  everything the account is doing, not only to this process tree. The headroom
  is sized for that (512 threads above the measured current count), but a second
  heavy job starting inside that window would see the same limit. A per-tree
  bound would need a cgroup, which is not available to this script.

## Also in this branch

PMAT-521's roadmap row says it shipped, which gate T's own T7 arm demanded the
moment PR #523 merged. And CB-2115 fell from 43 to 42 as the third ticket was
minted under the new convention, so its ceiling is lowered to what the tree
measures. Three tickets, three measurements, no ceiling raised.

IMPL-PMAT-522-RECEIPT-END
