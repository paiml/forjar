# Implementation receipt — PMAT-163 — forjar-dogfood v1: eight standing gates, Make targets, surface ledger, skill, ci.yml dogfood job

verdict: DONE — merged into main by PR #476 (ccfe2393); quorum receipt `.quorum/PMAT-163-forjar-dogfood.json` (31 confirmed, 5 refuted); no waiver.

## Identity

| field | value |
|---|---|
| ticket | PMAT-163 (kind: code) |
| branch | PMAT-163-forjar-dogfood |
| PR | #476 |
| base_commit (receipt) | 6b80b748c230a4b95dd53b46c262b7b27f37c8a3 |
| diff_sha256 (receipt) | bb8dcc384c0d6d8e68be33465a8d7137b5d0c6af |
| quorum | lanes 5, refuters/claim 3, judges 3 |
| crux systems | Rust: cargo publish --dry-run, cargo-semver-checks, cargo-mutants, Kubernetes SIG Release: conformance and release-blocking jobs, Debian: autopkgtest, piuparts, reproducible builds, Terraform: acceptance tests (TF_ACC) before a release |
| agy teamwork | ran=True |
| pmat | analyze_vacuous_tests; vacuous in touched paths 1 |

## Plan

| phase | what | route |
|---|---|---|
| P1 | scripts/dogfood/{comply,surface,docs,contracts,coverage,crux-reconcile,release-check}.sh, Make targets, surface ledger | subagent:opus (worker B) + delegate:teamwork |
| P2 | skill forjar-dogfood, guard tests, ci.yml dogfood job (orchestrator-edited), fixes PMAT-178..181 | subagent:opus + direct |
| P3 | quorum ×3, crux, pmat; receipt; merge review (lane 1 found the guard step missing a binary; fixed 14bac2ef); PR | delegate + direct |

## Dispatch ledger

| # | description | type | model | agent |
|---|---|---|---|---|
| 1 | PMAT-163/ph2.DF worker: forjar-dogfood v1 — the eight standing gates, mechanical scripts,  | paiml-impl-worker | opus | ac0329f7f668e36ec |
| 2 | PMAT-163/ph2.DF2 worker: continue forjar-dogfood v1 from the RED commit, committing each g | paiml-impl-worker | opus | aa27f4ff8199c3bb5 |
| 3 | PMAT-163/ph2.DF3 worker: finish forjar-dogfood gates F, H, release-check, Make, contract,  | paiml-impl-worker | opus | ad5b0b43a88f8f08c |
| 4 | PMAT-163/ph3.q1 quorum width 5: claims, teamwork, crux on the forjar-dogfood diff | paiml-agy-delegate | opus | — |
| 5 | PMAT-163/ph2.q1fix worker: the four forjar-dogfood review findings | paiml-impl-worker | opus | a6420c9597b665508 |
| 6 | PMAT-163/ph2.arm5 worker: release-check reads the committed quorum receipt per merged PR | paiml-impl-worker | sonnet | — |
| 7 | PMAT-163/ph3.q2 quorum width 3: refuters attack the forjar-dogfood dossier | paiml-agy-delegate | opus | — |
| 8 | PMAT-163/ph3.q3 quorum width 3: judges adjudicate the forjar-dogfood claims | paiml-agy-delegate | opus | — |

Subagent slots: 3 (config); running peak observed 3; every lane through the agy delegate (`--sandbox`, per-lane shared clones).

## Verification (claimed vs re-run)

| what | orchestrator re-run |
|---|---|
| make dogfood against the built binary | exit 0 |
| guard test binaries (3) | exit 0 |
| mutation: release-check window / skill name | observed (receipt falsification block) |

## Falsification

- test: its_first_frontmatter_line_declares_the_unshadowable_name (with exactly_one_skill_claims_the_name and nothing_in_the_tree_is_still_called_dogfood)
- file: `tests/falsification_dogfood_skill_is_named.rs` (target `falsification_dogfood_skill_is_named`)
- reverted: the `name: forjar-dogfood` line deleted from .claude/skills/forjar-dogfood/SKILL.md with sed, the target run, the file restored from a copy (tree verified clean)
- observed: `test result: FAILED. 2 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`

## Gaps

- Coverage and mutation (gate F) and the pv contract depth (gate G) are measured on the release head in docs/audits/dogfood-1.26.0-receipt.md, not per ticket.
- Status-line join rows: [U] (the session's transcript lives under the root project key; the worktree runs were not joined).

IMPL-PMAT-163-RECEIPT-END
