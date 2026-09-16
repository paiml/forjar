# PMAT-576 — the lanes, and what each returned

This table is the ONE place this receipt counts rounds and names heads.

| round | head | lane 1 | lane 2 | lane 3 | outcome |
|---|---|---|---|---|---|
| 1 | be830b24 | `gemini-3.1-pro-high` PASS (4 cited notes, no defect claimed) | `gemini-3.1-pro-low` FAIL (1 cited finding) | `gemini-3.6-flash-high` PASS | not agreed |

## Round 1's finding, and what happened to it

> The receipt claims `ticket PMAT-576 (forjar#576, milestone 1.32.0); kind:
> triage`, but the diff for PMAT-576 does not set `kind: triage` or have a
> `kind:triage` label.

CONFIRMED, and fixed. The row carried only `release:v1.32.0`; PMAT-557's row —
the 1.30.0 booking this one copies in shape — carries `kind:triage` beside it,
and the quorum receipt declares `kind: triage`, which is the field that lets the
gate apply the documentation-citation rule instead of the Rust one. A receipt
naming a kind its own ledger row does not carry is the cheapest kind of false
record, and a lane reading only the diff caught it.

Lane 1 returned PASS with four cited notes rather than findings: the row at
`docs/roadmaps/releases.yaml:86`, the declaration at `:94`, PMAT-574's completed
status and PMAT-576's own release field. They are the claims this receipt makes,
confirmed against the lines that carry them.

The merge rail runs its own round on the FINAL head; by construction that round
cannot be described in a file it is reviewing.
