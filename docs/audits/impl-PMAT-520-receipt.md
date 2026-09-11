# Implementation receipt — PMAT-520 — the 1.29.0 cut

verdict: PASS — `make dogfood-release` exits 0 on a clean tree with all nine gates green and coverage at 96.43%; the version and lockfile are bumped together; `[Unreleased]` was empty and the twelve behaviour bullets are written for the first time; every bullet has a crux row naming ≥3 surveyed systems; and three review lanes read the record looking for a false sentence and found four, all corrected before it became the record.

orch_model: opus [A]   orch_class: code   orch_decision: admit   orch_basis: release
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=impl  route=self  w=100.00  basis=absent  (the cut: version, CHANGELOG, crux, dogfood receipt)
  ph1.crux  class=research  route=agy-quorum  w=1.00  basis=absent  effort=1[U]  (one lane, ten rows)
  ph1.delegate  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]  (three lanes, 3 FAIL)
  ph2  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="make dogfood-release"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/dogfood-1.29.0-receipt.md
  cmd="bash scripts/dogfood/crux-reconcile.sh with docs/audits/crux-1.29.0.md removed"  claimed_exit=-  rerun_exit=1  log_path=docs/audits/logs/PMAT-520-cut.log
  cmd="cargo test --test falsification_crux_gate_reads_the_release_section"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-520-cut.log

## What is in the release

Twelve PRs across fifteen tickets since v1.28.0, every one labelled
`release:v1.29.0` at the moment it merged. Gate A and gate E each enumerate all
twelve and find a receipt for every one.

Every ticket is release machinery, gates, scripts, tests or the record. The
release changes **twelve `.rs` files** since v1.28.0 and **zero** under `src/`:
they are all falsification tests. That is why gate F's mutation arm measures a
zero — there is nothing to mutate — and it is the same state 1.28.0 was cut in.

## The falsification, and its honest limit

A release cut has no production hunk to revert, so the hunk is the record
itself: remove `docs/audits/crux-1.29.0.md` and gate H is RED naming all twelve
behaviour bullets; restore it and gate H is green. That is a genuine
revert-the-hunk proof about THIS change, because the crux document is the half
of this diff that makes a claim the tree can contradict.

**What could not be proven the same way**, and is recorded as such rather than
implied: reverting `Cargo.toml` in the working tree does NOT move gate T's "cut
in flight" verdict, because gate T reads the version at HEAD
(`git show HEAD:Cargo.toml`). Measured — with main's 1.28.0 in the working tree
the gate still reports 1.29.0. That is correct behaviour for the gate and it
makes a working-tree revert useless as evidence about it, so the receipt claims
only the crux half.

## What the lanes found

Three lanes, all FAIL. The brief asked for one thing above the six claims:
**find a sentence in the CHANGELOG, the crux audit or the dogfood receipt that
a reader could check and find false.** They found four, and every one was
re-measured before it was accepted:

1. **"Thirteen PRs across sixteen tickets."** Twelve and fifteen. The
   thirteenth is this cut's own PR, which has not merged. All three lanes.
2. **`README.md:96` read `forjar = "1.28"`** while the tree builds 1.29.0.
   Gate D passed it and is correct by its predicate — a caret admits 1.29.0 —
   but its own comment names "a README pinning a version the crate has moved
   past" as the defect and the code only fails on a claim that is NEWER.
   Corrected by hand; the gate's gap is PMAT-526 (#526), not widened inside a
   release cut.
3. **Crux row 1 said gate T "refuses the tag"** when the cookbook cannot use
   the release. It refuses the ledger ROW; the tag exists by then.
4. **The dogfood receipt said the release "changes no source file."** Twelve
   `.rs` files, zero under `src/`.

And one thing corrected without having been wrong: three lanes read the
CB-21xx figures in the CHANGELOG as a list of CEILINGS. They are finding
counts. A number three readers misread is a writing defect even when it is
true, so each now says which check it belongs to.

**One refutation does not reproduce.** A lane called the roster arithmetic
impossible — "172 − 6 + 1 = 167, not 166". Measured: 172 − 6 = 166 exactly,
because CB-148 is in BOTH rosters. The first build printed it as
`RETIRED — superseded by CB-2110`, which is a row and not an absence. Recorded
as a lane error rather than acted on.

## The finding the dogfood receipt leads with

**The comply check roster is not stable across local builds of one pmat
version.** Inside a single day: 172 checks with the CB-2110..CB-2115 family,
then 166 without it and CB-148 live again, then 172 with it back — with
`pmat --version` reading 3.40.0 throughout and its banner going
`commit: 5db342d5` → `commit: unknown` → `commit: d76e533e`.

Gate B's Arm 7 is RED while the family is absent, which is correct: a ceiling
cannot be asserted against a tool that does not run the check. It also means
this gate's verdict depends on which local build is installed, so the same tree
can be green here and red elsewhere on an identical version string. That is the
largest risk to the next cut and it is named in the dogfood receipt rather than
smoothed over.

## What is left after this merges

`git tag v1.29.0`, then `scripts/release-goal.sh cut v1.29.0 --next v1.30.0`
books the ledger row — which from this release also names the
paiml/forjar-cookbook commit the release was qualified against (PMAT-241) —
then publish, then `make dogfood-published VERSION=1.29.0`, then the record PR.
Gate T is red on main between the tag and the booking, by design.

PMAT-520 is left OPEN: CB-2113 refuses a commit whose `Pmat-Ticket` names a
ticket already completed, so a cut ticket is marked completed by the next PR.

## Gaps, named

- The crux audit's rows 11 and 12 are the orchestrator's, not the lane's,
  because neither behaviour existed when the lane ran. They are marked as such
  in the audit's own "limit" section.
- Determinism was not asserted by a second run: the dogfood receipt carries no
  timestamps, which is the property, but `--twice` was not passed.
- The README was corrected by hand. Nothing would have caught the next one
  until PMAT-526 lands.

IMPL-PMAT-520-RECEIPT-END
