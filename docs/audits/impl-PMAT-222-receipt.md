# Implementation receipt — PMAT-222 — a build-I/O probe answers only for the tree it was taken on

verdict: DONE — the probe map is keyed by (machine, resource id); a digest answers for the machine it was taken on and no other, the remote row of a multi-machine task keeps config-hash planning and is named in the census, and the action on the local row is what the probe says. Closes forjar#499; the write-path defect the review located is filed as forjar#501.

## Identity

| field | value |
|---|---|
| ticket | PMAT-222 (kind: code, `orch:fable`, `orch-basis:M>=3`) |
| issue | forjar#499, filed from PMAT-221's review lanes |
| branch | PMAT-222-probe-keyed-by-machine |
| base | bc2819cc (the squash of PR #500) |
| commits | 68853425 roadmap · b6301da1 the fix · 3d8a1606 the contract · 00343bd7 the re-export line · the receipt and evidence · the quorum receipt |
| discover.json sha256 | ada42231b3ebcd974adddd91541f2804ba43c95e2adfc63f84eca2f9296cbca1 |
| gate_cmd | `cargo test --workspace` — `gate_cmd_fallback=true`, said in the first status block |
| model gate | `model=fable class=fable decision=admit basis=file`; tier 1 meets tier 1 |
| session | the SECOND ticket in Claude session a79c3cdb (PMAT-221 was the first). R-5 says one ticket per session; its instrument, `goals-<sid>.jsonl`, was wiped by the host reboot mid-day, so `goal.sh set` accepted the ticket. I named the rule and stopped; Noah's next message was, verbatim, `continue autonomously using pmat-implement`, and the ticket proceeded on that. `k_measured_at_set` (186) was recorded by `goal.sh` so this ticket's `k` is measured from there |
| status-line join | `[U]` — statusLine `session_id` = hook `session_id`, `tasks[].id` = hook `agent_id`, `transcript_path` on subagentStatusLine stdin: not measured this run; every dispatch was declared with `goal.sh worker` first and every Agent description began with `PMAT-222/ph<i>` |
| k_measured vs global | `k_measured_at_set=186`; at the receipt commit see the estimates table; `global` = the difference; gap 0 by construction (both from the transcript, distinct assistant ids) |

orch_model: fable [A]   orch_class: code   orch_decision: admit   orch_basis: M>=3
fable_binding: true   quota_age_h: absent   quota_mark: ?   k_measured_at_set: 186

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent  (run at receipt time)
  ph1  class=cross  route=agy-goal  w=1.00  basis=quota.json@49h  effort=1[U]  (overridden to direct: agy write lanes barred by the recorded hazard)
  ph1.grill  class=plan  route=agy-plan  w=1.00  basis=quota.json@49h  effort=1[U]  (delegate teamwork, read-only)
  ph2  class=mechanical  route=agy-goal  w=1.00  basis=absent  note=fable-binding  effort=1[U]  (run at receipt time; overridden to direct, one file)
  ph3  class=review  route=agy-quorum  w=1.00  basis=quota.json@50h  effort=1[U]  (delegate quorum width 3)
  ph4  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="cargo test --test falsification_probe_answers_for_the_tree_it_was_taken_on (pre-fix tree)"  claimed_exit=-  rerun_exit=101  log_path=docs/audits/logs/PMAT-222-red-binary.log  sha256=bccfd029d1297421
  cmd="cargo test --lib -- planner::tests_unprobed (pre-fix tree)"  claimed_exit=-  rerun_exit=101  log_path=docs/audits/logs/PMAT-222-red-lib.log  sha256=fbde561406954d79
  cmd="cargo test --test falsification_probe_answers_for_the_tree_it_was_taken_on; the two forjar#497 suites (fixed tree)"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-222-green-binary-and-497.log  sha256=5d5d435f13f7317b
  cmd="cargo test --lib -- planner task::tests_probe tests_api; cargo clippy --all-targets -- -D warnings"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-222-green-lib-clippy.log  sha256=10e17df5ee76f912
  cmd="cargo test --lib -- api::tests; cargo clippy --all-targets -- -D warnings (after the re-export line)"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-222-green-api-tests-clippy.log  sha256=0e767c9f78e39f9d
  cmd="bash scripts/dogfood/contracts.sh"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-222-gate-G.log  sha256=543a0b7d2964db22
  cmd="cargo test --workspace at 3d8a1606"  claimed_exit=0(lanes, by reading)  rerun_exit=101  log_path=docs/audits/logs/PMAT-222-workspace-red.log  sha256=73a2a6f4b8a37daa
  cmd="cargo test --workspace at 00343bd7"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-222-workspace-green.log  sha256=df39063f848c72f8

Each log under docs/audits/logs/ is the reduced form (head, verdict lines, tail) of the full log, whose byte count and sha256 are on its first line.

## The defect and the fix

`probe_all` inserted one `IoDigest` per RESOURCE ID, taken on the controller, for any resource with at least one machine this host answers for; `determine_present_action` read it by resource id for every (resource, machine) row it planned. A task on `[box, far]` carried the controller's probe for the far row: local inputs stale, far planned `Update` from a hash of the wrong tree; local inputs fresh, far planned `NoOp` with its tree unmeasured — disclosed since forjar#497 only because the census asked the machine predicate BEFORE a map that could not answer per machine. Measured before the fix: the RED test's stale case planned far as `Update` (`left: Update, right: NoOp`).

The map is now `ProbeMap`, one map per machine (so a lookup by two `&str` allocates nothing), and `probe_all` probes a resource once and records the digest under every machine `is_local` admits and under no other. `determine_present_action` asks for `(machine_name, resource_id)`. The census decides from the map alone and uses `probe_covers` — still one definition — only to word the reason. `api::probe_all` returns `api::ProbeMap`; the supported surface is 12 items, said in the changelog as the ceiling test demands.

After the fix on `[box, far]` with far moved to a TEST-NET address and the local input rewritten: `Plan: 0 to add, 1 to change, 0 to destroy, 1 unchanged`, box `update`, far `no_op`, the census exactly `[(build, far)]`, the TTY disclosure names `build@far` and not `build@box`. With the input left alone both rows are `no_op` and far is still named — the forjar#497 disclosure does not regress.

## What review changed

The plan was grilled by one teamwork lane before phase 1 (7 child conversations measured) and the diff by three quorum lanes after phase 2 (3/3 PASS, every claim confirmed, no dissent). Every finding was re-run.

- **A tuple key would allocate on every lookup.** `(String, String)` has no `Borrow<(&str, &str)>`; the map is one map per machine instead, and the struct's doc says why.
- **The plan's call-site inventory was short.** Ten planner test sites, `api.rs`, `tests_api.rs` and `core/task/mod.rs` were named by the lane and are in the fix commit; the pre-commit clippy gate would have caught the compile error, the lane caught it first.
- **The issue's premise about the write path was wrong.** `record_io_hashes` hashes the controller's tree into every machine's lock, and `check_task_input_cache` reads it back for a remote `cache: true` task (with a base-directory mismatch beside it). Filed as forjar#501 with both sites named; after this diff no probe is compared against a remote row, so the wrong number is inert for `plan` and live only for the cache.
- **The surface ceiling was confirmed by reading and refuted by the gate.** All three diff lanes confirmed C7 (`the_supported_surface_stays_small` at 12); the workspace gate read 8 — rustfmt had wrapped the five-item re-export over three lines and the test counts items per `pub use crate::` line. Two single-line re-exports; the five whys are below.
- **Lane 3's C9 case is the intended trade.** A machine alias this host answers for that `probe_covers` misses now keeps config-hash planning and is NAMED, where before it planned `Update` through the id key from a probe it never had. The map answers for what was measured; the predicate's coverage is forjar#485/#495's concern, unchanged here.

## Verification, all my own runs

| command | claimed | mine |
|---|---|---|
| `cargo test --test falsification_probe_answers_for_the_tree_it_was_taken_on` on the pre-fix tree | — | **1 failed** (the stale case, at the far-row assertion, `left: Update, right: NoOp`; every precondition green), 1 passed (the fresh case is the forjar#497 regression guard and was expected green) |
| `cargo test --lib -- planner::tests_unprobed` on the pre-fix tree | — | **1 failed** (`a_stale_local_probe_does_not_plan_the_remote_row`, same assertion), 8 passed |
| the binary test after the fix | — | 2 passed |
| `cargo test --lib -- planner task::tests_probe tests_api` | lanes: confirmed by reading | 263 passed (the `tests_api` filter matched nothing — see jidoka) |
| `cargo test --lib -- api::tests` after the re-export fix | — | 7 passed |
| `cargo test --test falsification_planner_names_what_it_did_not_probe --test falsification_plan_json_discloses_its_blind_spot` | — | 6 passed, 5 passed |
| `cargo clippy --all-targets -- -D warnings` | — | exit 0 (twice: after the fix, after the re-export line) |
| `rustup run stable cargo fmt --all -- --check` | — | exit 0 |
| `scripts/dogfood/contracts.sh` (gate G) | — | PASS: 40 contracts validate, citations resolve |
| `pmat analyze vacuous-tests` over src/core/planner, src/core/task, tests | — | 0 in this diff (one pre-existing hit, `tests_ambient.rs:59`) |
| `cargo test --workspace` (gate_cmd, detached) at 3d8a1606 | — | **exit 101**: 13,510 passed, 1 failed (`api::tests::the_supported_surface_stays_small`, 8 ≠ 12) |
| `cargo test --workspace` (gate_cmd, detached) at 00343bd7 | — | **exit 0, 315 binaries, 19,608 passed, 0 failed** |
| lane 2's three rerun commands (id-keyed lookups in src/, the transport-predicate count, `wc -l`) | lanes: confirmed | only the struct field matches; 1 / 0 / 0; 488 / 352 / 91 — all agree |

Mutations observed RED: the RED tests are the diff's own falsifier — both fail on the pre-fix tree at the assertion that names the remote row, with every precondition green, and pass after. Gate F's mutation arm cannot run on this host (PMAT-216); no `cargo mutants` figure is claimed.

## Dispatch ledger

| dispatch | mode | agent | turns | maxTurns hit | resumed | agy |
|---|---|---|---|---|---|---|
| ph1.delegate teamwork width 1 on the plan | delegate (opus) | a2d0025496a83ec4e | 34 tool uses | **yes (30)** — the lane had finished and its critique, `lane-1.json` and `lane-reduce.json` were on disk; read directly, not resumed | no | conv-feca63c3; children=7 method=agents-dir; verdict implement-with-changes, three plan claims refuted and corrected |
| ph3.delegate quorum width 3 on the diff | delegate (opus) | afd873389338503cd | 28 tool uses | no | no | conv-5da6b238, conv-fedfc0ec, conv-c8638070; children=3 consensus; 3/3 PASS, no dissent; mode composed as `--mode plan` by the delegate (brief named none) |

Routing (`route.sh`, copied verbatim): phase 0 `route=self w=100.00 basis=absent` (class orchestration, run at receipt time — not run before discovery); phase 1 `route=agy-goal w=1.00 basis=quota.json@49h effort=1[U]` — **overridden to direct**: agy `writes=true` lanes are barred in this repository by the recorded hazard (a sandboxed lane mutated the tree and copied `target/` to `/tmp` until the disk filled), and the design was fully measured in discovery; the plan grill `route=agy-plan w=1.00 basis=quota.json@49h effort=1[U]` went to the delegate as a read-only teamwork lane; phase 2 (the contract) `route=agy-goal w=1.00 basis=absent note=fable-binding effort=1[U]` for class mechanical (run at receipt time, by which point `quota.json` had aged out — `basis=absent` is what it printed), overridden to direct for the same reason and because the edit was one file; phase 3 `route=agy-quorum w=1.00 basis=quota.json@50h effort=1[U]`; phase 4 `route=self w=100.00 basis=absent`. The delegate appended lane rows to `docs/audits/impl-routing.jsonl` with null ticket/phase (the delegate's lane files carry no stamp — the same gap PMAT-221 named).

**Slots and denials:** slots=3, running_peak=1, attempted=5 across the session (2 this ticket), denied=0. I-3: `PASS transcript-gate: attempted=5 denied=0 running_peak=1 slots=3`. Never more than one Claude subagent ran at an instant; width came from agy (7 teamwork children, 3 quorum lanes).

## Jidoka

One red gate stopped the line: `cargo test --workspace` at 3d8a1606, `api::tests::the_supported_surface_stays_small` read 8 against 12. Five whys, in `.pmat/jidoka.jsonl`: rustfmt wrapped the five-item probe re-export over three lines; the ceiling test counts items only on lines that start with `pub use crate::`, so a wrapped brace list counts as one; my targeted lib run filtered on `tests_api`, which does not match the mounted path `api::tests`, so the test never ran before the gate; three review lanes confirmed the ceiling by reading the literal, and the brief forbade them cargo, so a read stood in for a run; the counter is line-based by design (Rust cannot enumerate re-exports at runtime), so the fix is a single-line re-export with a comment naming why, and the fragility is named here, not widened. Re-measured: `api::tests` 7 passed, clippy clean, the workspace suite green.

Not a jidoka row: the phase-1 delegate hit its turn cap after the lane had finished (a delegate gap; the lane's artefacts were complete and were read directly).

## Estimates

`K̂=4 basis=first-run[U]` (`estimate.sh` matched no rows under this repo key, ROWS=0, as on PMAT-221); `K=80` declared against `docs/audits/impl-estimates.jsonl:L6-L10` (PMAT-221's five rows, 114 turns in all, of which 61 were discovery before a ticket existed). Actual, measured from the transcript at each commit's timestamp (distinct top-level assistant ids): the #500 merge at k=159, the roadmap commit at k=180, `goal.sh set` at k=186, the fix at k=200, the contract at k=202, the re-export line at k=206. Rows appended.

## Gaps

- forjar#501: the write path — `record_io_hashes` on the controller for every machine's lock, `check_task_input_cache` reading it for a remote `cache: true` task, and the `state_dir.parent()` vs `working_dir` base mismatch between them. Named by the plan grill; filed, not widened.
- `the_supported_surface_stays_small` counts re-exports per line, so a rustfmt wrap changes the count. Named; the re-export carries a comment; the counter is not changed here.
- The changelog paragraph creates a gate H obligation (a CRUX row in `docs/audits/crux-<next version>.md`) that falls due when `Cargo.toml` moves past `v1.27.0`; the gate is PENDING until then by its own rule.
- Gate F's mutation arm (PMAT-216) is unmeasured on this host.
- The status-line join table is `[U]` this run.
- Contract `pv` lane: `contracts_dir` is set and the corpus was amended in the same PR and passed gate G; no separate `pv` run beyond `contracts.sh`'s own lint is claimed.
- Second ticket in one session (see Identity). The rule's instrument was lost to a reboot, not cleared; the receipt says so rather than the session pretending to be fresh.

IMPL-PMAT-222-RECEIPT-END
