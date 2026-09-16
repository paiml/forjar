# impl receipt — PMAT-579

**Registers** paiml/forjar#579. Ships no implementation.

## The measurement the row carries

On paiml/infra, 2026-09-16, `forjar history --json` stamps a correct `forjar_version` on every `apply_started` event. For the same 1.31.0 pin, applied by the same operator seven minutes apart:

| run id | machine | started | binary that performed it |
|---|---|---|---|
| `r-c61e1b0dc7cd` | lambda-labs | 10:24:19Z | **1.30.0** |
| `r-c67eb3d5c454` | intel | 10:31:14Z | **1.31.0** |

Neither run said anything about the mismatch, because nothing in `apply` or `plan` compares the running binary with the version `stack-tool-forjar` pins.

## What would make this row false

- `forjar apply` or `forjar plan` already refusing, or warning, when the running binary differs from the pinned version
- the event log above not recording 1.30.0 for `r-c61e1b0dc7cd`

## Two different "kinds" — easy to confuse, so named separately

**This diff** is `kind: triage` in the quorum gate's sense. It touches only `docs/roadmaps/roadmap.yaml` and three receipts under `docs/audits/`, which is exactly the gate's triage path rail (`docs/audits/**`, `docs/roadmaps/roadmap.yaml`, `docs/roadmaps/releases.yaml`, `.quorum/**`). No source, test, workflow or contract file is touched. Every row is registration-only by instruction ("mint the tickets, no implementation this session").

**Each row** carries `labels: - kind:code`, and that is correct, not a contradiction. The label classifies the *work the row registers*, and PMAT-579, PMAT-581 and PMAT-582 will each be implemented as code. forjar's roadmap uses the label this way throughout — measured on this tree: 115 rows carry `kind:code` and 7 carry `kind:triage`; every `planned` row that registers future code work carries `kind:code`, and `kind:triage` is reserved for work that is itself classify-and-link (release-ledger bookings, status syncs). Relabelling these rows `kind:triage` would make the label false about the work.

## Why the title reads like an order

`pmat work` titles are immutable. A row's title is therefore written as the definition-of-done of the work it registers, because that is the sentence the eventual implementation will be judged against. It is **not** this diff's contract. The criterion states this diff's contract; the implementation is recorded in the row's `notes:` as future scope with its own falsifier. On paiml/infra#633 a lane failed exactly this confusion, correctly, and the fix was a receipt like this one rather than a reworded title.
