# Implementation receipt — PMAT-227 — book v1.28.0 in the release ledger: the row from the tag, the open goal v1.29.0, and gate T green on main again

verdict: PASS — the v1.28.0 row `scripts/release-goal.sh cut` wrote is byte-identical to `scripts/release-goal.sh window v1.28.0`, its cut is the tag's own creatordate, and `next.due` is that instant plus exactly two days; gate T passes over main's HEAD with this ledger (6 tagged releases, 26 tickets carrying their tag) and is red on the branch only on the bypassed-review arm every ticket meets before its merge; the diff stays inside the `kind: triage` rail PMAT-226 widened; three lanes judged nine claims and refuted two, one a real error in the claim text (filed as PMAT-229) and one a lane misread that does not reproduce. Not claimed here: the GitHub release, whose workflow was still building binaries — its verdict is `docs/audits/release-1.28.0-receipt.md`.

orch_model: opus [A]   orch_class: triage   orch_decision: admit   orch_basis: release
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=orchestration  route=self  w=100.00  basis=absent  (the booking itself: `release-goal.sh cut`, one hand-removed line, two roadmap rows)
  ph2  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]  (delegate quorum width 3, read-only)
  ph3  class=orchestration  route=self  w=100.00  basis=absent  (receipts, artifact, push, PR)

verification:
  cmd="the booked row against scripts/release-goal.sh window v1.28.0, comment lines dropped from both"  claimed_exit=0(lanes)  rerun_exit=0  log_path=docs/audits/logs/PMAT-227-row-equals-window.log  sha256=652a108350916025
  cmd="scripts/dogfood/tagged.sh in a worktree at main (cdcc0e80) with this ledger, DOGFOOD_RELEASES_REF=worktree"  claimed_exit=0(lanes)  rerun_exit=0  log_path=docs/audits/logs/PMAT-227-gate-T-main.log  sha256=155d2bdc0b575031
  cmd="scripts/dogfood/tagged.sh on the branch (red on the bypassed-review arm alone)"  claimed_exit=1(lanes)  rerun_exit=1  log_path=docs/audits/logs/PMAT-227-gate-T-branch.log  sha256=e3b1b72580883eff
  cmd="scripts/release-goal.sh show and sync --check, on the branch and at main with this ledger"  claimed_exit=2(lanes)  rerun_exit=2/0  log_path=docs/audits/logs/PMAT-227-show.log  sha256=e231092ab12c8211

## Identity

| field | value |
|---|---|
| ticket | PMAT-227, kind:triage, orch:fable (`orch-basis:release`), roadmap row minted with `pmat work add`, patched textually, label `release:v1.29.0` applied at mint time |
| issue | none filed — the booking half of a release cut |
| branch | `PMAT-227-book-v1.28.0` on main `cdcc0e80`, which is the commit `v1.28.0` names |
| commits | `e82b6cd2` the row and the open goal; `a47ca249` the PMAT-228 row; then PMAT-229, this receipt, the evidence and the artifact |
| session | the fifth ticket of one Claude session. `goal.sh set` refused it (R-5, naming PMAT-223); it proceeded on the user's verbatim instruction quoted in PMAT-226's roadmap row |
| model | the session model changed mid-ticket: `model-gate.sh` REFUSED this ticket under fable (a `kind: triage` row admits opus only) and ADMITTED it under opus (`model=opus class=opus decision=admit basis=file`). Both runs are recorded in `.quorum/evidence/book-1.28.0-pmat.md`; no lane was dispatched under the refusal |

## Why this is a second PR and not part of the cut

A tag's ledger row cannot be written before the tag exists: its `cut` field is the tag's creatordate, and gate T's T2 refuses a row naming a tag `origin` does not carry. So every release under PMAT-225's cadence is two PRs — the cut, which bumps and documents, and the booking, which writes down what the tag turned out to be. The booking is classify-and-link with no code in it, which is why PMAT-226 widened the committed-quorum gate's `kind: triage` rail to admit `docs/roadmaps/releases.yaml`: before that, this PR could only have been pushed with a waiver.

Between the tag and this merge, gate T is red on main by design (no row for v1.28.0, `next.tag` not above the newest tag). The daily `release-goal` workflow would open its issue if that were still true at 05:00 UTC; closing the window fast is the point.

## What was measured

- The row: cut `2026-09-10T16:07:14Z` (the annotated tag's creatordate, 18:07:14+02:00 local), prs `[493, 494, 496, 498, 500, 502, 504, 505, 507, 508]`, tickets `[PMAT-215, PMAT-217, PMAT-219, PMAT-220, PMAT-221, PMAT-222, PMAT-223, PMAT-224, PMAT-225, PMAT-226]`, dogfood and crux receipts that exist at HEAD. Byte-identical to the tool's own `window v1.28.0`.
- The open goal: `next.tag v1.29.0`, `next.due 2026-09-12T16:07:14Z` — the cut plus exactly 172800 seconds, the arithmetic T6 re-derives.
- Labels, both directions: ten rows carry `release:v1.28.0` and they are exactly the ten the row names; PMAT-227, PMAT-228 and PMAT-229 carry `release:v1.29.0`, applied when each row was minted rather than at the next cut. That is the continuous half of the brief: a ticket is linked to its release when it is opened, not when it ships.
- Gate T over main's HEAD with this ledger: `GATE T PASS 6 tagged release(s) since v1.25.0 reconcile with git and GitHub and 26 ticket(s) carry their tag; 0 of 0 PR(s) merged since v1.28.0 carry release:v1.29.0; due 2026-09-12T16:07:14Z, 47h left`.

## Two defects this booking found, both filed

- **PMAT-228** — `release-goal.sh cut` pasted the window library's `#490 … is inside v1.27.0 (the previous release) — not counted` note into the ledger, because the note goes to stdout and `cut` captures stdout. The leading `#` made it a YAML comment, so nothing broke and nothing complained; it was removed by hand here and the fix belongs with its fixture test.
- **PMAT-229** — `release-goal.sh show` refuses to render the goal at all on any branch whose own commits are not yet in a merged PR, which is every branch while work is happening. The refusal is right for gate T, a release gate, and wrong for the status line `make release-goal`: the due instant and the elapsed bar do not depend on the unmerged commits, only the merged count does. Found by all three review lanes refuting the claim that said otherwise.

## Falsification

Not applicable, and stated as such rather than dressed up: this branch contains no code, reverts no test and runs none. The artifact declares `falsification.not_applicable` and `scripts/quorum-gate.sh` prints it as NOT VERIFIED — the shape PMAT-224 built so a triage branch would stop being pushed with a waiver, and which PMAT-226 widened to admit the release ledger. What stands in for a falsifier here is that every number in the row was re-derived by a tool that refuses when the declared and the measured disagree, and that the gate is red on the branch for a reason this receipt names.

## Review record

Three sandboxed lanes on the diff at a47ca249, base pinned at cdcc0e80 (`conv-4372a570`, `conv-86493b36`, `conv-71645ff3`): 0/3 PASS, 3/3 FAIL, dissent 3. Seven claims confirmed by all three.

- **C8 refuted by all three, correctly.** The claim said `show` prints the goal with `basis=…:L61`-as-L62 and that `sync --check` finds nothing. On the branch both exit 2; at main the basis is L61, not the L62 the claim quoted, because removing the stray line shifted every later line up by one. Corrected in the digest and filed as PMAT-229.
- **C4 refuted by one lane, wrongly.** Lane 1 reported that PMAT-227's labels block held only `kind:triage`. Re-run at a47ca249: the block is `kind:triage`, `orch:fable`, `release:v1.29.0`, and lanes 2 and 3 read it correctly. Recorded as a lane misread in `.quorum/evidence/book-1.28.0-judges.md`, not as a finding — a lane verdict is a claim, and this is the round where that cut both ways.

No second round: neither finding changes the diff, and re-judging an unchanged diff to convert a 3/3 FAIL into a green line is the theater these gates exist to prevent.

Evidence: `.quorum/evidence/book-1.28.0-{claims,lanes,judges,agy,pmat}.md`; artifact `.quorum/PMAT-227-book-v1.28.0.json` (kind triage, 3 lanes, judges 3, refuters 3, confirmed 8, refuted 1).

## Gaps, named

- The GitHub release for v1.28.0 was still building its binaries when this receipt was written; `docs/audits/release-1.28.0-receipt.md` is where that verdict lives, and gate R reports it PENDING until it is real.
- `release-goal.sh cut` still does not verify that the receipts it books exist at write time (PMAT-225's gap, unchanged); gate T refuses downstream, which is how this cut's paths were checked.
- The skill's own `kind: triage` rail is built for GitHub-issue triage (a snapshot partitioned into batches with a ledger) and does not fit a ledger booking. `kind-gate.sh` admitted it (`kind=triage files=0`) and this receipt keeps the code-shaped rows rather than declaring a shape it did not follow.
- A re-tag would move the cadence clock, because the cut instant is the tagger date. Named in PMAT-225's receipt, still true.

IMPL-PMAT-227-RECEIPT-END
