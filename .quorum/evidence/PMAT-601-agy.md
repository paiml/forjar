# agy lanes — impl receipts for the v1.32.0 window (PMAT-601)

Round count and heads: see `PMAT-601-lanes.md`.

`gemini-3.1-pro-high` (effort high) and `gemini-3.1-pro-low`, `--sandbox`, JSON
schema, brief pasted inline with a ground-truth block of merge shas, runner names
and quorum counts to check the receipts against.

agy-high found the arithmetic the receipts got wrong and, with agy-low, the one
claim that was true but uncheckable from anything they were shown. agy-high also
raised one false refutation, conflating a roadmap row's `kind:code` label with the
path-based triage rail. The Claude lane, able to query `gh` and read the quorum
files, found the two defects the agy lanes could not: a citation to a file not yet
written, and a stale count inside an already-merged receipt.
