# agy lane — correct how the 24h nightly gate failed (PMAT-631)

Round count and heads: see `PMAT-631-lanes.md`.

One agy lane, round 1, head 3e47bc2c: `gemini-3.1-pro-high` through
`agy-lane.sh --mode plan`, sandboxed in a self-contained lane clone, JSON-schema
output, the brief pasted inline with an explicit instruction not to build. It
read the workspace and wrote nothing.

Verdict PASS, no findings, conv-5e7403c6. Its summary: the PR modifies only
comments in the two files; the corrected statements are accurate — a commit
landing after 04:00 is not missed by a 24-hour window at the next day's run,
while missed, queued, late, failed or dropped runs leave commits outside it; the
dates are correctly given as 2001-09-09; no instance of the false claims remains.

The same agy model, on the pzsh port of the gate, is the lane that found the
false mechanism this change corrects.
