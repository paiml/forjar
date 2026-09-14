# PMAT-540 — the lanes

One round, three sandboxed agy quorum lanes, review-only (`writes=false`),
against a FULL standalone clone, dispatched in one message with `--not-before`
pinned to the dispatch instant. 22 findings.

| lane | brief | verdict |
|---|---|---|
| 1 (conv-677f8786) | the arm and the floor: find a shape that still misattributes | FAIL |
| 2 (conv-df6462cd) | the nine cases and the fixture they run against | FAIL |
| 3 (conv-26c150de) | the documents and the numbers, re-derived | FAIL |

Lane 2 carried the round: it measured that the test fixture did not reproduce
the message shape the whole ticket is about, so an arm rewritten to use git's
own trailer parser would have passed every case. That is the one regression the
suite exists to prevent, and no other lane and no test found it.

Lane 3 re-derived the 30-PR table row by row and caught a count in the script's
own comment that disagreed with it. Lane 1 read the arm against the file above
it and found the receipt describing a log section that says something else.

The delegate hit its 30-turn limit before writing a receipt — the eleventh time
in this session, and filed as paiml-implement#141. All three lane JSONs were on
disk with `status: SUCCESS` and were read directly.
