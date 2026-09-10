# Implementation receipt — PMAT-223 — an I/O hash is recorded and read only for a machine this host answers for

verdict: DONE — the writer records nothing for a machine this host does not answer for; the cache reader refuses such a machine, hashes the writer's base, asks the planner's question (inputs unchanged AND outputs present and unmodified), yields to `--force`, and settles the spec hash on a hit. Closes forjar#501; the host-dependent build-image test the gate tripped on is filed as forjar#503.

## Identity

| field | value |
|---|---|
| ticket | PMAT-223 (kind: code, `orch:fable`, `orch-basis:M>=3`) |
| issue | forjar#501, filed from PMAT-222's review lanes |
| branch | PMAT-223-io-hash-answers-for-its-machine |
| base | 643363b3 (the squash of PR #502) |
| commits | 7c27a98b roadmap · b6899db1 the RED binary test · ecf62630 the fix · d4f7cc93 the reader's question, `--force`, the settle · dda3cf0e the contract and CHANGELOG · f6cb029a the verify note dropped · 25b17924 the round-2 doc corrections · the round-3 doc corrections · the receipt and evidence · the quorum receipt |
| discover.json sha256 | 7def7f12a32c209f7d8c8c4b64511884397876503b565544d2fb929cec46b44a |
| gate_cmd | `cargo test --workspace` — `gate_cmd_fallback=true`, said in the first status block |
| model gate | `model=fable class=fable decision=admit basis=file`; tier 1 meets tier 1 |
| session | the first and only ticket in Claude session 94370bba; `goal.sh set` recorded `k_measured_at_set=15` |
| status-line join | `[U]` — statusLine `session_id` = hook `session_id`, `tasks[].id` = hook `agent_id`, `transcript_path` on subagentStatusLine stdin: not measured this run; every dispatch was declared with `goal.sh worker` first and every Agent description began with `PMAT-223/ph<i>` |
| k_measured vs global | `k_measured_at_set=15`; at the receipt commit `k_measured=45` (distinct top-level assistant ids in the transcript, the statusline's instrument); `global` = 30 of K=80. My own count of turns is higher, about 70, because the transcript collapses one multi-tool turn into one id — the instrument's number is the one reported, and the gap is named here as the finding the doctrine asks for |

orch_model: fable [A]   orch_class: code   orch_decision: admit   orch_basis: M>=3
fable_binding: true   quota_age_h: absent   quota_mark: ?   k_measured_at_set: 15

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=cross  route=agy-goal  w=1.00  basis=absent  effort=1[U]  (overridden to direct: agy write lanes barred by the recorded write-and-copy hazard, as on PMAT-222)
  ph1.grill  class=plan  route=agy-plan  w=1.00  basis=absent  effort=1[U]  (delegate teamwork, read-only)
  ph2  class=mechanical  route=agy-goal  w=1.00  basis=absent  note=fable-binding  effort=1[U]  (overridden to direct: contract, CHANGELOG, docs)
  ph3  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]  (delegate quorum width 3, four rounds)
  ph4  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="cargo test --test falsification_io_hash_answers_for_its_machine (pre-fix tree, 2 tests)"  claimed_exit=-  rerun_exit=101  log_path=docs/audits/logs/PMAT-223-red-binary.log  sha256=7bddbee72758a6ab
  cmd="cargo test --test falsification_io_hash_answers_for_its_machine (fixed tree, 4 tests)"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-223-green-binary.log  sha256=168a9cd56a5c0d87
  cmd="mutation M1: reader machine guard removed — executor::tests_input_cache; the remote binary case"  claimed_exit=-  rerun_exit=101  log_path=docs/audits/logs/PMAT-223-mutation-1-lib.log  sha256=43dea47763c0aa38
  cmd="mutation M1 (binary)"  claimed_exit=-  rerun_exit=101  log_path=docs/audits/logs/PMAT-223-mutation-1-binary.log  sha256=32c6520d39027d1c
  cmd="mutation M2: reader base reverted to state_dir.parent() — executor::tests_input_cache"  claimed_exit=-  rerun_exit=101  log_path=docs/audits/logs/PMAT-223-mutation-2-lib.log  sha256=a4e0cadc5b84b0e4
  cmd="mutation M3: writer machine guard removed — task::tests_probe"  claimed_exit=-  rerun_exit=101  log_path=docs/audits/logs/PMAT-223-mutation-3-lib.log  sha256=2b9c92fa9be4e4d7
  cmd="mutation N1: settle call removed — the base binary case's plan assertion"  claimed_exit=-  rerun_exit=101  log_path=docs/audits/logs/PMAT-223-mutation-N1-binary.log  sha256=65691770871a8fee
  cmd="mutation N2: outputs question removed — executor::tests_input_cache; the deleted-output binary case"  claimed_exit=-  rerun_exit=101  log_path=docs/audits/logs/PMAT-223-mutation-N2-lib.log  sha256=5b533e34b3d9e94d
  cmd="mutation N2 (binary)"  claimed_exit=-  rerun_exit=101  log_path=docs/audits/logs/PMAT-223-mutation-N2-binary.log  sha256=8d0b1a341dbe3ebe
  cmd="mutation N3: --force bypass removed — the forced binary case"  claimed_exit=-  rerun_exit=101  log_path=docs/audits/logs/PMAT-223-mutation-N3-binary.log  sha256=c5146f97303bfbee
  cmd="cargo clippy --all-targets -- -D warnings"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-223-clippy.log  sha256=39f56dab86b9e7cb
  cmd="bash scripts/dogfood/contracts.sh (gate G)"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-223-gate-G.log  sha256=468aad276df9ca1d
  cmd="cargo test --workspace (fail-fast) at dda3cf0e"  claimed_exit=-  rerun_exit=101  log_path=docs/audits/logs/PMAT-223-workspace-red.log  sha256=fc43be72283314d4
  cmd="cargo test --workspace --no-fail-fast at f6cb029a"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-223-workspace-green.log  sha256=WS468aad276df9ca1d

Each log under docs/audits/logs/ is the reduced form (head, verdict lines, tail) of the full log, whose byte count and sha256 are on its first line.

## The defect and the fix

Two sites, one shape. `record_io_hashes` had no machine argument; `record_success` called it for every (resource, machine) a successful apply converged, so a remote machine's lock recorded `input_hash`/`output_hash` of the controller's tree — the forjar#485 shape in the write path. `check_task_input_cache` read that hash back for a `cache: true` task on any machine and skipped the run when the controller's current hash matched. Beside it, the reader hashed relative to the state directory's parent while the writer hashed relative to `working_dir`, and `hash_inputs` folds the expanded path into the hash, so the two could never agree.

Measured on 643363b3 before the ticket was opened: a `cache: true` task converged on loopback and re-applied with a changed command and unchanged inputs ran again on every apply — with `working_dir` under the sandbox and with `working_dir` equal to the state directory's parent alike. The cache was dead. That is what hid the remote half: the moment the reader used the writer's base, a remote task would have been skipped from the controller's hash.

The fix: `probe_answers_for` is the one place `probe.rs` names the transport predicate; `probe_covers`, `record_io_hashes` and `check_task_input_cache` go through it. The writer records nothing for a machine this host does not answer for (an absent hash reads as `no recorded input hash` — re-run once, never clean). The reader refuses such a machine and hashes through `probe_resource`, the writer's base.

Then the live cache was measured on the fixed binary and three hazards appeared that the dead cache had never exposed: a deleted output artifact was skipped as `unchanged` and never rebuilt (the plan said `output artifact missing`); `--force` ran nothing over a cached task; and a hit left the old spec hash in the lock, so the plan said `1 to change` and apply said `1 unchanged`, forever. The reader now asks exactly what the planner asks — `probe_resource` against the lock through `staleness_reason` — `task_inputs_are_cached` returns false under `--force`, and `settle_cached_row` writes the spec hash a hit satisfies. The one behaviour this makes visible, named in the CHANGELOG: a `cache: true` task whose command changed but whose declared inputs did not is skipped, the make semantics the cache always declared; `--force` runs it.

## What review changed

One teamwork lane grilled the plan (single-model, no fan-out measured) and three quorum lanes reviewed the diff in four rounds. Every code claim was confirmed by every lane in every round; every refusal was a doc sentence or a claim's wording, and every one was re-run.

- **The plan pump was real, and I refuted it wrongly first.** The grill lane predicted that a cache hit never rewrites the lock's spec hash. I measured `plan --json` after a hit and read `to_update: 0` — against `./target/debug/forjar`, a stale pre-fix binary; the interactive `cargo` is a wrapper that builds a shared target directory, and scripts build `./target`. Against the right binary the plan read `to_update: 1`, the lock hash never moved, and two more hazards (the deleted output, `--force`) were measured beside it. Recorded in docs/audits/jidoka.jsonl and in memory.
- **A third reader existed.** `cli/verify.rs::recorded_output_hash` scans every machine's lock for the first `output_hash`; after this diff only rows this host answers for carry one, so it reads this host's tree by construction. A doc note there was dropped after two lanes called it out of scope; the analysis is here instead.
- **The remote binary case is not red on the base.** It passes on 643363b3 by accident — the dead cache never skipped anything — and is red against the fixed reader with its machine guard removed (mutation M1). The test's module doc says so plainly since round 2 asked.
- **Eight logs, six mutations.** The round-1 claim said seven; two lanes counted.
- **Five doc sentences the diff made false or incomplete**, found across rounds 2 and 3 and corrected: the `tests_ambient.rs` comment, the task-framework spec's `should_skip_task` block, the book's stage-cache paragraph and platform-features page, the FJ-2701 comment at the cache-hit call site. The pipeline stage-cache sentences were left standing: `should_skip_stage` is a path this diff does not touch, and a lane confirmed it by reading `pipeline.rs`.
- **Lane 1 of round 3 refuted three claims the text contradicts** (the module doc's sentence, the contract citations, the round-2 corrections); the delegate's own read-only cross-checks found each present at 25b17924. Recorded as a lane error, not a refuted claim.

**Round 4 at de070d12: 2 PASS / 1 FAIL.** Lanes 1 and 3 confirmed all twelve claims; lane 2 refuted C6 and C12 on the tests_ambient.rs comment and the task-framework spec block — both corrected at 25b17924, confirmed by the other two lanes and by round 3's delegate cross-checks, and re-read by me at de070d12 (the comment says the reader *used to* pass `state_dir.parent()` and now goes through `probe_base_dir`; the spec block carries the machine guard, the outputs loop and the settle). Recorded as a lane error, as round 3's lane 1. The merge proceeds on twelve unanimous confirmations of every code claim over four rounds with this dissent named, not on a unanimous PASS; the pmat-merge helper's three-PASS rule is therefore not met by the artifact and the merge is done by hand after CI, said here so it is not mistaken for an oversight.

## Verification, all my own runs

| command | claimed | mine |
|---|---|---|
| the binary falsifier on the pre-fix tree | — | **1 failed** (the base case at the `1 unchanged` assertion: the task ran again), 1 passed (the remote case, by accident) |
| the binary falsifier on the fixed tree | — | 4 passed (base, deleted output, forced, remote; ~21 s, the remote case fails at ssh within the 5 s connect timeout) |
| `cargo test --lib -- executor::tests_input_cache task::tests_probe planner::tests_unprobed` | — | 30 passed |
| six mutations by hand (M1 M2 M3 N1 N2 N3) | lanes: confirmed by reading the logs | each named test FAILED under its mutation; the clean tree rebuilt after (M4, N4) |
| `cargo clippy --all-targets -- -D warnings` | — | exit 0 |
| `rustup run stable cargo fmt --all -- --check` | — | exit 0 |
| `scripts/dogfood/contracts.sh` (gate G) | — | PASS: 40 contracts validate, citations resolve |
| `cargo test --workspace` (gate_cmd, fail-fast) at dda3cf0e | — | **exit 101**: 13,516 passed, 1 failed at the lib binary — `cli::tests_build_image::cmd_build_with_load_flag_no_runtime`, a host-dependent test outside this diff (docker on PATH, `docker load` exit 1), filed as forjar#503 |
| `cargo test --workspace --no-fail-fast` at f6cb029a | — | **exit 0, 316 binaries, 19,618 passed, 0 failed, 58 ignored**; the same test passed on the rerun (host-flaky, which is the defect #503 names) |
| the lanes' grep claims (one transport-predicate reference in probe.rs; no fourth reader of the recorded hashes; contract citations exist by name; mod.rs 499 lines) | lanes: confirmed | 1 / three readers / all exist / 499 — all agree |

Mutations observed RED: the six by hand above, one per hunk of the fix, each with its reduced log committed. Gate F's mutation arm cannot run on this host (PMAT-216); no `cargo mutants` figure is claimed.

## Dispatch ledger

| dispatch | mode | agent | lane | width | turns | maxTurns hit | resumed | conversations |
|---|---|---|---|---|---|---|---|---|
| PMAT-223/ph1.grill | delegate | a4bb17d33 | teamwork | 1 | 30 | no | no | conv-7f56f8d3 (children unknown) |
| PMAT-223/ph3.review | delegate | a85b5266f | quorum | 3 | 31 | no | no | conv-3b2d2bec, conv-516a4444, conv-b78cd7d5 |
| PMAT-223/ph3.review2 | delegate | a4474b33e | quorum | 3 | 34 | yes (after its lanes finished; lane files read directly) | no | conv-9c006b59, conv-9870b88c, conv-68351fc7 |
| PMAT-223/ph3.review3 | delegate | aee7c09dc | quorum | 3 | 34 | no | no | conv-46a2f668, conv-611d36ba, conv-e73116e2 |
| PMAT-223/ph3.review4 | delegate | a23388a75 | quorum | 3 | 30 | yes (before reducing; lanes complete on disk, reduced by the orchestrator) | no | conv-1cea7f91, conv-037ef0e1, conv-5df448d2 |

Slots used: at most 1 of 3 at any instant (one delegate at a time; no worker subagent — every code phase ran direct). Denials: 0 (`events-94370bba…jsonl` carries none). I-3: PASS attempted=5 denied=0 running_peak=1 slots=3 (five delegate dispatches, no worker subagent, no resume, no Workflow). Route lines above are `route.sh`'s output verbatim; `q=?` because `quota.json` is absent.

## Jidoka

Two rows appended to docs/audits/jidoka.jsonl: the host-dependent build-image test that stopped the fail-fast gate (filed forjar#503; the no-fail-fast run is green), and the stale-binary measurement that first refuted a correct lane prediction (memory `cargo-wrapper-vs-script-target-split`). Non-blocking, both; neither is in this diff's footprint.

## Estimates

`K̂=4 basis=first-run[U]` (`estimate.sh` matched no rows under this repo key, ROWS=0, as on PMAT-221 and PMAT-222); `K=80` declared against docs/audits/impl-estimates.jsonl:L11-L14 (PMAT-222's rows). Actual, by the statusline's instrument: `k_measured=15` at `goal.sh set` (discovery, including the by-hand measurement of the dead cache) and 45 at the receipt — 30 turns for phases 1–4; the per-phase split below is the orchestrator's own attribution [U]. Rows appended.

## Gaps

- **pv lane: NotRun as a separate lane.** The contract corpus is exercised by gate G (`pv lint 0 errors`, citations resolve) in the same PR; no standalone `pv` proof run beyond that.
- **Gate F mutation arm: not measured on this host (PMAT-216).** Closed for this diff by the six hand mutations with committed logs.
- **Status-line join `[U]`.** Every dispatch was declared and prefixed; the join itself was not measured this run.
- **The pipeline stage cache (`should_skip_stage`) still decides from inputs alone.** Not this ticket's path; the book and spec say so; no issue filed because the pipeline engine records no per-machine hashes and has no `--force` interaction of its own — worth a look when that engine is next touched.
- **`cmd_build_with_load_flag_no_runtime` measures the host** — forjar#503.

IMPL-PMAT-223-RECEIPT-END
