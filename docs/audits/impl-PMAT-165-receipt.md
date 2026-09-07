# Implementation receipt — PMAT-165 — scripts/publish-from-tag.sh and make publish-from-tag TAG=: detached worktree of the tag, local credentials only, bounded index poll

verdict: DONE — merged into main by PR #480 (2acefaec); quorum receipt `.quorum/PMAT-165-publish-from-tag.json` (19 confirmed, 8 refuted); no waiver.

## Identity

| field | value |
|---|---|
| ticket | PMAT-165 (kind: code) |
| branch | PMAT-165-publish-from-tag |
| PR | #480 |
| base_commit (receipt) | d9ba922264ecdda7f18917a160c7e8c4e2662f71 |
| diff_sha256 (receipt) | 47eeecc366d62ddf73794c48948bac8100e4d3d0 |
| quorum | lanes 5, refuters/claim 3, judges 3 |
| crux systems | cargo publish --workspace, cargo-release, release-plz, cargo-workspaces, crates.io Trusted Publishing |
| agy teamwork | ran=True |
| pmat | analyze_vacuous_tests; vacuous in touched paths 0 |

## Plan

| phase | what | route |
|---|---|---|
| P1 | script with refusals (exit 2), scratch outside the worktree, env -u CARGO_REGISTRY_TOKEN, cargo info poll; shim-backed harness | subagent:opus |
| P2 | quorum ×3, teamwork, crux, pmat; PMAT-184..187 fixes; receipt; PR | delegate + direct |

## Dispatch ledger

| # | description | type | model | agent |
|---|---|---|---|---|
| 1 | PMAT-165/ph2.PP worker: Makefile publish-from-tag poka-yoke with a falsification test | paiml-impl-worker | sonnet | — |
| 2 | PMAT-165/ph2.PP worker: Makefile publish-from-tag poka-yoke with a falsification test | paiml-impl-worker | sonnet | a2ab7b8de87c79728 |
| 3 | PMAT-165/ph2.PP2 worker: finish publish-from-tag from the previous worker's tree | paiml-impl-worker | sonnet | aacfcb6a594fe0000 |
| 4 | PMAT-165/ph3.q1 quorum width 5: claims, teamwork, crux on the publish-from-tag diff | paiml-agy-delegate | opus | — |
| 5 | PMAT-165/ph2.PPfix worker: the four publish-from-tag review findings | paiml-impl-worker | opus | a7a4e2ea538f43bff |
| 6 | PMAT-165/ph3.q2 quorum width 3: refuters attack the publish-from-tag dossier | paiml-agy-delegate | opus | — |
| 7 | PMAT-165/ph3.q2b quorum width 3: refuters attack the committed publish-from-tag dossier | paiml-agy-delegate | opus | — |
| 8 | PMAT-165/ph3.q3 quorum width 3: judges adjudicate the publish-from-tag claims | paiml-agy-delegate | opus | — |

Subagent slots: 3 (config); running peak observed 3; every lane through the agy delegate (`--sandbox`, per-lane shared clones).

## Verification (claimed vs re-run)

| what | orchestrator re-run |
|---|---|
| cargo test --test falsification_publish_from_tag | 10 passed |
| mutation: worktree add → clone (line 114) | case c RED (9 passed, 1 failed) — the first recorded attempt had not applied the mutation; corrected |
| scripts/quorum-gate.sh | passed |

## Falsification

- test: c_dry_run_publishes_only_from_a_worktree_of_this_repo (with the nine other shim-backed cases of falsification_publish_from_tag: refusals a, b; scratch f; index poll g, h, i; dev-dependency j)
- file: `tests/falsification_publish_from_tag.rs` (target `falsification_publish_from_tag`)
- reverted: the `git worktree add` line of scripts/publish-from-tag.sh replaced with `git clone --no-local` plus `checkout --detach` (a copy, not a worktree of the repository), the target run, the script restored from a copy (tree verified clean)
- observed: `test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s`

## Gaps

- Coverage and mutation (gate F) and the pv contract depth (gate G) are measured on the release head in docs/audits/dogfood-1.26.0-receipt.md, not per ticket.
- Status-line join rows: [U] (the session's transcript lives under the root project key; the worktree runs were not joined).

IMPL-PMAT-165-RECEIPT-END
