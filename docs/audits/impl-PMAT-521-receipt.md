# Implementation receipt — PMAT-521 — gate B records what pmat 3.40's new checks measure, and refuses growth

verdict: PASS — gate B is green again on this branch, naming every count it measured: `ratchet CB-2110=49/49 CB-2111=49/49 CB-2112=34/34 CB-2114=34/34 CB-2115=43/43 held`. Nothing is skipped and nothing is waived: the five checks still report Fail inside `pmat comply check`, their counts are recorded with the instrument that produced them, and the gate is red by name the moment any of them grows.

orch_model: opus [A]   orch_class: code   orch_decision: admit   orch_basis: state
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=impl  route=self  w=100.00  basis=absent  (one gate script, one baseline, one test file)
  ph1.delegate  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]  (three lanes, 3 FAIL)
  ph2  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="bash scripts/dogfood/comply.sh"  claimed_exit=1(before)  rerun_exit=0  log_path=docs/audits/logs/PMAT-521-gate-b.log
  cmd="cargo test --test falsification_cb21xx_ratchet_holds_the_ceiling"  claimed_exit=101(lanes)  rerun_exit=0  log_path=docs/audits/logs/PMAT-521-gate-b.log
  cmd="bash scripts/ratchets/comply-count.sh CB-2110 … CB-2115"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-521-gate-b.log

## What happened, and what it is not

Gate B was GREEN when 1.28.0 was cut on 2026-09-10 and RED on main on
2026-09-11. pmat 3.40 put six checks in the comply roster this repository has
never satisfied:

| check | findings | made of |
|---|---|---|
| CB-2110 Spec Epics | 49 | NO-FRONT-MATTER 49 |
| CB-2111 Spec Reviews | 49 | UNJUDGEABLE 49, every one downstream of CB-2110 |
| CB-2112 Ticket Linkage | 34 | NO-ISSUE 24, TAIL-MISMATCH 10 |
| CB-2113 Commit Traceability | branch-local | — |
| CB-2114 Release Binding | 34 | NO-RELEASE 34 |
| CB-2115 Roadmap Coherence | 43 | ORPHAN-ROADMAP, ORPHAN-GITHUB, DRIFT |

**Not one is a regression from any ticket in the 1.29.0 window.** They are a
new obligation arriving with a tool upgrade, over a backlog that was always
there and that nothing had ever asked about.

The doctrine forbids a skip. It does not forbid RECORDING what is true and
refusing to let it get worse — that is what CB-200 has done one arm above since
PMAT-201, under the rule the dogfood protocol already states: *record a dated
baseline in the crate rather than weakening a gate*.

## The design, and the two things that make it a ratchet rather than a waiver

`scripts/ratchets/cb21xx-baseline.json` records a ceiling per check with the
instrument that produced it. Arm 1 exempts exactly five ids, written out rather
than matched by prefix. Arm 7 enforces the ceilings **from the same comply run
Arm 1 made** — one measurement, not two that can disagree.

1. **The exemption list and the ceiling map are one list, checked by the GATE.**
   A check in the array and not in the baseline would be exempt from Arm 1 and
   ignored by Arm 7 — waived entirely, by nobody, in two files that each look
   correct alone. A review lane found this. The arm now refuses the mismatch
   itself; a test alone would only have caught it in CI.
2. **Every unmeasurable outcome fails closed and SAYS which one it is.** A check
   that left the roster, a check reporting Fail with no count, a second entry
   under one id, comply output that does not parse, a baseline that does not
   read. A ceiling that only ever looks upward greets a rotted predicate as
   perfection.

## The convention that stops it being a treadmill

A ratchet whose ceiling must rise on every new ticket is not a ratchet. Two
tickets were minted for this release and **the counts did not grow** — CB-2115
fell by one. The three rules that make that true were measured, not assumed:

1. **The GitHub issue is created FIRST and the roadmap id's tail IS the issue
   number.** `PMAT-520` for issue 520. Measured: with the old convention
   (`PMAT-242` against issue 520) CB-2112 reads **36** against a ceiling of 34 —
   one TAIL-MISMATCH per new ticket. With the new one it reads **34**.
2. **The issue goes on the release MILESTONE.** `release:` is projected from it,
   so an item with no milestone is a NO-RELEASE — again one per new ticket.
   Milestone `1.29.0` was created for this release and both issues are on it.
3. **`release:` is the BARE version string**, `1.29.0` and not `v1.29.0`, and
   the milestone's title must equal it exactly. A `v` prefix is its own finding
   class, PREFIXED, which is how this was found: CB-2114 read 36 with `v1.29.0`
   and 34 without.

This is the answer to the ticket's open question rather than an assumption:
the 241 historical ids stay as they are, and every id from here follows its
issue.

## `pmat work sync` is the documented writer of `release:` and must not be used here

`pmat work sync --direction github-to-yaml` is what pmat's own message names as
the only writer of that field. Running it **dropped 20 of the roadmap's 57
`kind:` fields and reflowed 369 lines** (measured, then reverted). `kind:` is
what `paiml-implement`'s kind-gate reads, so the sync silently removes a gate's
input. Both fields are written textually instead, from the milestone the issues
are actually on. This repository already knew the rule for this file — *edit it
textually* — and this is the second measurement of why.

## pmat's own ratchet cannot serve these, and that is a defect worth reporting

`.pmat-ratchet.toml` (CB-2102) is the right-shaped mechanism and was tried
first. A metric whose `command` runs `pmat comply check` **re-enters CB-2102,
which runs every metric's command again, without bound.** Measured: a single
comply run is 3 seconds, and five such metrics did not finish in ten minutes.
The file was removed before it was ever committed, so no CB-2102 "deleted"
finding was created.

## What the lanes found

Three lanes, all FAIL, six refutations. Every one was re-run here before it was
accepted and every one reproduced:

1. **A second entry under one id overwrites the first**, so a later Pass could
   replace an earlier Fail. Today's roster carries 172 checks with no duplicate
   id, so it is a latent hazard rather than an observed defect — and a ratchet
   that can lose a finding to dictionary assignment is not one. Now UNMEASURED.
2. **A check exempt but not ratcheted is waived by both arms.** The worst
   outcome this design had. The arm refuses the mismatch itself now.
3. **`sys.exit("message")` writes to stderr and `$( )` captures stdout**, so the
   arm went red with an EMPTY detail — `the CB-2110..CB-2115 ratchet (Arm 7): `
   and nothing after the colon. All three lanes found it. Every diagnostic is a
   `print` on stdout now and the substitution takes `2>&1` as well, so a
   traceback still reaches the reader.
4. **Three of the five counts could not be measured by the lanes**, because this
   host was inside a GitHub secondary rate limit. That is a real property, not a
   lane error: CB-2112, CB-2114 and CB-2115 each take a `gh` snapshot, so on a
   host that cannot reach GitHub those three are UNMEASURED — which the arm
   reports as a failure rather than a pass.
5. **The falsifier's harness defined `CB21XX_BASE` itself and merged stderr into
   stdout**, which would have hidden finding 3 exactly. It now extracts the
   array, the path and the arm from the script, and reads `$cb21xx` alone.
6. **CB-2113 was NOT satisfied on this branch.** Two lanes measured it failing.
   True, and mine: the branch's own `Pmat-Ticket` named a ticket whose roadmap
   row was on a different branch. Fixed, and then again when the id changed.

## Falsification

`tests/falsification_cb21xx_ratchet_holds_the_ceiling.rs`, nine cases over
synthetic comply output: at the ceiling is green and names every number; one
finding over is a REGRESSION naming the check, the count and the ceiling; fewer
is green; a check that has LEFT the roster is UNMEASURED rather than zero; a
check reporting Fail with no count is UNMEASURED; a check exempt but not
ratcheted is refused BY THE GATE; every verdict the arm reaches arrives with
words on stdout; the two lists are the same set; and CB-2113 is in neither.

Gate B itself, before and after, in `docs/audits/logs/PMAT-521-gate-b.log`.

## Gaps, named

- The backlog is recorded, not fixed. 49 specifications still have no
  front-matter and no review, 24 roadmap items still have no issue, 10 ids
  still do not match their issue, and 34 items have no release binding. The
  ceilings say so out loud and refuse growth; they do not do the work.
- CB-2111 cannot fall except by fixing CB-2110 first, and CB-2114's NO-RELEASE
  half is bounded below by CB-2112's NO-ISSUE half. Two of the five ceilings are
  therefore not independently movable, which is recorded in the baseline.
- Three of the five counts need GitHub. On an offline host gate B is UNMEASURED
  for them and fails, which is correct and also means this gate cannot run in a
  clean room without a token.
- `pmat comply ratchet` could not be used and the reason is a pmat defect that
  has not been reported upstream from here.

IMPL-PMAT-521-RECEIPT-END
