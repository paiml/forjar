# PMAT-579 / PMAT-581 / PMAT-582 — the lanes, and what each returned

This table is the ONE place this receipt counts rounds and names heads. Every
round is three sandboxed `agy` review lanes, each in its own clone with push
removed, `writes=false`, briefed with the three tickets' rows and receipts.

| round | head | lane 1 | lane 2 | lane 3 | outcome |
|---|---|---|---|---|---|
| 1 | d267f429 | FAIL (3 cited findings) | PASS | PASS | not agreed |
| 2 | 5a6d607a | terminated before any lane returned — no verdict, not counted | | | void |
| 3 | 5a6d607a | `gemini-3.1-pro-high` PASS | `gemini-3.8-flash-high` PASS | `gemini-3.7-flash-high` PASS | AGREED 3/3 |

Round 1's posted artifact carries lane verdicts and finding counts but not the
lane model ids, so they are not restated here. Round 2 was the same command
relaunched on 5a6d607a and killed by the orchestrator's session before a lane
answered; it left no artifact and is listed so the round numbering is honest.

## Round 1's finding, and what happened to it

Lane 1 at d267f429 failed the diff because the receipts' scope section claimed
the change rode the `kind: triage` rail while every row carried the label
`kind:code`, and read the two as a contradiction the receipts had not
addressed. That was a true reading of a badly worded section: it named both
kinds in one breath without saying they classify different things.

Fixed at 5a6d607a, in the receipts, not the rows. The section is now "Two
different kinds — easy to confuse, so named separately": the DIFF is triage
(it is classify-and-link on the rail), while each ROW's label classifies the
work it registers, which will be implemented as code. Relabelling the rows
`kind:triage` was rejected because it would make the label false about the
work — measured at the merge base, forjar's roadmap uses `kind:code` on 115
rows and `kind:triage` on 7, the latter reserved for bookings and status syncs.

Round 3's three PASS summaries each confirm the rail (only the roadmap and
`docs/audits/**` touched), the absence of code or test changes, and the
quotation from `src/core/planner/unprobed.rs`. None of them could see the
paiml/infra measurements behind PMAT-579 and PMAT-582; see `pin-preflight-agy.md`.

The merge rail runs its own round on the FINAL head; by construction that round
cannot be described in a file it is reviewing.
