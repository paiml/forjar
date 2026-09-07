# Implementation receipt — PMAT-204 — a file resource whose path contains 666 or 777 could not be applied: bashrs SEC017 read the path as a chmod mode, and forjar now refuses a world-writable mode itself

verdict: DONE — merged into main by PR #None (sha per git log); quorum receipt `.quorum/PMAT-204-chmod-path-false-positive.json` (7 confirmed, 9 refuted); no waiver.

## Identity

| field | value |
|---|---|
| ticket | PMAT-204 (kind: code) |
| branch | PMAT-204-chmod-path-false-positive |
| PR | #None |
| base_commit (receipt) | 0776fd82a57158a87b7dc3e7602a12dee52db893 |
| diff_sha256 (receipt) | 3837cc58f1e7369292c803a015375fcd75017b18 |
| quorum | lanes 5, refuters/claim 3, judges 3 |
| crux systems | Ansible file module, Puppet file, Chef file, Terraform local_file, Nix writeTextFile, shellcheck/bashrs linters |
| agy teamwork | ran=True |
| pmat | analyze_vacuous_tests; vacuous in touched paths 0 |

## Plan

| phase | what | route |
|---|---|---|
| P1 | the falsification suite and the first rule (mode argument parsed) | subagent:opus |
| P2 | four adversarial rounds and five merge reviews; the rule became redact-the-path-and-re-lint | delegate:quorum x4 + direct |
| P3 | the review-round pins split into their own test file; the receipt | direct |

## Dispatch ledger

| # | description | type | model | agent |
|---|---|---|---|---|

Subagent slots: 3 (config); running peak observed 0; every lane through the agy delegate (`--sandbox`, per-lane shared clones).

## Verification (claimed vs re-run)

| what | orchestrator re-run |
|---|---|
| cargo test --test falsification_chmod_path_is_not_a_mode | 22 passed across the two files |
| 50 shapes measured at origin/main and at the head | 4 refused to accepted (the bug), 13 accepted to refused, no regression |

## Falsification

- test: the 18 tests of falsification_chmod_path_is_not_a_mode (both directions of every round: the path false positive, the world-writable refusal, the refuted multi-chmod shapes, the whitespace-padded modes, the plain-path-literal restriction, any octal width, and the two declared undecidable shapes)
- file: `tests/falsification_chmod_path_is_not_a_mode.rs` (target `falsification_chmod_path_is_not_a_mode`)
- reverted: src/core/purifier.rs restored to 0776fd82 and src/core/purifier_sec017.rs removed from the build, the target run, both restored (tree verified clean)
- observed: `error: 1 resource(s) failed`

## Gaps

- Coverage and mutation (gate F) and the pv contract depth (gate G) are measured on the release head in docs/audits/dogfood-1.26.0-receipt.md, not per ticket.
- Status-line join rows: [U] (the session's transcript lives under the root project key; the worktree runs were not joined).

IMPL-PMAT-204-RECEIPT-END
