# impl receipt — PMAT-581

**Registers** paiml/forjar#581. Ships no implementation.

## The measurement the row carries

`src/core/planner/unprobed.rs` states the behaviour in its own module doc: the probe "is only taken for machines this host answers for — every SSH target, and since forjar#495 every pepita namespace, has planned `NoOp` over changed sources for as long as the probe has existed", and "the action is left alone on purpose". forjar#497 added the census that *names* those resources; nothing gives a remote resource a content key it would act on.

The downstream cost, measured by the paiml/infra fleet session on infra#629: **35 hand-rolled activation checks across 6 configs** standing in for the missing primitive, each asserting loaded/enabled/active, none of which changes when a unit body changes.

## What would make this row false

- a remote resource already able to plan `Update` from a controller-side change without contacting the host
- `unprobed.rs`'s module doc no longer describing SSH targets as planning `NoOp` over changed sources

## Scope check

This PR touches only `docs/roadmaps/roadmap.yaml` and three receipts under `docs/audits/` — all on the `kind: triage` rail. No source, test, workflow or contract file is touched. Every row is registration-only by instruction ("mint the tickets, no implementation this session").

## Why the title reads like an order

`pmat work` titles are immutable. A row's title is therefore written as the definition-of-done of the work it registers, because that is the sentence the eventual implementation will be judged against. It is **not** this diff's contract. The criterion states this diff's contract; the implementation is recorded in the row's `notes:` as future scope with its own falsifier. On paiml/infra#633 a lane failed exactly this confusion, correctly, and the fix was a receipt like this one rather than a reworded title.
