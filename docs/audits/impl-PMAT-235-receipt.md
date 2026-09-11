# Implementation receipt — PMAT-235 — the roadmap's status field says what shipped

verdict: PASS — 16 tickets that a tagged release or a merged PR names read `planned` or `inprogress`; all 16 now read `completed`, moved with `pmat work edit` through the lifecycle the tool enforces, never by editing the YAML. Measured over the whole file: 16 status changes, 16 `updated` bumps, 14 top-level `kind:` fields the tool drops, 29 list items and 2 notes requoted, 183 rows in and 183 out, and **zero semantic diffs outside status, updated and kind**. Every dropped `kind:` field is duplicated by the `kind:*` label `kind-gate.sh` reads first, and every `release:<tag>` label is intact. `pmat work validate` passes and gate T is green. Filed: PMAT-236, the gate arm that would have caught this.

orch_model: opus [A]   orch_class: triage   orch_decision: admit   orch_basis: state
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=mechanical  route=self  w=100.00  basis=absent  (a tool invocation per ticket and a field-by-field diff; no lane can measure a YAML file more cheaply than the diff itself)

verification:
  cmd="the drift before, the refused transition, the legal path, the classified diff, pmat work validate, gate T"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-235-status-sync.log  sha256=recorded-in-the-log

## What was wrong

The ledger and the labels were right the whole time. `docs/roadmaps/releases.yaml` named the right tickets, every one of them carried `release:<tag>`, and gate T reconciled all of it — while the roadmap said none of the 1.28.0 work had started on the day 1.28.0 shipped.

| release | tickets reading `planned` or `inprogress` |
|---|---|
| v1.25.2 | PMAT-137, PMAT-159 |
| v1.26.0 | PMAT-162, PMAT-204 |
| v1.27.0 | PMAT-208 |
| v1.28.0 | PMAT-215, 217, 219, 220, 221, 222, 223, 224 |
| merged since v1.28.0 | PMAT-227, PMAT-230, PMAT-231 |

Gate T checks the `release:<tag>` label in both directions and never looks at `status`, so nothing went red for five releases. That is PMAT-236.

## What the tool did, and what it cost

`pmat work edit <id> -s completed` **refuses** a `planned` ticket: `Invalid transition: Planned → Completed. See work-dbc-v1.yaml §work_lifecycle`. The legal path is `-s inprogress` then `-s completed`, and that is what every ticket took. The refusal is the tool doing its job and it is recorded rather than worked around.

The edit drops any top-level field it does not know, which cost 14 rows their `kind:` field — the behaviour this repository had already measured on a different field. It is not a loss here: all 14 carry the matching `kind:*` label, and `kind-gate.sh` reads `label-kind` before the field. Checked explicitly rather than assumed.

## Gaps, named

- PMAT-236 is filed, not built: until gate T checks status, this can drift again.
- PMAT-159 carries no `kind` at all, field or label. It predates the convention and is left alone.
- `pmat work complete` was not used: it runs quality gates and falsification against work being finished now, and every one of these was finished days ago. `edit` is the honest instrument for a backfill, and this receipt says so rather than implying the heavier command ran.

IMPL-PMAT-235-RECEIPT-END
