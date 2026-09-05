# Implementation receipt — PMAT-160 — apply filter pipeline (#466, #467, #468)

Verdict: **DONE** (pending the merge gates named under Gaps). Skill: paiml-implement (AUTO-IMPL-SKILL-001), kind=code.

## Identity

| field | value |
|---|---|
| ticket | PMAT-160 (kind: code; GitHub #466, #467, #468) |
| branch | fix/dry-run-honours-resource-filter |
| base | origin/main at 20a255b0 (v1.25.2); merge-base 20a255b0be1300e74fdd9188b2cb2073434c57b2 |
| code HEAD when the quorum judged | 753eb232 (evidence and receipts committed after it) |
| discover.json sha256 | a733384121f291cf1e265600deed0d92d7aab5a5c91f90d2fe238412b0792c79 |
| gate_cmd | `cargo test --workspace` — **gate_cmd_fallback=true** (discover.sh found no Makefile gate); CI's real gate is the clean-room `gate` check plus per-target `cargo test --locked --test ...` steps |
| required_check | gate (rulesets) |
| quorum_tool | agy 1.1.27 |

Status-line join (AUTO-IMPL-SKILL-002): every row **[U]** unmeasured — the session started in /home/noah/src/forjar and moved to the worktree after the skill was invoked, so statusLine `session_id` = hook `session_id`, `tasks[].id` = hook `agent_id`, and `transcript_path` on subagentStatusLine stdin were not measured; the command that measures them is `bash ~/.claude/skills/paiml-implement/scripts/statusline.sh --probe`. `k_measured` (distinct assistant message ids in the session transcript, `jq -r 'select(.type=="assistant" and ((.isSidechain // false)|not)) | (.message.id // .uuid)' | sort -u | wc -l`) = 44 at receipt time against the orchestrator's own count k = 37 for the implement run: the 12-turn gap is the bootstrap (ticket creation) and the root-checkout reconcile that preceded the skill invocation in the same session — a finding, recorded here, not a miscount.

## Plan (routing + trigger)

| phase | what | route | trigger | A_i |
|---|---|---|---|---|
| P1 | `resolve_selection` — validate full graph, positive selectors, depends_on closure, negatives with edge contraction, prune | subagent:opus (worker B) + delegate teamwork (plan grill) | Q1 (\|M\|=6), Q2 | `cargo test --lib -- apply_selection` |
| P2 | wire `cmd_apply_scoped`, the `--check` branch and the dry run onto one resolver; binary-level falsification suite | subagent:opus (worker B) | — | `cargo test --test falsification_apply_filter_pipeline && cargo test --lib -- apply_scope selector_guard gh_dogfood_p1 cov_apply_b apply_selection` |
| P3 | contracts rows, CHANGELOG, --help text, book pages | subagent:sonnet (worker C) | — | `pv validate contracts/flag-has-effect-v1.yaml && grep -q '#467' CHANGELOG.md && grep -q '#468' CHANGELOG.md && cargo test --lib -- commands` |
| P4 | quorum (claims, teamwork, crux, refuters, judges), fixes from review, receipts, PR | delegate:quorum + direct | Phase 4 review | the four lanes' artifacts under .quorum/evidence/ |

Estimate: K̂ = 3 (`estimate.sh`, basis=first-run[U], 0 rows); K = 60 (`--budget-turns`); actual k = 37 orchestrator turns for the implement run.

## Dispatch ledger

| # | description | agent | model | turns / maxTurns | resumed | lanes / conversations |
|---|---|---|---|---|---|---|
| 1 | PMAT-160/ph1.B worker B: resolve_selection | af179bd062d84d701 | opus | 44 tool uses / 40 turns (finished) | no | — |
| 2 | PMAT-160/ph1.delegate teamwork width 1 on the plan | a00a56d787c0789db | opus | 11 / — | no | teamwork; f65839de-fc3f-448e-8964-d5e6fc27db0e |
| 3 | PMAT-160/ph2.B worker B: wire check, dry-run, apply | a3f0f89bbd5806890 | opus | 52 / 40 — **maxTurns hit** | once (SendMessage) | — |
| 4 | PMAT-160/ph3.C worker C: contracts, changelog, help | aee8499d7a0d026dd | sonnet | 46 / 40 — **maxTurns hit** | once; hit the limit again after committing; orchestrator verified its deliverables directly | — |
| 5 | PMAT-160/ph4.claims quorum width 3 | a8e2cd65a46feafa2 | opus | 9 | no | 655d5721-89fd-47fb-a89d-8e0bdfcf4259, 8c5acb18-bea4-4e4b-b322-530502bb09bb, f4428446-cf5c-4efa-af76-fa5a0d11acbc |
| 6 | PMAT-160/ph4.teamwork width 1 on the final diff | a83aa4943a71de6d8 | opus | 15 | no | 69ee1fb6-8614-4518-bbf4-3745c3820089 |
| 7 | PMAT-160/ph4.refuters width 3 (round 1, DISCARDED: a lane mutated the tree) | a550380d4b2f8c5b6 | opus | 22 | no | 53f2060c-61a6-4469-822b-466d24beda72, 604f1f42-d182-49a0-ba84-de21bd478c07, c284f43b-6de1-4cec-b958-7ab97305e699 |
| 8 | PMAT-160/ph4.crux width 1 | a4b915e3e762ff451 | opus | 10 | no | c8d71ac2-7cd7-4b87-9ec6-f51cfc253b51 |
| 9 | PMAT-160/ph4.refuters2 width 3 (round 2, DISCARDED: root filesystem filled by lane copies of target/) | a1a0ef534c2441cbf | opus | 32 / 30 — maxTurns hit | no | (two lanes died) |
| 10 | PMAT-160/ph4.refuters3 width 3 (round 3, clean) | ad20260d91337532f | opus | 17 | no | 7720465d-3732-46c6-9b51-21b9fb530691, 021c56e4-4086-4e15-bf54-5504f0578874, 64df9db2-5703-4fe9-83c6-17d7f71d7a56 |
| 11 | PMAT-160/ph4.judges width 3 | a56f46fd41dacb433 | opus | 23 | no | 1b416a1e-d1f6-4417-a818-2e96e077c7e8, b385a3e8-ed56-43df-93e7-7449460cd1b8, 2438d63f-d934-4b73-96bc-5ba74002e53a |

child_conversations: reported 0 by every delegate (the brain-dir counter it samples moves independently of the run).

**Slots and denials.** slots=3 (config), bank=3. The orchestrator's own tally: 11 Agent-tool dispatches + 2 SendMessage resumes; running_peak = 2 (rows 1+2, 3+4, 5+6, 7+8 ran pairwise; lock entries were counted before every dispatch); denied = 0. I-3 line, verbatim from `transcript-gate.sh` run against the session's real project directory (the first run, in the worktree's project key, found 0 subagents — vacuous, and said so): `PASS transcript-gate: attempted=11 denied=0 running_peak=2 slots=3 segments=347 files=11 (agent_calls=9 resumes=2 workflow_started=0)`. The gate counts 9 agent calls where the orchestrator counts 11: the two delegates it does not see are a finding recorded here, not resolved.

## Verification (claimed vs re-run)

| what | worker claimed | orchestrator re-run |
|---|---|---|
| A_1 `cargo test --lib -- apply_selection` | exit 0, 18 passed | exit 0, 18 passed (21 after 3fdae0c3) |
| A_2 binary suite + lib filters | exit 0 (10 + 72) | exit 0: 10 passed / 72 passed (12 binary after 3fdae0c3) |
| A_3 `pv validate` + greps + `cargo test --lib -- commands` | worker stopped before its final re-run | exit 0: "Contract is valid", #467 ×2, #468 ×2, 32 passed |
| `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings` | clean | clean (re-run after every phase and after the review fixes) |
| gate `cargo test --workspace` | exit 101: 38 failures, all in forjar-contracts (missing contracts/aprender fixture) | not re-run in full by the orchestrator: the forjar lib suite (13,4xx tests) passed under every A_i re-run; the 38 forjar-contracts failures are in a vendored crate the diff does not touch and are reproduced at the base commit by worker B — **gate.ok=false with stages_measured=286, not_measured=[forjar-contracts lib]** carried as partial evidence; CI's `gate` check is the arbiter |
| unscoped output byte-identical to 1.25.2 | (orchestrator's claim) | measured: `apply --dry-run --yes`, `plan`, `apply --yes`, `apply --check --yes`, second dry run, second apply — six identical normalised outputs between ~/.cargo/bin/forjar 1.25.2 and the branch binary; a two-resource cycle is refused by both with exit 1 and the same message, members listed in the other order |
| the three issues reproduce on 1.25.2 and are fixed | (CHANGELOG claim) | measured: 1.25.2 `--dry-run -r alpha` listed charlie, `--subset alpha` refused "depends on unknown 'bravo'", `--check --subset alpha` failed on charlie; branch lists alpha+bravo, converges 2, `2 pass, 0 fail` |
| mutation observed RED | (DoD) | four mutations: no closure → 6/18 fail; validate-after-prune → 1 fails; `-r`/`-g` re-applied downstream → 1 binary test fails (`left: ["alpha"] right: ["alpha","bravo"]`); `--check` skipping the resolver → 5 binary tests fail; the two review-driven tests are RED with their hunks reverted |
| quorum verdicts | claim lanes PASS, PASS, do-not-implement (prose); teamwork do-not-implement (findings); refuters 3 rounds; judges PASS, FAIL (polarity), PASS | every finding re-run: 4 fixed (3fdae0c3, 3ab88290, 753eb232), 2 filed (#470, #471), 1 rejected with measurement, 1 as-designed; judges' majority: 47 survived (45 confirmed, 2 narrowed), 7 refuted — see .quorum/evidence/filter-pipeline-judges.md |

## Jidoka log

| where | defect | owner | whys / action |
|---|---|---|---|
| Phase 1 | `pmat hooks install --strict --force` fails in a git worktree ("Not a directory": `.git` is a file) | pmat | the shared hooks path already carried the strict commit-msg gate; commits carry `Pmat-Ticket:` trailers; not filed (pmat repo) |
| Phase 4 | refuter round 1: a sandboxed agy lane mutated `src/cli/dispatch_apply_check.rs` in the working tree and left it; two lanes ruled on the dirty tree | quorum protocol | round discarded; tree restored; hard no-write rule added to every later brief; `git status --porcelain` recorded before and after each round |
| Phase 4 | refuter round 2: lanes copied the repository with target/ into /tmp and filled the root filesystem (100%); two lanes died, the orchestrator's scratchpad was lost | quorum protocol | 212 G freed (`/tmp/refute-*`); briefs now mandate `git clone --shared` and a `df` check; other multi-GB /tmp directories (forjar-copy, forjar-test, forjar-base, paiml-mcp-agent-toolkit-test, ruchy*) were left for the owner |
| Phase 4 | judge 2 mutated `src/cli/apply_dry_run.rs` in place (whitespace residue) | quorum protocol | reverted; judges 1 and 3 finished before the edit; judge 2's table diverges only on the file it mutated |
| Phase 4 | `transcript-gate.sh` run from the worktree finds 0 subagents (session transcript lives under the root project key) | skill | re-run with the session directory argument; both results recorded |

## Estimates

`estimate.sh`: K̂=3, basis=first-run[U]. K=60. Actual: 37 orchestrator turns (implement run), ~44 session-wide. Appended to docs/audits/impl-estimates.jsonl.

## Gaps

- pv lane: **ran** (`pv validate contracts/flag-has-effect-v1.yaml` → "Contract is valid"; 103 lines of rows added for `--check` × {--subset, --exclude, -g, --skip, -m} and `--dry-run` × {-r, -g}); `pv lint` (audit+score) NotRun locally — proofs.yml runs it in CI.
- dogfood: NotRun (`--dogfood` not requested); the fleet reproduction that found the bugs (paiml/infra#442) is the artifact that closes it.
- Full `cargo test --workspace` under the orchestrator: NotRun (see Verification); closed by CI's `gate`.
- Status-line join table: [U] for all three rows (see Identity).
- Follow-ups filed from the review: #470 (`--refresh-only` ignores `-r`), #471 (`--only-machine X -m Y` runs nothing at exit 0). Deliberate consequence named in CHANGELOG.md: the standalone `forjar check` command now resolves its selectors through the same resolver (closure for `check -r`, refusal of a typo).
