# Implementation receipt — PMAT-535 — a branch names the ticket its work claims

verdict: PASS — the quorum gate refuses, at push time, a branch that names a ticket no commit being pushed claims. The branch name is read by three gates and was checked by none; the defect it produced is still visible on main, where PMAT-520 carries two release labels and nobody put the second there by hand.

orch_model: opus [A]   orch_class: code   orch_decision: admit   orch_basis: state
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=impl  route=self  w=100.00  basis=absent  (one gate arm, six cases)
  ph1.delegate  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]
  ph2  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="cargo test --test falsification_a_branch_names_its_own_ticket"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-535-branch-names-its-ticket.log
  cmd="cargo test --test falsification_quorum_gate_{reads_the_pushed_ref,has_a_triage_shape} --test falsification_quorum_anchors_release_shaped"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-535-branch-names-its-ticket.log
  cmd="bash scripts/quorum-gate.sh HEAD"  claimed_exit=-  rerun_exit=1(receipt arm, as expected)  log_path=docs/audits/logs/PMAT-535-branch-names-its-ticket.log
  cmd="bashrs lint scripts/quorum-gate.sh"  claimed_exit=-  rerun_exit=1  log_path=docs/audits/logs/PMAT-535-branch-names-its-ticket.log

## bashrs, stated as measured rather than as a slogan

`bashrs lint scripts/quorum-gate.sh` exits **1** on this branch. It also exits 1
on `origin/main`. bashrs returns non-zero whenever it has anything at all to
say, so its exit code carries no information about this change; the number that
does is ERRORS, which is **0** on both sides.

The arm adds four warnings. All four are one artifact: bashrs parses the
`die` message — a double-quoted string spanning seven lines — as shell source.
Two SC2086 fire on `$branch_id` at columns inside that string, BRS0023 fires on
the English word "read" in the sentence *"The branch name is read by gate A"*,
and SC2101 fires on `*[![:space:]]*`, which is a case PATTERN and is the
correct spelling. The log lists each with its column.

## What the branch name decides

| gate | resolves from it |
|---|---|
| A | `docs/audits/impl-<ticket>-receipt.md`, the receipt path |
| E | `.quorum/<slug>.json`, this receipt's own filename |
| T | the RELEASE WINDOW a merged PR belongs to |

Three gates read one unvalidated string.

## What it produced

PR #532 was pushed from `PMAT-520-book-v1.29.0`. Its merge commit `b4719737`
carries `Pmat-Ticket: PMAT-531`; its title says PMAT-531; its receipt says
PMAT-531.

Every window arm resolved it to **PMAT-520** — which had already SHIPPED in
1.29.0 — so gate T demanded `release:v1.30.0` on it, `release-goal.sh sync`
obligingly added the label, and PMAT-520 now claims two releases on main. The
ticket that actually owned the work was invisible to all three gates.

**Nobody added that label by hand.** The gate demanded it, and the gate was
doing exactly what it was told.

## The rule, and what it is not

If the branch names a ticket, **some commit being pushed must claim it**.

Not "every trailer equals the branch's id" — a branch legitimately carries
commits for more than one ticket, which this repository does on every release
cut. What it must not do is name one that none of them claims.

And not "every branch names a ticket": `fix/ci-lint` is the naming this
repository's own guidelines suggest for work with no ticket, and refusing it
would be this arm inventing a requirement nobody asked for.

**Only when the commits claim something.** A branch whose commits carry no
`Pmat-Ticket` at all is a different defect with a different owner — the
commit-msg hook refuses it, CB-2113 refuses it again — and an arm that also
refused it would be reporting someone else's finding in its own words. That
distinction surfaced by breaking a pre-existing fixture, which builds its
commits without hooks.

At push time, because after the push the name is in three gates' arithmetic and
a merge commit cannot be renamed.

## The trailer is read the way pmat reads it

`git log --format='%(trailers:key=Pmat-Ticket,valueonly=true)'` reads trailers
from the **last paragraph only**. A message written with several `-m` flags —
each of which becomes its own paragraph — has a `Pmat-Ticket:` line git does
not consider a trailer at all.

**Every commit in this session was in that shape** until this ticket measured
it: `Pmat-Ticket`, `Claude-Session` and `Co-Authored-By` each in their own
paragraph, and `%(trailers)` returning nothing for all of them. pmat's CB-2113
and this repository's commit-msg hook both match the LINE wherever it appears,
which is why they all passed.

So the arm matches the line too. A gate that disagreed with the hook and the
comply check about what a trailer is would refuse commits they accept. The case
that pins this asserts git returns nothing for that shape **first**, so it
cannot quietly stop testing anything if git's parser changes.

This branch's own commits carry the trailer block as one paragraph at the end,
which is the shape git reads — the first commits in this session that do.

## Falsification

`tests/falsification_a_branch_names_its_own_ticket.rs`, six cases: the exact
shape PR #532 was in, red and naming the ticket, the claim, and what the name
decides; a branch naming its own ticket, which must reach the RECEIPT arm
(asserted by name, so a branch check that refused everything could not pass
it); a branch carrying three tickets and naming one; a branch with no ticket id;
a branch whose commits claim nothing; and the trailer-shape case above.

## Gaps, named

- The arm runs at push time. A branch already merged keeps its name, and
  PMAT-520's second label stays until someone decides whether to strip it —
  the ledger row is the record of what shipped, and the label is a gate's
  demand rather than a claim anyone made.
- It reads the FIRST `PMAT-<n>` in the branch name. A branch named
  `fix-PMAT-1-and-PMAT-2` is judged on PMAT-1 alone.
- Nothing checks the same agreement for the PR TITLE, which the window resolver
  also reads.

IMPL-PMAT-535-RECEIPT-END
