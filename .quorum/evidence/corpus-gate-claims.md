# Quorum evidence — PMAT-217 — the claims put to the lanes

1. Gating `coverage_map_enrichment` on the `aprender-corpus` feature is the right fix, not a real failure parked behind a flag.
2. The ratchet figures are moved correctly and completely, everywhere the tree carries them.
3. The two new falsification rules are non-vacuous.
4. Leaving `cross_project_tests.rs` alone is a defensible distinction rather than an inconsistency.
5. The diff does nothing the ticket does not ask for.

Claim 1 was confirmed by all three lanes. Claim 4 was confirmed in substance and refuted in mechanism: the distinction is right and that file was passing the rule by accident, not by decision. Claims 2 and 3 were refuted. Claim 5 was answered by no lane and is judged in the receipt.
