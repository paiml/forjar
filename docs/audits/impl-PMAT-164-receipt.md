# Implementation receipt — PMAT-164 — CRUX audit for 1.26.0: nine behaviours × three reference systems, dispositions, reconciliation, gate H keys

verdict: DONE — merged into main by PR #478 (bc46113c); quorum receipt `.quorum/PMAT-164-crux-1.26.0.json` (16 confirmed, 8 refuted); no waiver.

## Identity

| field | value |
|---|---|
| ticket | PMAT-164 (kind: code) |
| branch | PMAT-164-crux-1.26.0 |
| PR | #478 |
| base_commit (receipt) | 0776fd82a57158a87b7dc3e7602a12dee52db893 |
| diff_sha256 (receipt) | 5ec5a3fe844a2e0bf6e7fbf6ef5d8c9e66ddb13d |
| quorum | lanes 5, refuters/claim 3, judges 3 |
| crux systems | Terraform, Ansible, SaltStack, Puppet, Make, Nix profile generations, Kubernetes rollout history, Pulumi per-stack state |
| agy teamwork | ran=True |
| pmat | analyze_vacuous_tests; vacuous in touched paths 0 |

## Plan

| phase | what | route |
|---|---|---|
| P1 | audit from the three CRUX panel lanes; citations against the PMAT-161 branch | subagent:sonnet |
| P2 | quorum ×3, teamwork, crux, pmat; PMAT-188 citation fix; shape test + contract | delegate + subagent |
| P3 | gate H measured FAIL in a scratch clone at 1.26.0 → keys table (3a5e0bf7) → PASS; receipt; PR | direct |

## Dispatch ledger

| # | description | type | model | agent |
|---|---|---|---|---|
| 1 | PMAT-164/ph1.CX worker: write docs/audits/crux-1.26.0.md from the three CRUX lane outputs | paiml-impl-worker | sonnet | — |
| 2 | PMAT-164/ph3.q1 quorum width 4: claims, teamwork, pmat-style vacuity check on the CRUX aud | paiml-agy-delegate | opus | — |
| 3 | PMAT-164/ph2.fix worker: correct the CRUX audit's citations, dispositions and behaviour-4  | paiml-impl-worker | sonnet | — |
| 4 | PMAT-164/ph3.q2 quorum width 3: refuters attack the CRUX audit dossier | paiml-agy-delegate | opus | — |
| 5 | PMAT-164/ph3.q2b quorum width 3: refuters attack the committed CRUX audit dossier | paiml-agy-delegate | opus | — |
| 6 | PMAT-164/ph3.q3 quorum width 3: judges adjudicate the CRUX audit claims | paiml-agy-delegate | opus | — |
| 7 | PMAT-164/ph2.shape worker: a falsification test that pins the CRUX audit's shape | paiml-impl-worker | sonnet | — |

Subagent slots: 3 (config); running peak observed 3; every lane through the agy delegate (`--sandbox`, per-lane shared clones).

## Verification (claimed vs re-run)

| what | orchestrator re-run |
|---|---|
| cargo test --test falsification_crux_audit_shape | 9 passed |
| crux-reconcile.sh at version 1.26.0 (scratch clone) | GATE H PASS 2 of 2 |
| mutation: audit emptied → 6 of 9 shape tests RED | observed |

## Falsification

- test: the nine tests of falsification_crux_audit_shape (title, three rows per behaviour, one disposition per row, [X] and [V] cells, no pre-refactor stamp path, no ownership file, the GO-spec precedents, the reconciliation mapping, no [X] in README or the book)
- file: `tests/falsification_crux_audit_shape.rs` (target `falsification_crux_audit_shape`)
- reverted: docs/audits/crux-1.26.0.md emptied, the target run, the file restored from a copy (tree verified clean)
- observed: `test result: FAILED. 3 passed; 6 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s`

## Gaps

- Coverage and mutation (gate F) and the pv contract depth (gate G) are measured on the release head in docs/audits/dogfood-1.26.0-receipt.md, not per ticket.
- Status-line join rows: [U] (the session's transcript lives under the root project key; the worktree runs were not joined).

IMPL-PMAT-164-RECEIPT-END
