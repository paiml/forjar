# PMAT-579 / PMAT-581 / PMAT-582 — adjudicated claims

The rounds, lanes and verdicts are the table in `pin-preflight-lanes.md`. What
matters here is the adjudication: seven claims CONFIRMED and three REFUTED,
with the refuting authority named in each case. Two of the three refutations
are the orchestrator's own, recorded because a receipt that only lists what
its lanes blessed is a tally with extra steps.

## CONFIRMED

1. [rail-only] That the diff touches nothing outside the triage rail, so a
   `kind: triage` receipt is the honest shape.
   - evidence: `git diff --name-only` at the merge base lists exactly four
     paths — the roadmap and three receipts — and round 3's three lanes each
     confirmed it independently. The rail and what it excludes are stated at
     `docs/audits/impl-PMAT-579-receipt.md:23`, and the gate verifies the rail
     from the diff, not from the receipt.

2. [579-two-binaries-one-pin] That the same 1.31.0 pin was applied by two
   different binaries seven minutes apart and forjar said nothing.
   - evidence: measured by the orchestrator on paiml/infra with
     `forjar history --json`, which stamps `forjar_version` per event: run
     `r-c61e1b0dc7cd` on lambda-labs at 10:24:19Z by 1.30.0 and
     `r-c67eb3d5c454` on intel at 10:31:14Z by 1.31.0, the table at
     `docs/audits/impl-PMAT-579-receipt.md:11` and `:12`. No lane can reach
     that repository; the claim is the orchestrator's.

3. [581-quote-is-verbatim] That PMAT-581's quotation of the planner's module
   doc is verbatim and still true on this tree.
   - evidence: `src/core/planner/unprobed.rs:6` reads "the probe is only taken
     for machines this host answers", `:8` "planned `NoOp` over changed sources
     for as long as the probe has existed", `:10` "The action is left alone on
     purpose". Lanes 2 and 3 of round 3 confirmed the quotation against the
     file; the receipt carries it at `docs/audits/impl-PMAT-581-receipt.md:7`.

4. [582-partial-gets-a-verdict] That a drift run inspecting 40 of 153
   resources returned the same exit 1 a complete run returns.
   - evidence: measured by the orchestrator on one host at one commit, varying
     only `--state-dir`; the three rows at
     `docs/audits/impl-PMAT-582-receipt.md:12` and its neighbours. The zero line
     is the constant `DRIFT_DECLINED_MARKER` at `src/core/error.rs:122`,
     "declined: inspected 0 of", which fires at zero and nowhere else; the
     receipt says so at `docs/audits/impl-PMAT-582-receipt.md:15`.

5. [kind-code-labels-are-true] That each row's `kind:code` label is true about
   the work it registers, and that this repo uses the label that way.
   - evidence: at the merge base the roadmap carries 115 `kind:code` and 7
     `kind:triage` labels, the latter on bookings such as the 1.30.0 ledger
     row at `docs/roadmaps/roadmap.yaml:4154`; with these three rows it is 118
     and 7. The distinction from the DIFF's kind is drawn at
     `docs/audits/impl-PMAT-579-receipt.md:25`, and round 3's lane 1 called the
     classification correct.

6. [criteria-describe-this-diff] That each acceptance criterion states what
   this diff does, and the implementation is future scope in `notes:`.
   - evidence: every criterion opens "This row REGISTERS paiml/forjar#NNN and
     ships no implementation" and closes by naming the notes; every `notes:`
     opens "Future scope, NOT shipped by this row" and ends with a falsifier.
     Why the immutable title still reads like an order is explained at
     `docs/audits/impl-PMAT-579-receipt.md:29`. Round 3's lanes each confirmed
     the criteria match the diff.

7. [580-dropped-not-hidden] That the fourth row minted in this session was
   dropped as a duplicate and the reason is on the record.
   - evidence: the branch's second commit, bd38127b, drops PMAT-580 because it
     duplicated PMAT-565, which forjar#570 had already fixed and merged; the
     roadmap on this branch carries zero `PMAT-580` rows, and forjar#580 was
     closed with its evidence moved to #570.

## REFUTED

1. [receipts-could-not-be-misread] That the receipts' original scope section
   could not be read as claiming two kinds for one diff.
   - evidence: round 1's lane 1 at d267f429 refuted it: the section said the
     diff rode the `kind: triage` rail while every row carried `kind:code`, and
     the lane read that as a contradiction the receipts had not addressed. It
     was a fair reading of careless wording. Fixed at 5a6d607a by the section
     beginning at `docs/audits/impl-PMAT-579-receipt.md:21`, which names what
     each kind classifies; the rows were not relabelled, because that would be
     false.

2. [lanes-confirm-the-measurements] That three PASS verdicts confirm the
   measurements behind PMAT-579 and PMAT-582.
   - evidence: refuted by what a lane can reach. Both measurements were taken
     on paiml/infra — apply events and drift runs on real hosts — and a lane in
     a forjar clone has neither that repository nor those hosts. The lanes
     confirmed the rail, the in-repo quotation and the criteria; the numbers
     are the orchestrator's and `pin-preflight-agy.md` says so.

3. [crux-covers-three-rows] That the crux survey in `pin-preflight-crux.md`
   settles the prior art for all three registered defects.
   - evidence: refuted by the survey itself. It answers one question, the one
     PMAT-579 registers, across four systems, and its "What was not surveyed"
     section says PMAT-581 and PMAT-582 were not covered. Three systems for one
     question are not coverage of three tickets; each implementation owes its
     own survey.

## The kill rule

No lane finding was dismissed. Round 1's finding was true and is fixed in the
tree, and the fix was to the receipts' wording, not to the rows' labels. Round
3's three PASS summaries are the receipt's own claims, confirmed against the
lines that carry them. A red gate stops this registration and no verdict
overrides it.
