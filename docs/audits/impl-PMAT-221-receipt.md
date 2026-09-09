# Implementation receipt — PMAT-221 — the planner names what it did not probe

verdict: DONE — a converged resource whose declared build I/O this host could not measure is now named on every plan surface, per (resource, machine), with the reason and the instrument that can answer; the action is unchanged. Closes forjar#497; the probe-map keying the review located is filed as forjar#499.

## Identity

| field | value |
|---|---|
| ticket | PMAT-221 (kind: code, `orch:fable`, `orch-basis:M>=3`) |
| issue | forjar#497, filed from PMAT-220's review lanes |
| branch | PMAT-221-planner-names-what-it-did-not-probe |
| base | 4fd70763 |
| commits | 5349a0a2 roadmap · 1102238b RED · 081159a8 core · 9957cb04 surfaces · faf5d77f tests+contract · 7cd37067 contract row · this receipt |
| discover.json sha256 | 3bbbd6ac8efcb062fe35047255a2f4c17526925a5ef7166dcfee0f91c37edd5b |
| gate_cmd | `cargo test --workspace` — `gate_cmd_fallback=true`, said in the first status block |
| model gate | `model=fable class=fable decision=admit basis=file`; tier 1 meets tier 1 |
| status-line join | `[U]` — statusLine `session_id` = hook `session_id`, `tasks[].id` = hook `agent_id`, `transcript_path` on subagentStatusLine stdin: not measured this run (the command that measures it is `subagent-statusline.sh --probe`, which was not exercised); `goal.sh` declarations were made before every dispatch and every Agent description began with `PMAT-221/ph<i>` |
| k_measured vs global | 104 at the start of phase 4 (transcript, distinct assistant ids) vs `global=104`; gap 0 |

## The defect and the fix

`determine_present_action` lets an observed probe override a matching config hash and falls through to the hash comparison when there is none, so a MISSING probe and a probe that found nothing stale both plan `NoOp`. The probe is only taken for machines this host answers for, so every converged task on an SSH target — and since forjar#495 on a pepita namespace — has read as `unchanged` however far its declared inputs moved. Measured before the fix: a task converged on a loopback machine, moved to a TEST-NET address, its input rewritten, prints `Plan: 0 to add, 0 to change, 0 to destroy, 1 unchanged.` and nothing else.

The action is deliberately unchanged. The plan now carries `unprobed`, a census of what it did not measure, computed once by the planner as a pure post-pass after propagation (NoOp only), and every shipped surface renders it the way forjar#342 and #372 render the other two blind spots: a TOTAL list beside a PARTIAL prose disclosure folded into the one `disclosure` field. The predicate the probe skips on has ONE definition (`probe_covers`), asked by the probe, the executor's pre-plan probe and the census, and pinned by a test that counts the wrapping site.

| surface | carries the list | folds the prose |
|---|---|---|
| TTY `print_plan` | — | yes |
| `plan --json` | yes, always | yes |
| MCP / HTTP / verb `PlanOutput` | yes, always | yes |
| `apply --dry-run` text and `--json` | yes | yes |
| sealed plan file and its `apply --plan-file` preview | yes, always; read back, a bad shape is `PLAN_MALFORMED` | yes |

`ExecutionPlan.unprobed` is not serialised when empty, so the seal's diff leg is byte-identical for a plan with nothing to disclose and every previously sealed plan file still verifies; a non-empty census round-trips.

## What review changed

The plan was grilled by one teamwork lane before phase 1 (fan-out measured as unknown — a single-lane review, never consensus) and the diff by three quorum lanes after phase 2 (2 PASS / 1 FAIL, every code claim confirmed by all three). Every finding was re-run.

- **The plan-file preview was missing from the plan.** `apply --plan-file` previews the sealed body; a reviewer would have read `=` beside a resource nothing looked at. Added to phase 2 and wired.
- **The struct-literal breakage was real and mechanical.** Twenty-one files, edited by script; `examples/planner_proof_sat_why.rs` was missed by the first sweep and caught by the pre-commit clippy gate — fixed, not bypassed. The lane's alternative, an on-the-fly census at presentation time, was rejected: it is a second measurement that can disagree with the plan and cannot travel with a sealed file.
- **The probe map is keyed by resource id alone**, so a resource on a local and a remote machine carries the local verdict for the remote row. Pre-existing; the census asks the machine predicate before the map, so the remote row is named anyway (`a_local_probe_says_nothing_about_a_remote_machine`). Filed as forjar#499 rather than widened here.
- **One contract row cited the wrong test.** FALSIFY-PQ-009 bundled the plan-file rule and cited the Update-path test; the plan-file case was cited by no row. Narrowed and given its own row; gate G re-run.

## Verification, all my own runs

| command | claimed | mine |
|---|---|---|
| `cargo test --test falsification_planner_names_what_it_did_not_probe` at 1102238b | — | **6 failed** (RED, each at its first disclosure assertion, every precondition passing) |
| the same at faf5d77f | worker receipt missing | **6 passed** |
| `cargo test --test falsification_plan_json_discloses_its_blind_spot` | — | 5 passed |
| `cargo test --lib planner::tests_unprobed` | — | 8 passed |
| `cargo test --lib -- cli::tests_plan_file cli::tests_print_helpers cli::tests_gh_dogfood_p1 mcp planner::tests_unprobed` | — | 135 passed |
| `cargo clippy --all-targets -- -D warnings` | — | exit 0 |
| `rustup run stable cargo fmt --all -- --check` | — | exit 0 |
| `cargo test --workspace` (gate_cmd, detached) | — | **exit 0, 314 binaries, 19,603 passed, 0 failed** |
| `scripts/dogfood/contracts.sh` (gate G) | — | PASS: 40 contracts validate, every citation resolves |
| `pmat analyze vacuous-tests` over src/core/planner, src/cli, src/mcp, tests | — | 0 in touched paths (one pre-existing hit in `progress.rs`) |

Mutations observed RED: the RED commit 1102238b is the diff's own falsifier — six cases, each failing at the assertion it names with every precondition green. Gate F's mutation arm cannot run on this host (PMAT-216); no `cargo mutants` figure is claimed.

## Dispatch ledger

| dispatch | mode | agent | turns | maxTurns hit | resumed | agy |
|---|---|---|---|---|---|---|
| ph1.delegate teamwork width 1 on the plan | delegate (opus) | a9f3c1b4360a5ce85 | 25 tool uses | no | no | conv-c0b0dbb6; children=unknown method=none — single-lane; verdict do-not-implement-as-written, three claims re-checked |
| ph2 worker B: surfaces | subagent:opus | ab457a0f215068c9f | 48 tool uses | **yes (40)** | no — the work was committed (9957cb04, faf5d77f) and verified by rerun; a resume would have bought a receipt, not a fact | — |
| ph3.delegate quorum width 3 on the diff | delegate (opus) | a205a030b44e89802 | 23 tool uses | no | no | conv-3f70e5ed, conv-6193ebcb, conv-aced4c1e; children=3 consensus; 2 PASS / 1 FAIL; an interrupted first run of lane 3 excluded |

Worker receipt: missing ⇒ treated as `partial=true`; every number above is an orchestrator rerun. Routing: phase 1 `route=agy-goal w=1.00 basis=quota.json@44h note=fable-binding effort=1[U]` (overridden to direct: the design was already measured in discovery and a brief would have been the diff); phase 2 `route=agy-goal w=1.00 basis=quota.json@45h effort=1[U]` (overridden to subagent:opus — five files across cli and mcp, not a single-module phase; R-4's one-writer agy lane was not used); phase 3 `route=agy-quorum w=1.00 basis=quota.json@44h effort=1[U]`; phase 4 `route=self w=11.11 basis=quota.json@44h`. Four lane rows were appended to `docs/audits/impl-routing.jsonl` by `route.sh record`; the delegate's lane files carry no ticket/phase stamp, so those fields are null in the rows (a gap in the delegate, named here).

**Slots and denials:** slots=3, running_peak=1, attempted=3, denied=0 (`events-<session>.jsonl` carries no denial). I-3: `PASS transcript-gate: attempted=3 denied=0 running_peak=1 slots=3`. Never more than one Claude subagent ran at an instant; width came from agy (one teamwork lane, three quorum lanes).

## Jidoka

No red gate stopped the line. Two refusals worth recording: the pre-commit clippy gate refused the phase-1 commit over the `examples/` literal (fixed in place); bashrs lint refused the first fixture's `cat in.txt > out.txt` as SC1035 (a file named `in` reads as the keyword) — the fixture uses `src.txt`, and the falsification test's doc says why. Nothing appended to `.pmat/jidoka.jsonl`: neither was a defect in a module.

## Estimates

`K̂=4 basis=first-run[U]` (`estimate.sh` matched no rows under this repo key); `K=120` declared against `docs/audits/impl-estimates.jsonl:L1-L5`. Actual: 61 turns of discovery before the ticket existed, 24 core, 10 surfaces, 5 review, 14 receipt — 114 of 120, above the 0.8K andon line but with the gate PASS throughout. Rows appended.

## Gaps

- `parallel_multi_stack` and `multi_config` serialise `ExecutionPlan` raw: they carry `unprobed` only when non-empty and never the prose. They never carried the forjar#342 disclosure either; the contract's "every shipped plan surface" rows do not list them. Named, not widened.
- forjar#499: the probe map keyed by resource id alone. Disclosed by this ticket through the machine predicate; the keying itself is the follow-up.
- Gate F's mutation arm (PMAT-216) is unmeasured on this host.
- The status-line join table is `[U]` this run.
- Contract `pv` lane: `contracts_dir` is set and the contract corpus was amended in the same PR and passed gate G; no separate `pv` run beyond `contracts.sh`'s own lint is claimed.

IMPL-PMAT-221-RECEIPT-END
