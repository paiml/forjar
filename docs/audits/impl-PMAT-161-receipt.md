# Implementation receipt — PMAT-161 — state stamp keyed by config name; shared --state-dir for apply, status and stack_conflict; undo refused in a multi-stack dir

verdict: DONE — merged into main by PR #473 (0776fd82); quorum receipt `.quorum/PMAT-161-state-stamp-per-name.json` (59 confirmed, 8 refuted); no waiver.

## Identity

| field | value |
|---|---|
| ticket | PMAT-161 (kind: code) |
| branch | PMAT-161-state-stamp-per-name |
| PR | #473 |
| base_commit (receipt) | a8efff16c57e39efffdbe5ace621c789487ecd34 |
| diff_sha256 (receipt) | 42599deeb1a0414d8faa1f2c5f3b740d270ae5d6 |
| quorum | lanes 5, refuters/claim 3, judges 3 |
| crux systems | Terraform workspaces and state locking, Pulumi per-stack state and stack rename, Nix profiles and generations, Kubernetes namespaces and ownerReferences, Ansible (stateless) |
| agy teamwork | ran=True |
| pmat | analyze_vacuous_tests; vacuous in touched paths 4 |

## Plan

| phase | what | route |
|---|---|---|
| P1 | GlobalLock.stacks map (schema 1.1), stack_conflict keyed by name, per-stack outputs and status attribution | subagent:opus + delegate:teamwork |
| P2 | multi_stack_restore_refusal at the restore primitive; retention skip note; rename retirement | subagent:opus |
| P3 | quorum ×3 (claims, refuters, judges), teamwork, crux, pmat; S1 fixes PMAT-168/174/175/176/177/182/183; receipts, PR #473 | delegate + direct |

## Dispatch ledger

| # | description | type | model | agent |
|---|---|---|---|---|
| 1 | PMAT-161/ph1.B worker B: per-name stack stamp in the global lock, with migration | paiml-impl-worker | opus | abc24690a26c4434b |
| 2 | PMAT-161/ph1.delegate teamwork width 1 on the per-name stamp plan | paiml-agy-delegate | opus | — |
| 3 | PMAT-161/ph2.B worker B: undo and status on the stacks map, -f threading, binary suite | paiml-impl-worker | opus | a9883d521c7c3c654 |
| 4 | PMAT-161/ph3.C worker C: docs, changelog and help for the six-stack state dir | paiml-impl-worker | sonnet | ad15ac2d58d08486b |
| 5 | PMAT-161/ph4.B worker B: undo selects and restores only its own stack's generation | paiml-impl-worker | opus | — |
| 6 | PMAT-161/ph2.SG worker B: refuse restore in a multi-stack state dir | paiml-impl-worker | opus | — |
| 7 | PMAT-161/ph2.C2 worker C: docs claim narrowed, forjar-dogfood skill name and guard | paiml-impl-worker | sonnet | aa9deda06e790211a |
| 8 | PMAT-161/ph1.tr quorum width 3 triage research over issues, tickets and the named candidat | paiml-agy-delegate | opus | — |
| 9 | PMAT-161/ph1.trcx quorum width 6 (triage 3 + CRUX 3) in a throwaway clone | paiml-agy-delegate | opus | — |
| 10 | PMAT-161/ph2.S1fix worker: the multi-stack refusal must precede undo's destroy | paiml-impl-worker | opus | aebe6ae821a36e583 |
| 11 | PMAT-161/ph2.rename worker: a renamed stack is one lineage in the shared state dir | paiml-impl-worker | opus | a1fb6a286ec4c9f95 |
| 12 | PMAT-161/ph2.replay worker: undo's replay stamps the invoking stack's name, not the histor | paiml-impl-worker | opus | a08e6d67e5a7a534f |
| 13 | PMAT-161/ph3.q1 quorum width 5: three claim lanes, teamwork, crux on the PMAT-161 diff | paiml-agy-delegate | opus | — |
| 14 | PMAT-161/ph2.q1fix worker: the four round-1 quorum S1s | paiml-impl-worker | opus | a73723f59faaa593f |
| 15 | PMAT-161/ph2.q1fix2 worker: finish the four round-1 S1 fixes from the previous worker's tr | paiml-impl-worker | opus | aa47ec17552040953 |
| 16 | PMAT-161/ph3.q2 quorum width 3: refuters attack the state-stamp claims dossier | paiml-agy-delegate | opus | — |
| 17 | PMAT-161/ph3.q2b quorum width 3: refuters attack the committed state-stamp dossier | paiml-agy-delegate | opus | — |
| 18 | PMAT-161/ph2.q2fix worker: guard the second retention path; exact path comparison in stack | paiml-impl-worker | opus | aadc2468841068eb4 |
| 19 | PMAT-161/ph2.q2fix2 worker: finish PMAT-183 from the previous worker's tree | paiml-impl-worker | opus | — |
| 20 | PMAT-161/ph3.q3 quorum width 3: judges adjudicate the state-stamp claims against the refut | paiml-agy-delegate | opus | — |
| 21 | PMAT-161/ph4.merge quorum-review.sh width 3 for the merge helper on PR 473 | paiml-agy-delegate | opus | — |

Subagent slots: 3 (config); running peak observed 3; every lane through the agy delegate (`--sandbox`, per-lane shared clones).

## Verification (claimed vs re-run)

| what | orchestrator re-run |
|---|---|
| cargo test --lib -- state::stamp | exit 0 |
| binary falsification suites (undo interlock, state dir) | exit 0 |
| mutation: refusal removed → falsification RED | observed (receipt falsification block) |

## Falsification

- test: a_state_dir_holding_several_stacks_refuses_every_restore (with a_refused_undo_destroys_nothing_on_the_way_to_refusing, rename_cases::rollback_on_failure_is_refused_before_the_apply_writes_anything and rename_cases::a_renamed_stack_is_one_lineage_not_two_stacks; the per-name suite drives the built binary over temp state dirs)
- file: `tests/falsification_state_stamp_per_name.rs` (target `falsification_state_stamp_per_name`)
- reverted: the multi-stack threshold disabled: `names.len() <= 1` → `<= 99` in src/core/state/stamp/mod.rs (multi_stack_restore_refusal), applied with sed at HEAD, the target run, the file restored with git checkout (tree verified clean). Also measured at 5044ab1f: the threshold moved to three stacks (1 test red) and the guard call removed from cmd_undo (1 test red); no stack_conflict test moved under any mutation.
- observed: `test result: FAILED. 14 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.24s`

## Gaps

- Coverage and mutation (gate F) and the pv contract depth (gate G) are measured on the release head in docs/audits/dogfood-1.26.0-receipt.md, not per ticket.
- Status-line join rows: [U] (the session's transcript lives under the root project key; the worktree runs were not joined).

IMPL-PMAT-161-RECEIPT-END
