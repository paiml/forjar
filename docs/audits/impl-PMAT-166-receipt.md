# Implementation receipt — PMAT-166 — release.yml creates the draft prerelease after the clean-room gate (--draft --prerelease --verify-tag) and un-drafts after assets; binary-release.yml dispatch-only

verdict: DONE — merged into main by PR #479 (d9ba9222); quorum receipt `.quorum/PMAT-166-release-prerelease.json` (20 confirmed, 8 refuted); no waiver.

## Identity

| field | value |
|---|---|
| ticket | PMAT-166 (kind: code) |
| branch | PMAT-166-release-prerelease |
| PR | #479 |
| base_commit (receipt) | de4dee15dcfd048588a3575950d24456790fec9f |
| diff_sha256 (receipt) | 2887a32a47715e959921ebe236ab2d28e177bb59 |
| quorum | lanes 5, refuters/claim 3, judges 3 |
| crux systems | cargo-dist, GoReleaser, release-please / semantic-release, GitHub release events (published, prereleased, --latest), Homebrew and Nix asset consumers |
| agy teamwork | ran=True |
| pmat | analyze_vacuous_tests; vacuous in touched paths 0 |

## Plan

| phase | what | route |
|---|---|---|
| P1 | workflow edits (orchestrator-only) + tests/falsification_release_workflow_shape.rs (7 rules) + contract | direct + subagent:sonnet |
| P2 | quorum ×3, teamwork, crux, pmat; PMAT-170; receipt; workflow_dispatch falsifier (run 34110072499: failed at the release step, no release, no tag); PR | delegate + direct |

## Dispatch ledger

| # | description | type | model | agent |
|---|---|---|---|---|
| 1 | PMAT-166/ph3.q1 quorum width 5: claims, teamwork, crux on the release-workflow diff | paiml-agy-delegate | opus | a0af7b3f61b9b7be0 |
| 2 | PMAT-166/ph3.q2 quorum width 3: refuters attack the release-prerelease dossier | paiml-agy-delegate | opus | — |
| 3 | PMAT-166/ph3.q3 quorum width 3: judges adjudicate the release-prerelease claims | paiml-agy-delegate | opus | — |
| 4 | PMAT-166/ph2.shape worker: a falsification test for the release workflows' shape | paiml-impl-worker | sonnet | — |

Subagent slots: 3 (config); running peak observed 3; every lane through the agy delegate (`--sandbox`, per-lane shared clones).

## Verification (claimed vs re-run)

| what | orchestrator re-run |
|---|---|
| cargo test --test falsification_release_workflow_shape | 7 passed at HEAD; 6 failed against origin/main workflows (RED observed) |
| gh workflow run binary-release.yml -f tag=v0.0.0-falsifier-pmat166 | completed failure at "Create GitHub Release if missing"; gh release view → not found; no tag pushed |
| scripts/quorum-gate.sh | passed |

## Falsification

- test: the seven tests of falsification_release_workflow_shape (binary-release.yml dispatch-only; release.yml the only tag trigger; draft+prerelease+verify-tag on create with created= outputs; the assert string; the publish-release job shape; no cargo publish / registry token / continue-on-error outside mutation.yml; the backfill concurrency group and --prerelease)
- file: `tests/falsification_release_workflow_shape.rs` (target `falsification_release_workflow_shape`)
- reverted: both workflow files restored from origin/main with git show, the target run, the files restored with git checkout (tree verified clean)
- observed: `test result: FAILED. 1 passed; 6 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s`

## Gaps

- Coverage and mutation (gate F) and the pv contract depth (gate G) are measured on the release head in docs/audits/dogfood-1.26.0-receipt.md, not per ticket.
- Status-line join rows: [U] (the session's transcript lives under the root project key; the worktree runs were not joined).

IMPL-PMAT-166-RECEIPT-END
