# PMAT-557 — the claims put to the round

A release-ledger booking, not a code change. forjar v1.30.0 was tagged on
2026-09-13 and never declared in `docs/roadmaps/releases.yaml`, so gate T was
red on main (issue #557) and no later cut could pass it. The branch:

1. adds the v1.30.0 row with `scripts/release-goal.sh cut v1.30.0 --next
   v1.31.0` — cut time, PRs, tickets, the dogfood and crux paths — and replaces
   the cookbook commit the script copied from the previous row (0be3e1ec, which
   locks forjar 1.29.0) with 60acf9c9, the squash of paiml/forjar-cookbook#21,
   which locks 1.30.0;
2. declares `next.tag v1.31.0`, due 2026-09-15T20:52:01Z;
3. appends the `DOGFOOD-1.30.0-RECEIPT-END` marker the 1.30.0 dogfood receipt
   was committed without;
4. moves `release:v1.30.0` off three rows v1.30.0 did not ship and labels the
   tickets merged since the tag `release:v1.31.0`;
5. marks PMAT-555, PMAT-547 and PMAT-562 completed — merged, issues closed;
6. mints roadmap rows for five open issues that had none, on a 1.32.0
   milestone.

Three read-only lanes against self-contained clones at `2edf43c7`. Each was
asked to check the row against the tag and the window, the honesty of the
marker, the label moves, the completed rows, the minted rows' shape and any
YAML round-trip churn, and to quote any false sentence in the commit messages.
`gh` was expected to be unauthenticated in the sandbox; lanes were told to say
UNMEASURED rather than guess.
