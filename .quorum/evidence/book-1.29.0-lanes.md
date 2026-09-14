# PMAT-531 — the lanes

One round, three sandboxed agy quorum lanes, width 3, `writes=false`, on the
booking diff with `--not-before` pinned to the dispatch instant.

| lane | conversation | exit | duration | verdict |
|---|---|---|---|---|
| 1 | conv-d56db057 | 0 | 730s | FAIL |
| 2 | conv-c27abfd8 | 0 | 430s | FAIL |
| 3 | conv-d1221a10 | 0 | 611s | FAIL |

## The finding that mattered was unanimous

All three refuted claim 5 — the label moves — and they were right twice over.
A cancelled ticket was being carried into the next window forever, and a ticket
marked finished had thirteen of its own sites still open.

Neither is visible from inside the booking: `release-goal.sh cut` prints
`removed … / labelled …` for each move and both looked ordinary. What made them
findable was reading the ROWS rather than the moves.

## The split that mattered was claim 1

Two lanes refuted the `tickets` list as missing PMAT-200, PMAT-233 and
PMAT-240; the third confirmed it against the rule. The third was right. The
window's rule is the first `PMAT-<n>` in the branch, then the title, then the
body — the same rule gate A applies — and what the other two found were
MENTIONS in PR bodies, not shipped tickets.

A 2-1 split where the minority is correct is the reason this repository re-runs
every refutation instead of taking the count.

## A confirmation that expired

All three confirmed claim 6, that the booking is honestly a `kind: triage`
change touching only the ledger and the roadmap. That was true when they read
it and is false now: fixing the ratchet's ceilings brought
`scripts/ratchets/cb21xx-baseline.json` into the diff, which is off the triage
rail. The receipt is `kind: code` with a falsification, and the digest records
the expiry rather than keeping a confirmation that no longer holds.

## The no-write rule

The brief opened with it and named the lanes in two earlier rounds this session
that wrote into the repository root. The tree was clean afterwards.
