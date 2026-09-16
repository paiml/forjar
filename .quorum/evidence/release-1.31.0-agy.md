# PMAT-574 — the agy round

One round of three sandboxed `agy` quorum lanes, review-only, over head
203d8a65 against `main` at 9884334a.

| lane | model | sandbox | writes | verdict |
|---|---|---|---|---|
| 1 | `gemini-3.1-pro-high` | yes | false | RECORDED BELOW |
| 2 | `gemini-3.8-flash-high` | yes | false | RECORDED BELOW |
| 3 | `gpt-oss-120b-medium` | yes | false | RECORDED BELOW |

Three DISTINCT model ids, none in the author's family (the author is
`opus`/claude). The distinctness matters and was chosen deliberately: the
PMAT-565 round ran `gemini-3.1-pro-high` twice and the receipt validator marked
it `partial` — "lanes sharing an id are resamples, not independent reviewers
(PMAT-125)".

Each lane reviewed in its own full clone with push removed. No lane may write to
this tree; the `writes=false` column is a property of the harness, not a promise
from the lane.

## What a lane can and cannot rule on

A lane reads a diff. It cannot run the release gates, cannot ask GitHub what
merged, and cannot measure the CB-21xx ratchet — so on a release cut, whose diff
is almost entirely a record of things measured elsewhere, the lanes are the
WEAKER half of the adjudication and this receipt does not pretend otherwise.
The refutations that mattered came from the gates and the hooks, and
`release-1.31.0-judges.md` names them.
