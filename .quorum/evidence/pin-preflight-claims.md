# PMAT-579 / PMAT-581 / PMAT-582 — the claims put to the round

This branch registers three forjar defects found while auditing a fleet pin on
paiml/infra with forjar 1.31.0 on 2026-09-16. It adds three `planned` rows to
`docs/roadmaps/roadmap.yaml` and one receipt per row under `docs/audits/`, and
ships no implementation: no source, test, workflow, contract or script changes.

The claims put to the lanes were:

1. The diff touches nothing outside the quorum gate's triage rail — the roadmap
   and `docs/audits/**` — so `kind: triage` is the receipt shape that applies.
2. PMAT-579's measurement is real: the same 1.31.0 pin was applied to
   lambda-labs by the 1.30.0 binary and to intel by the 1.31.0 binary, seven
   minutes apart, and forjar said nothing, because nothing compares the running
   binary to the pin.
3. PMAT-581's quotation is verbatim from `src/core/planner/unprobed.rs`: SSH
   targets plan `NoOp` over changed sources, on purpose, and nothing gives them
   a content key.
4. PMAT-582's measurement is real: with an absent default state dir, drift
   inspected 40 of 153 resources and returned an ordinary exit 1 verdict,
   because forjar#569 declines only at zero.
5. Each row's `labels: kind:code` is true about the work it registers; the
   diff's own kind is triage. The two are named separately in every receipt.
6. Each row's acceptance criterion describes THIS diff (registration and a
   cited measurement), and the future implementation lives in `notes:` with a
   falsifier, so no lane is asked to grade work that did not happen.
7. PMAT-580 was minted and dropped as a duplicate of PMAT-565 (forjar#570,
   merged), so the branch registers three rows, not four.

Every round's lanes, models and verdicts are the table in
`pin-preflight-lanes.md`, which is the only place this receipt counts them.
