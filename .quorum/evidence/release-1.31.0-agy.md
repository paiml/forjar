# PMAT-574 — the agy rounds

Every round is three sandboxed `agy` quorum lanes, review-only, in their own
clones with push removed, `writes=false`, against `main` at 9884334a. THE
PER-ROUND TABLE IN `release-1.31.0-lanes.md` IS THE ONLY PLACE THIS RECEIPT
COUNTS ROUNDS: this file and `release-1.31.0-claims.md` deliberately restate no
number, because a count repeated in four files is a count that goes stale in
three of them — which two separate rounds caught, and which is the reason for
this rule.

Models used: `gemini-3.1-pro-high`, `gemini-3.8-flash-high`,
`gemini-3.8-flash-medium`, `gemini-3.6-flash-high`, `gemini-3.6-flash-medium`,
`gpt-oss-120b-medium`. Every round ran three DISTINCT model ids, none in the
author's family (the author is `opus`/claude). The distinctness was chosen
deliberately: the PMAT-565 round ran `gemini-3.1-pro-high` twice and the
validator marked it `partial` — "lanes sharing an id are resamples, not
independent reviewers (PMAT-125)". `gemini-3.8-flash-*` returned a SUCCESS
envelope with no verdict object in every round it ran in, and was dropped after
round 3 for that reason and no other.

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
