# Quorum evidence — #446 — judges

Decision: ship `facts` as a report (`--json`) and defer the resolver-visible facts model to E11 (#414), or build the model now.

| judge | report now, model in E11 | model now |
|---|---|---|
| scope (the ticket's three asks) | 3/3 delivered as verbs the operator runs | 2/3 + a resolver change the ticket did not ask for |
| risk to the apply path | none (no executor change) | resolver/template changes touch every plan |
| release fit (1.25.0, fewest iterations) | one PR, one falsifier | a second design cycle |
| reversibility | E11 can lift `Facts` into the resolver unchanged | a rushed model becomes a compatibility surface |

3/3 judges: report now. Recorded as REFUTED claim 1 in the claims file.
