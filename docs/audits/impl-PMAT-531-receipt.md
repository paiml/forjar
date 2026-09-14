# Implementation receipt — PMAT-531 — the v1.29.0 booking, and two false records

verdict: PASS — the row is byte-identical to `release-goal.sh window v1.29.0` and is the FIRST to name the paiml/forjar-cookbook commit a release was qualified against; `next` is v1.30.0 at the cut instant plus `cadence_days`; the two false records three lanes found are corrected, the two gates that let them stand are filed, and the ratchet's own "may only shrink" rule is enforced for the first time by the raise that exposed it.

orch_model: opus [A]   orch_class: code   orch_decision: admit   orch_basis: release
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=impl  route=self  w=100.00  basis=absent  (the booking, by `release-goal.sh cut`)
  ph1.book  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]  (three lanes, 3 FAIL)
  ph2  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="bash scripts/release-goal.sh window v1.29.0"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-531-booking.log
  cmd="bash scripts/dogfood/comply.sh with the justification block removed"  claimed_exit=-  rerun_exit=1  log_path=docs/audits/logs/PMAT-531-booking.log
  cmd="cargo test --test falsification_cb21xx_ratchet_holds_the_ceiling"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-531-booking.log

## The row

```
  - tag: v1.29.0
    cut: 2026-09-11T19:55:10Z
    prs: [509, 510, 511, 513, 514, 515, 516, 517, 518, 519, 523, 524, 527]
    tickets: [PMAT-227 … PMAT-522]   (sixteen)
    dogfood: docs/audits/dogfood-1.29.0-receipt.md
    crux: docs/audits/crux-1.29.0.md
    cookbook: 7c100454e8f9fb2b5b13f076b773f808071d5e08
```

**The first `cookbook:` field any release has carried.** Every release before
this one claimed to have been dogfooded against a cookbook nobody named.

The row names thirteen PRs where the CHANGELOG says twelve, and both are right:
the cut's own PR #527 has merged now, so the window includes it. That is exactly
why gate T's T9 arm reads the CHANGELOG only while a cut is in flight. Two lanes
checked that reasoning independently and confirmed it.

## The two false records

Three lanes, unanimous, and both reproduce:

1. **PMAT-240 read `status: completed`** with thirteen pipelines of its own
   shape still in `scripts/` and NEITHER acceptance criterion met. It was
   `planned` when minted at `f98b388d` and `completed` by `c5e4aaa3` — flipped
   during unrelated work, with no commit saying so.

   Corrected **textually and deliberately**: `pmat work edit` refuses
   `Completed -> Planned` and `Completed -> InProgress` as invalid transitions.
   A lifecycle rule that keeps a false `completed` in the record is the wrong
   rule for this case, and a row claiming eighteen pipelines were fixed while
   thirteen are still there is the exact half-true record PMAT-235 and PMAT-236
   exist to refuse. It now has issue #530 and a release binding, neither of
   which it had ever had.

2. **PMAT-233 reads `cancelled`** and `release-goal.sh cut` moved its `release:`
   label into the next window. A cancelled ticket belongs to no release, so that
   is a treadmill: every future cut carries it one window further while gate T
   counts it as open work. Label removed.

**Both gates that should have caught them are filed rather than patched here**,
because both have a judgement in them:

- **PMAT-528 (#528)** — T7 is one-directional. It refuses a shipped ticket that
  says `planned` and permits one that says `completed` and never shipped. The
  predicate cannot be "every completed ticket is named by a release": a ticket
  can be closed as obsolete, folded into another, or finished before the floor.
- **PMAT-529 (#529)** — `cut` and `sync` label the open window from the roadmap
  rows without asking what state a row is in.

## The ratchet caught its own author, in the other direction

Correcting the false `completed` turned a closed row into an open one, and open
rows are what CB-2112 and CB-2114 count. Both went one over their ceilings and
gate B went red. **The growth was real**, so:

- CB-2114 is back at 34 **by real work** — PMAT-240 has an issue and a release
  binding now, which removes its NO-RELEASE finding;
- CB-2112 is **raised to 35 with a written justification**, because the row keeps
  its historical id and PMAT-240 against issue 530 is a TAIL-MISMATCH no sync can
  rename away.

That raise is legitimate. **It is also exactly the shape an illegitimate one
has**, and the only thing separating them was a sentence nobody was required to
write: `"MAY ONLY SHRINK"` is stated in the baseline's own text and the ceilings
are a JSON file, so raising one is a one-character edit that turns every future
regression green. The `justification` field has been in the schema since it was
written and nothing read it.

So this branch makes the rule real. A ceiling above the **base branch's** — a
tree the author of a raise did not write — is refused unless the baseline
carries `justification.<CHECK>` for that check. Lowering needs nothing.

## Falsification

`a_ceiling_may_not_rise_without_a_written_reason` drives six outcomes over a
temp repository: raised without a reason is refused naming the check and both
numbers; raised WITH one passes and SAYS so, because a silent allow makes the
reason decorative; whitespace is not a reason; lowering is green and says
nothing rose; a reason for a check that did not rise does not license a
different one that did; and a baseline the base does not carry is a new file,
which declares rather than raises.

Measured on the real gate: removing the `justification` block from this
branch's own baseline turns gate B red with
`RAISED WITHOUT A REASON: CB-2112 raised 34 -> 35 with no justification.CB-2112`.

## Gaps, named

- PMAT-240's correction is a textual edit against a lifecycle the tool
  enforces. Whether that lifecycle should admit a documented correction is part
  of PMAT-528 rather than settled here.
- The raise arm compares against `origin/main`. On a branch cut from something
  else it compares against the wrong tree, and `COMPLY_BASE_REF` exists for a
  fixture rather than for that case.
- Thirteen SIGPIPE sites remain. PMAT-240 is `planned` again with an issue, and
  the count in its title is corrected from eighteen to thirteen.

IMPL-PMAT-531-RECEIPT-END
