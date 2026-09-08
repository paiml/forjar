# Quorum evidence — PMAT-210 — agy /teamwork-preview

Lane: `agy-lane.sh --mode teamwork --timeout 25m`, sandboxed, schema-enforced. Conversation `284839f2-6d45-40a1-9466-e0782ed9d5df`. Verdict PASS with two findings, one of which changed the branch.

Judgment 1, the disposition: deferral to 1.27 is right. Yanking is for a catastrophic regression introduced BY the release in question, and this defect predates it; a same-day 1.26.1 trades a measured fix for a rushed one against a defect the fleet has carried since 1.25.2; holding the board open is not a thing that can be done to a release already published and pinned.

Judgment 2, the evidence: the receipt overclaimed. Its exact words are quoted in `triage485-lanes.md`. The missing probe was named precisely — the same local probes on the 1.25.2 binary — and it was run. See `triage485-pmat.md` for the numbers and the provenance.
