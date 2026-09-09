# Quorum evidence — PMAT-221 — the claims put to the lanes

1. The census changes no action: every counter and every `PlannedChange.action` is what it was before the diff (f(f(x)) = f(x) at the plan level).
2. NoOp-only, taken after propagation, is the right rule; a resource that plans Create/Update is never named.
3. `probe_covers` is the one definition of "this host's probe covers machine m"; the probe, the executor's pre-plan probe and the census all ask it.
4. Seal compatibility: an empty census serialises to the base's bytes, a non-empty one round-trips through the plan file, a malformed one is refused.
5. Every shipped plan surface carries the TOTAL list and folds the prose into the one `disclosure`.
6. The wording counts what the plan did not MEASURE, never "N drifted", and names the instruments.
7. The tests are non-vacuous: each binary case fails without the fix at the assertion it names.
8. The moved types are byte-identical apart from the new field; every struct literal was updated; every file stays under 500 lines.
9. The diff does nothing the ticket does not ask for; the resource-id-keyed probe map is pre-existing, disclosed, and filed rather than widened.
10. The contract amendment is sound and every FALSIFY row resolves.

Put to the plan first (one teamwork lane, which raised three objections against the plan as written), then to the finished diff (three quorum lanes). Claims 1–9 were confirmed by all three diff lanes. Claim 10 was refuted by one lane and named as a gap by the other two: the plan-file case was cited by no row. The teamwork lane's three objections were each re-checked and are recorded as refuted claims with their corrections.
