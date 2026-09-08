# Implementation receipt — PMAT-206 — the 1.26.0 merges pushed CB-200 from 651 to 654 functions below grade A and gate B refused the cut; three behaviour-preserving reductions and one measurement fix bring it back to the ceiling with the ceiling untouched

verdict: DONE — merged into main by PR #482 (b2442fe0); quorum receipt `.quorum/PMAT-206-cb200-back-under-the-ceiling.json` (7 confirmed, 6 refuted); no waiver.

## Identity

| field | value |
|---|---|
| ticket | PMAT-206 (kind: code) |
| branch | PMAT-206-cb200-back-under-the-ceiling |
| PR | #482 |
| base_commit (receipt) | 2acefaec0c9fd59b7e34ccabf467a52d4f553b86 |
| diff_sha256 (receipt) | b8a4b1c2862e1f2c01c541ea1a46f7e9b48204a9 |
| quorum | lanes 5, refuters/claim 3, judges 3 |
| crux systems | SonarQube, CodeClimate, Ratchet-style debt gates, clippy with a baseline |
| agy teamwork | ran=True |
| pmat | analyze_vacuous_tests; vacuous in touched paths 0 |

## Plan

| phase | what | route |
|---|---|---|
| P1 | observe::classify as a table, the purifier decompositions, an example split behind one criterion helper | direct |
| P2 | the measurement: the ratchet was grading a stale pmat comply cache that pmat query --rebuild-index does not refresh | direct |
| P3 | five lanes then three refuter-judges; the JSON-regex rm -rf design withdrawn; the falsification test with a pmat shim; two merge reviews | delegate:quorum x2 + direct |

## Dispatch ledger

| # | description | type | model | agent |
|---|---|---|---|---|

Subagent slots: 3 (config); running peak observed 0; every lane through the agy delegate (`--sandbox`, per-lane shared clones).

## Verification (claimed vs re-run)

| what | orchestrator re-run |
|---|---|
| bash scripts/cb200-ratchet.sh | 651, exit 0, with the cache fresh and with a source newer than the cache |
| cargo test --test falsification_cb200_ratchet_measures_this_tree | 4 passed; 1 of 4 RED against the ratchet as it stood on main |
| bash scripts/dogfood/comply.sh | GATE B PASS |

## Falsification

- test: the four cases of falsification_cb200_ratchet_measures_this_tree (a stale cache is announced and removed before comply runs; a fresh cache is left alone; the ceiling still refuses 652; a missing cache root is skipped), with a pmat shim that records whether the cache directory existed when comply was invoked
- file: `tests/falsification_cb200_ratchet_measures_this_tree.rs` (target `falsification_cb200_ratchet_measures_this_tree`)
- reverted: scripts/cb200-ratchet.sh replaced by its origin/main version (git show origin/main:scripts/cb200-ratchet.sh), the target run, the file restored (tree verified clean)
- observed: `test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s`

## Gaps

- Coverage and mutation (gate F) and the pv contract depth (gate G) are measured on the release head in docs/audits/dogfood-1.26.0-receipt.md, not per ticket.
- Status-line join rows: [U] (the session's transcript lives under the root project key; the worktree runs were not joined).

IMPL-PMAT-206-RECEIPT-END
