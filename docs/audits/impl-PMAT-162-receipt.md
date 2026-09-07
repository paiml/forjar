# Implementation receipt — PMAT-162 — generation ownership and stack-scoped restore: the reviewed design (v10) for the restore 1.26.0 refuses in a multi-stack state dir; implementation deferred to 1.27

verdict: DONE — merged into main by PR #481 (de4dee15); quorum receipt `.quorum/PMAT-162-generation-ownership-spec.json` (19 confirmed, 12 refuted); no waiver.

## Identity

| field | value |
|---|---|
| ticket | PMAT-162 (kind: code) |
| branch | PMAT-162-generation-ownership-spec |
| PR | #481 |
| base_commit (receipt) | bc46113c8fec9eae37f7193b2c452138ad3647d2 |
| diff_sha256 (receipt) | 82b08f101edd34eb797daabdc0a5cf82af7a8c23 |
| quorum | lanes 5, refuters/claim 3, judges 3 |
| crux systems | Nix profile generations, Kubernetes rollout history and undo, Terraform state locking and workspaces, Pulumi per-stack state |
| agy teamwork | ran=True |
| pmat | analyze_vacuous_tests; vacuous in touched paths 0 |

## Plan

| phase | what | route |
|---|---|---|
| P1 | the spec itself, nine three-lane review rounds (v1 to v9), each round in per-lane clones | delegate:quorum x9 |
| P2 | v10 polish, the shape test and its contract, the receipt | direct + subagent:sonnet |

## Dispatch ledger

| # | description | type | model | agent |
|---|---|---|---|---|
| 1 | PMAT-162/ph1.spec quorum width 3 reviewing the generation-ownership spec in a clone | paiml-agy-delegate | opus | — |
| 2 | PMAT-162/ph1.spec2 quorum width 3 re-reviewing the v2 spec in a clone | paiml-agy-delegate | opus | — |
| 3 | PMAT-162/ph1.spec3 quorum width 3 reviewing the v3 spec in a clone | paiml-agy-delegate | opus | — |
| 4 | PMAT-162/ph1.spec4 quorum width 3 reviewing the v4 spec in a clone | paiml-agy-delegate | opus | — |
| 5 | PMAT-162/ph1.spec5 quorum width 3 reviewing the v5 spec in a clone | paiml-agy-delegate | opus | — |
| 6 | PMAT-162/ph1.spec6 quorum width 3 reviewing the v6 spec in a scratchpad clone | paiml-agy-delegate | opus | — |
| 7 | PMAT-162/ph1.spec7 quorum width 3 reviewing the v7 spec in per-lane scratchpad clones | paiml-agy-delegate | opus | — |
| 8 | PMAT-162/ph1.spec8 quorum width 3 reviewing the v8 spec in per-lane scratchpad clones | paiml-agy-delegate | opus | — |
| 9 | PMAT-162/ph1.spec9 quorum width 3 reviewing the v9 spec in per-lane scratchpad clones | paiml-agy-delegate | opus | — |
| 10 | PMAT-162/ph1.spec9 quorum width 3 reviewing the v9 spec in per-lane scratchpad clones | paiml-agy-delegate | opus | — |
| 11 | PMAT-162/ph2.shape worker: a falsification test that pins the generation-ownership spec's  | paiml-impl-worker | sonnet | — |

Subagent slots: 3 (config); running peak observed 3; every lane through the agy delegate (`--sandbox`, per-lane shared clones).

## Verification (claimed vs re-run)

| what | orchestrator re-run |
|---|---|
| cargo test --test falsification_spec_generation_ownership_shape | 8 passed |
| mutation: the spec emptied | 5 of 8 RED |

## Falsification

- test: the eight tests of falsification_spec_generation_ownership_shape (status line; R1–R9 in order; a §9 row per rule; a mutation per rule; restore_decision with RestoreVerb and RestoreScope; the v9 review record; parent and restores in §4; no TODO/TBD/ownership-file term)
- file: `tests/falsification_spec_generation_ownership_shape.rs` (target `falsification_spec_generation_ownership_shape`)
- reverted: docs/specifications/forjar-state-generation-ownership.md emptied, the target run, the file restored from a copy (tree verified clean)
- observed: `test result: FAILED. 1 passed; 7 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`

## Gaps

- Coverage and mutation (gate F) and the pv contract depth (gate G) are measured on the release head in docs/audits/dogfood-1.26.0-receipt.md, not per ticket.
- Status-line join rows: [U] (the session's transcript lives under the root project key; the worktree runs were not joined).

IMPL-PMAT-162-RECEIPT-END
