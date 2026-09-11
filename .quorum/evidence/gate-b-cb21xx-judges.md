# PMAT-521 — adjudicated claims

One round of three sandboxed agy quorum lanes: 3/3 FAIL, and every one of the
six claims put to them REFUTED. Every refutation was re-run by the orchestrator
before it was acted on, and every one reproduced.

A round where every claim falls is not a failed round. It is the round doing
its job on a change whose author had reviewed it only against himself.

## CONFIRMED

(none — see above)

## REFUTED

1. [waiver] That a check exempt from Arm 1 is necessarily owned by Arm 7.
   - evidence: Arm 1 exempts by the `CB21XX` array and Arm 7 judges by the
     baseline's `ceiling` keys, and nothing compared the two. A check named in
     the array and absent from the baseline is skipped by Arm 1 because the
     array names it, and skipped by Arm 7 because the ceiling map does not —
     waived entirely, by nobody, in two files that each read correctly alone.
     A test asserted the two sets matched, but a test runs in CI and the hole
     is in the gate.
   - corrected: the arm is passed the array and refuses a mismatch itself,
     UNMEASURED, naming the check that is exempt and unowned.
     `tests/falsification_cb21xx_ratchet_holds_the_ceiling.rs:1` drives it with
     a sixth id in the list and no ceiling for it.

2. [mute] That Arm 7 says which unmeasurable state it is in.
   - evidence: refuted by all three lanes. `sys.exit("message")` writes to
     stderr; `cb21xx="$( … )"` captures stdout; `fail "… ${cb21xx}"` then
     printed `the CB-2110..CB-2115 ratchet (Arm 7): ` and stopped. Reproduced
     directly: a command substitution around a python that exits with a string
     yields an empty capture and exit 1. A gate that is red and mute is worse
     than one that is green and wrong, because nobody can act on it.
   - corrected: every diagnostic is a `print` on stdout followed by a numeric
     exit, and the substitution takes `2>&1` so a traceback still reaches the
     reader.
     `tests/falsification_cb21xx_ratchet_holds_the_ceiling.rs:1` asserts that
     four different verdicts all arrive with words.

3. [overwrite] That a check's count is read once.
   - evidence: two lanes found that `seen[cid] = …` lets a second roster entry
     under one id replace the first, so a later Pass could erase an earlier
     Fail. Measured on the live roster: 172 checks, no duplicate id — so this
     is a latent hazard rather than an observed defect, and a ratchet that can
     lose a finding to dictionary assignment is not a ratchet.
   - corrected: a second entry under one id is UNMEASURED and named, rather
     than silently replacing the first.

4. [harness] That the falsifier measures the arm rather than its own copy of it.
   - evidence: the harness defined `CB21XX_BASE` itself and merged stderr into
     stdout. The first would have passed over a renamed baseline; the second
     would have hidden refutation 2 exactly, which is the defect three lanes
     found and the test did not. An earlier ticket in this same session was
     refuted on the same pattern, which is why the brief asked about it.
   - corrected: the harness extracts the array, the path AND the arm out of
     `scripts/dogfood/comply.sh`, discards stderr, and reads `$cb21xx` alone —
     the exact value the gate interpolates into its verdict.

5. [unmeasured] That the five counts could be re-measured independently.
   - evidence: refuted 3/3, and it is a property of the gate rather than a lane
     error. CB-2112, CB-2114 and CB-2115 each take a `gh paiml/forjar` snapshot,
     so on a host that cannot reach GitHub — which this one could not, being
     inside a secondary rate limit — those three are UNMEASURED. Two lanes did
     confirm CB-2110 and CB-2111 at 49.
   - corrected: nothing to correct in the code; the arm already reports an
     unreadable count as a failure rather than a pass. Recorded as a named gap:
     gate B cannot run in a clean room without a GitHub token.

6. [traceability] That CB-2113 was satisfied on this branch.
   - evidence: refuted 3/3 and true. The branch's own commit carried
     `Pmat-Ticket: PMAT-243` while that row lived on a different branch, so the
     check reported `PMAT-243 is not in docs/roadmaps/roadmap.yaml`. The
     reading of CB-2113 as branch-local and satisfiable was right; the claim
     that this branch satisfied it was not.
   - corrected: the roadmap rows are on this branch, and the trailer now names
     `PMAT-521` — the id changed when the convention did, which is itself the
     second half of this ticket.
