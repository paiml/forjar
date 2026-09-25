# agy lane — the nightly rebuilds whenever its tag is not HEAD (PMAT-629)

Round count and heads: see `PMAT-629-lanes.md`.

One agy lane, round 3, head 0daebb43: `gemini-3.1-pro-high` through
`agy-lane.sh --mode plan`, sandboxed in a self-contained lane clone, JSON-schema
output, the brief pasted inline with an explicit instruction not to build. The
lane returned the tree witness minted in its clone (verified by agy-lane), and
the clone was removed byte-identical: it read the workspace and wrote nothing.

Verdict PASS, no findings, conv-9661d5ed. Its summary: the gate detects a tag
that is missing or behind HEAD, peels an annotated tag, and the release pins the
tag to the built commit; the test discriminates because it parses and EXECUTES
the workflow's own script in synthetic repositories rather than matching text.

Rounds 1 and 2 had no agy lane: agy reported "not logged into Antigravity" on
the reviewing host and the operator's standing fallback put Claude lanes in its
place. agy answered again before round 3, so round 3 used it.
