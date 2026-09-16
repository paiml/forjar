# impl receipt — PMAT-582

**Registers** paiml/forjar#582. Ships no implementation.

## The measurement the row carries

One host (intel), one commit (a worktree at paiml/infra `origin/main`), varying only `--state-dir`, 2026-09-16:

| state dir | inspected | reported | exit |
|---|---|---|---|
| `/home/noah/src/infra/state` | full scope | 9 DRIFTED | 1 |
| default `./state` — **absent**; `state/` is gitignored, so no git worktree has one | **40 of 153** | `Drift detected: 2` | 1 |
| `machines/intel/state` (a second live tree, another session) | — | 3 DRIFTED | 1 |

forjar#569 (PMAT-564) made `inspected 0 of N` decline with exit 2 and deliberately drew that line at zero; one inspected resource is enough to be graded. This row argues for the step past that line, not that it regressed. Naming the resolved lock is tracked separately as forjar#585.

## What would make this row false

- `drift` returning a distinguishable verdict or exit code for a partial inspection
- the 40-of-153 run above exiting anything other than 1

## Scope check

This PR touches only `docs/roadmaps/roadmap.yaml` and three receipts under `docs/audits/` — all on the `kind: triage` rail. No source, test, workflow or contract file is touched. Every row is registration-only by instruction ("mint the tickets, no implementation this session").

## Why the title reads like an order

`pmat work` titles are immutable. A row's title is therefore written as the definition-of-done of the work it registers, because that is the sentence the eventual implementation will be judged against. It is **not** this diff's contract. The criterion states this diff's contract; the implementation is recorded in the row's `notes:` as future scope with its own falsifier. On paiml/infra#633 a lane failed exactly this confusion, correctly, and the fix was a receipt like this one rather than a reworded title.
