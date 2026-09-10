# Implementation receipt — PMAT-225 — release goals: every ticket names the tag that shipped it, PRs are tagged as they merge, and the two-day cadence is a gate (forjar#506)

verdict: PASS — gate T is green on the committed ledger and red under its registered mutation; 13 fixture cases over temp repositories with a bare origin and a stub gh are green and each red arm is red by name; the two new gate A cases are red against main's scripts; three quorum lanes refuted two things in round 1, both fixed, and three lanes passed round 2 unanimously; the workspace suite, clippy, rustfmt, gates B and G are green.

## Identity

| field | value |
|---|---|
| ticket | PMAT-225, kind:code, orch:fable (`orch-basis:release`), roadmap row appended textually, `github_issue: 506` |
| issue | forjar#506 — filed from the measured state before the ticket was opened |
| branch | `PMAT-225-release-goals` on main `8e8e79e4` |
| commits | `bada9ff8` the declared side and the ticket rule; `ab9bddc8` gate T, tests, wiring, the daily workflow; `db298604` the cut flow end to end; `ca3f8d70` the quorum's two findings; then this receipt |
| session | the third ticket of one Claude session (PMAT-223, PMAT-224, PMAT-225). `goal.sh set` refused it (R-5, exit 2 naming PMAT-223); it proceeded on the operator's verbatim instruction quoted in `.quorum/evidence/goal-pmat.md`. Every lane was declared with `goal.sh worker`. |
| turns | k measured from the transcript (distinct assistant message ids): 75 at the PMAT-224 receipt, 123 at the round-2 verdict; basis `docs/audits/impl-estimates.jsonl` rows for PMAT-225 |

## What was measured before a line was written

- The newest tag was `v1.27.0`, created 2026-09-08T22:10:01Z (tagger date; the tagged commit is 21:46:04Z). Eight PRs naming eight tickets had merged since. No roadmap row (174), no receipt and no issue named the release any of them would ship in; the repository had zero GitHub milestones.
- The only ticket-to-tag link was the squash subject, and it is lossy: the 1.27.0 release commit says `PMAT-212/213/214` and a `PMAT-[0-9]+` scan sees one ticket of three.
- GitHub's `mergedAt` for a release's own PR is one second AFTER the tag's commit date (#490: 21:46:05Z against 21:46:04Z), so a window bounded above by time loses the PR that made the release; ancestry must decide, as `lib/window.sh` already did for the open window.
- PR #496's head branch is `PMAT-218-details-content-hash-asks-the-machine`; PMAT-218 is no roadmap row and `docs/audits/impl-PMAT-218-receipt.md` does not exist; the title names PMAT-219, whose receipt exists. Gate A takes the first id in the branch: the 1.28.0 cut would have failed gate A on a receipt nobody should write.
- The Friday-only crates.io rule is not live: 1.26.0 and 1.27.0 both published on Tuesday 2026-09-08 (crates.io timestamps); the 1.26.0 release receipt records it `overridden(operator 2026-09-06)`.
- `pmat work edit` drops unknown top-level row fields but keeps labels (measured on `kind:`), so a LABEL is the durable per-ticket link.

## The shape

The `paiml-implement` goal shape (AUTO-IMPL-SKILL-002): a DECLARED side a human writes, a MEASURED side a script reads, one status line joining them with `basis=` on every number, a refusal wherever they disagree, nothing enumerated from memory.

- **Declared:** `docs/roadmaps/releases.yaml` — `cadence_days: 2`, `floor: v1.25.0` (the first tag whose window names a pmat ticket), `harness_floor: v1.26.0` and `dogfood_floor: v1.26.0` (gate A's and the dogfood regime), one row per tag with `cut` (the tag's creation instant), `prs`, `tickets`, `dogfood`, `crux`, and the open goal `next: {tag: v1.28.0, due: 2026-09-10T22:10:01Z}`. Every row was printed by `scripts/release-goal.sh window TAG` and pasted; none was typed. Every roadmap row a tag's window names carries `release:<tag>` (25 rows); the eight tickets merged since v1.27.0 and PMAT-225 itself carry `release:v1.28.0`; PMAT-219 carries `alias:PMAT-218`.
- **Measured:** `scripts/dogfood/tagged.sh` prints one `GATE T PASS|FAIL` line. T1 the ledger parses and the floors are tags origin carries; T2 every reachable tag at or above the floor has a row whose cut, prs and tickets are exactly what git and GitHub say, and no row names a tag origin lacks; T3 from `harness_floor` every PR names a roadmap ticket and no stray id; T4 every shipped ticket carries `release:<tag>`, no row claims a release it was not in, every ticket merged since the newest tag carries `release:<next.tag>`; T5 from `dogfood_floor` the receipt (END marker, one verdict) and the crux document are at HEAD; T6 `next.tag` is above the newest tag, `next.due` is exactly the newest cut plus `cadence_days`, and the cut is not OVERDUE — past due with PRs merged and Cargo.toml still at the tagged version is red by the hour; a bumped Cargo.toml is a cut in flight. Everything at HEAD. A gh that cannot answer is UNMEASURED and red. `DOGFOOD_NOW` pins the clock for fixtures only.
- **The one ticket rule** (`lib/window.sh` `dogfood_pr_tickets`, shared by gate A and gate T): every `PMAT-<n>` in the branch and title with the slash shorthand expanded, resolved against the registry at HEAD; the FIRST id is gate A's receipt address; an id that is no row resolves only through a row labelled `alias:<id>` and is never skipped for the next one; two rows declaring one alias name none.
- **The status line:** `make release-goal` → `v1.28.0 ███████░░░ 36h/48h left=11h · 8 merged, 8 tagged · due 2026-09-10T22:10:01Z basis=docs/roadmaps/releases.yaml:L55 window=v1.27.0..HEAD(…) ledger=worktree`. `scripts/release-goal.sh sync` labels the open window (`--check` edits nothing, exit 1 when one is missing); `tag`, `alias` one label; `cut TAG --next NEXT` after `git tag` books the measured row, declares NEXT due exactly `cadence_days` later, and moves a goal that missed the cut to NEXT. Textual edits inside one row only; `git diff` of the roadmap on this branch is label lines, `updated:` stamps and the PMAT-225 row.
- **The daily instrument:** `.github/workflows/release-goal.yml`, 05:00 UTC and on demand: gate T with the token, then gates C and D as `ci.yml` runs them; one `release-goal` issue kept open while red, closed when green; `actionlint` clean; the shape test pins schedule, token, full clone, order, upsert, close, and that nothing is swallowed. The label exists on GitHub.
- **Wiring:** `make dogfood-release` runs T after A and E and before F; the mutations guard's REQUIRED list has 10 entries; the forjar-dogfood skill table, CLAUDE.md, `contracts/forjar-dogfood-coverage-v1.yaml` (FALSIFY-DF-016..020) and CHANGELOG `[Unreleased]` carry it.

## Falsification

| what | RED | GREEN |
|---|---|---|
| gate T's registered mutation | `86400` → `86401`: `GATE T FAIL next.due is declared 2026-09-10T22:10:01Z … puts the due instant at 2026-09-10T22:10:03Z — the declared goal and the derived one disagree`, exit 1 (`docs/audits/logs/PMAT-225-gate-T-mutant.log`) | reverted: `GATE T PASS 5 tagged release(s) since v1.25.0 … 8 of 8 PR(s) merged since v1.27.0 carry release:v1.28.0` (`docs/audits/logs/PMAT-225-gate-T.log`) |
| the two gate A cases | against main's `harness.sh` + `lib/window.sh` (stashed): `gate_a_a_first_id_that_is_not_a_roadmap_row_is_named_and_red` and `gate_a_a_stray_id_resolves_through_the_row_that_declares_it_as_alias` FAILED, 0 passed 2 failed | popped: 19 passed |
| the 12 gate T cases and the cut flow | `scripts/dogfood/tagged.sh` does not exist on main: every case panics at "the gate under test must exist" | 12 passed; 1 passed (`docs/audits/logs/PMAT-225-gate-tests.log`) |
| what the suite caught while it was written | a `>/dev/null` on the window call swallowed the verdict (every case: "a death, not a verdict"); a forgiven `\|\| true` call exited the script silently; the fixture rendered the cut instant in local time and the gate refused it by two hours | all gone; `docs/audits/jidoka.jsonl` carries the five rows |

## Review record

| round | lanes | verdict | what was refuted |
|---|---|---|---|
| plan grill (before any code) | 1, `--mode plan` | `do-not-implement-as-written`, 7 findings | a stray id skipped for the next one would let a forgotten roadmap row fall through to an older ticket in the body and pass gate A on its receipt — now a stray id is red unless a row declares the alias; the cadence must count from the tag's creation instant, not the commit's; the status tool must read the working tree; the cadence arm must pass a non-overdue main at the tagged version; a free-rider label at the cut moves to the next goal; the grep-under-pipefail printer |
| quorum round 1 (db298604) | 3, `--mode plan` | 3/3 FAIL, C1..C15 confirmed by two lanes and 14/15 by the third | the resolver's refusal of an alias declared by two rows was reachable and untested (all three); `release-goal.sh window` omitted the receipt keys, so the rows were not byte-identical to its output (lane 1, C7) |
| quorum round 2 (ca3f8d70) | 3, `--mode plan` | 3/3 PASS, lane-reduce agreed, zero dissent | nothing; H2's second half left open by all three: `cut` does not itself verify a receipt exists, gate T refuses an unbacked `dogfood:` key downstream with `not at HEAD` |
| CRUX survey | 1, `--mode plan`, documentation memory only, every third-party cell `[X]` | `.quorum/evidence/goal-crux.md` | accept(a declared cadence file measured mechanically; labels as the per-change link) and reject(an exact-second due instant; a repository-wide red on a late release) — answered below |

Every lane verdict was a claim: the orchestrator re-ran gate T and its mutation, the five `window` diffs against the ledger (all identical), the six test binaries, bashrs on five scripts, actionlint, gates B and G, clippy and the workspace suite.

## Gates measured

| gate | result | log |
|---|---|---|
| `cargo test --workspace --locked --no-fail-fast` on ab9bddc8 | exit 0 — 322 binaries, 19,643 passed, 0 failed | `docs/audits/logs/PMAT-225-workspace.log` |
| `cargo clippy --all-targets --locked -- -D warnings` | exit 0 | `docs/audits/logs/PMAT-225-clippy.log` |
| the six gate test binaries on ca3f8d70 | 12 + 1 + 19 + 5 + 4 + 9 passed, 0 failed | `docs/audits/logs/PMAT-225-gate-tests.log` |
| gate T live | PASS | `docs/audits/logs/PMAT-225-gate-T.log` |
| gate B `comply.sh` | PASS, 14 gate scripts at 0 bashrs errors | `docs/audits/logs/PMAT-225-gate-B.log` |
| gate G `contracts.sh` | PASS, 40 contracts, citations resolve | `docs/audits/logs/PMAT-225-gate-G.log` |
| bashrs on the five touched scripts, actionlint | 0 errors; 0 findings | `docs/audits/logs/PMAT-225-bashrs.log` |
| `make release-goal`, `window` for every tag | the status line; five rows | `docs/audits/logs/PMAT-225-release-goal.log` |
| gate F's mutation arm | NOT MEASURED on this host (PMAT-216, `cargo mutants` interrupted); the registered mutation was run by hand instead | — |

## Routing and dispatch

| phase | class | `route.sh` (verbatim) | executed by |
|---|---|---|---|
| 0 plan grill | plan | `route=agy-plan w=1.00 basis=absent effort=1[U]` | delegate, one lane |
| 1 declared side | impl | `route=agy-goal w=1.00 basis=absent note=fable-binding effort=1[U]` | self — the phase edits `docs/roadmaps/roadmap.yaml` textually (a measured lane hazard) and shares `lib/window.sh` with phase 2 in one worktree; named deviation |
| 2 gate T, tests, wiring, workflow | cross | `route=agy-goal w=1.00 basis=absent effort=1[U]` | self — same reason; workers and lanes may not edit `.github/workflows` |
| 3 review | review | `route=agy-quorum w=1.00 basis=absent effort=1[U]` | delegate, two rounds of three lanes |
| 4 CRUX | research | `route=agy-grillme w=1.00 basis=absent effort=1[U]` | delegate, one plan lane (the lane ran `--mode plan`; the brief named both) |
| 5 receipt, push, PR | orchestration | `route=self w=100.00 basis=absent` | self |

Dispatch ledger: four `paiml-agy-delegate` (opus) dispatches, descriptions `PMAT-225/ph1.grill`, `PMAT-225/ph3.delegate`, `PMAT-225/ph3.delegate2`, `PMAT-225/ph4.crux`; slots 3, live never above 1, denials 0; no worker subagent, no resume, no Workflow. Two delegates hit their 30-turn cap after their lanes had finished; the orchestrator read the lane files and ran `lane-reduce.sh` itself (round 1: not agreed, 3 FAIL; round 2: agreed). Conversation ids are in `.quorum/evidence/goal-agy.md`, shortened.

## Gaps, named

- `release-goal.sh cut` does not verify that the dogfood receipt and crux document exist before writing their paths; gate T refuses the unbacked key at HEAD (`not at HEAD`). Write-time refusal is unimplemented.
- The skill's own half — labelling a ticket `release:<next.tag>` at mint time — is outside this repository: filed as paiml/paiml-implement#72.
- The cadence arm counts from the tag's creation instant; a re-tag of an existing version would move the clock, which the ledger's `cut` would then contradict (red, correctly), but nothing prevents the re-tag itself.
- Tags below v1.25.0 are not declared and not checked; tags below v1.26.0 may hold unticketed PRs by declaration.
- v1.28.0 is due 2026-09-10T22:10:01Z and is not cut by this ticket (its own ticket: the bump, CHANGELOG, `crux-1.28.0.md` rows for the four `[Unreleased]` paragraphs, `make dogfood-release`, the tag, `release-goal.sh cut v1.28.0 --next v1.29.0`, the receipt).

IMPL-PMAT-225-RECEIPT-END
