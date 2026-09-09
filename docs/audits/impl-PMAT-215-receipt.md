# Implementation receipt — PMAT-215 — drift SKIPPED the resources the lock believed were broken, so it was blind to exactly the ones whose live state was most worth measuring

verdict: DONE — a `type: task` guard whose lock says `failed` is now evaluated, because its `completion_check` is an assertion and needs no baseline; the file, image and state-query paths still exclude `Failed`, and a case pins that narrowness by failing when the removal is made blanket. Closes forjar#487's second half.

## Identity

| field | value |
|---|---|
| ticket | PMAT-215 (kind: code) |
| issue | forjar#487, second half |
| branch | PMAT-215-drift-evaluates-broken-entries |
| base | 54e36f130a4a12b623576d0332aae44f779dc34e |
| discover.json | `gate_cmd=cargo test --workspace` with `gate_cmd_fallback=true`, `required_check=gate`, `code_search=pmat query`, `contracts_dir=contracts` |
| model gate | `model=opus class=opus decision=admit basis=file` |

## The defect, measured before anything was changed

```
apply     ->  0 converged, 0 unchanged, 1 FAILED
lock      ->  status: failed
the completion_check passes by hand
drift     ->  inspected 0 of 1 resource(s) in scope: none
              skipped 1: not converged in the lock 1
              No drift detected.
```

The operator hit it on two fleet machines at once: `skipped 10: in the lock, not in the config 7, not converged in the lock 3`, where those three were exactly these guards. The refuse-loudly pattern forjar itself makes necessary — `command` exits 1 and names the human step, `completion_check` is the real assertion — stopped being measured the moment an apply recorded it failed.

## The distinction that makes the fix correct, and narrow

`src/tripwire/drift/mod.rs` excludes `Failed`/`Unknown` from the hash-comparison paths, and the reason written there is right: a failed apply's recorded hash is not a baseline anything can be compared against.

That reasoning does not reach a `completion_check`. A check is an **assertion**, not a baseline. It asks the host a question and reads the answer, and needs nothing from the lock to be answerable. A failed apply does not make the question unanswerable; it makes it urgent.

So the change is one early return removed from `skip_reason` in `src/tripwire/drift/task_check.rs`, and nothing else. `file.rs`, `image.rs` and the state-query path are untouched by design, and a test fails if that stops being true.

## Verification: claimed versus my rerun

| command | lane claimed | my rerun |
|---|---|---|
| `cargo test --test falsification_drift_measures_a_broken_guard` | pass | **4 passed** |
| `cargo fmt --all -- --check` | pass | **clean** |
| `cargo clippy --all-targets --all-features -- -D warnings` | pass | **exit 0** |
| `cargo test --lib` | pass, "previous failure was hallucinated" | **13488 passed, 0 failed** |
| `cargo test --workspace` (gate_cmd, fallback) | not claimed | **RED, 1 failure, pre-existing — see below** |

End to end with the real binary, which is what the operator reads:

| step | before | after |
|---|---|---|
| drift, assertion false | `skipped 1: not converged in the lock`, `No drift detected.` | `DRIFTED: runner-registered on box (completion_check fails on box: task=pending)` |
| `drift --tripwire`, assertion false | exit 0 | **exit 1** |
| drift, after the operator runs the make target | skipped | `inspected 1 of 1`, `No drift detected.` |
| `drift --tripwire`, after | exit 0 | exit 0 |

## Routing and dispatch

| phase | class | route | executor | outcome |
|---|---|---|---|---|
| 1 RED test | orchestration | `route=self w=0.00 basis=quota.json@36h` | me | 3 of 3 failing, committed `7b1d6f50` |
| 2 fix | impl | `route=agy-goal w=1.08 basis=quota.json@36h` | `paiml-agy-delegate`, 1 lane, `--mode goal --writes` | committed `5efc88d6`, `partial=true` |
| 3 gate | orchestration | `route=self w=0.00` | me | see the table above |
| 4 review | review | `route=agy-quorum w=1.08 basis=quota.json@36h` | `paiml-agy-delegate`, 3 lanes, `--mode plan`, sandboxed | 1 PASS, 2 FAIL, `partial=true` |

Slots: peak 3 of 3, `attempted=11 denied=2 running_peak=3` from `transcript-gate.sh`, which PASSES. Two hook denials this session, neither retried.

The goal lane returned `status=ERROR "timeout waiting for response"` with `num_turns=1` while still carrying a complete `structured_output` and having already committed, and `lane-reduce.sh` scored it `NO-VERDICT` because `goal-schema.json` names its verdict field `outcome`. Both are recorded as `partial=true` by the delegate. Neither is evidence the work happened or did not; the reruns above are.

## What the review changed

Three lanes, 1 PASS and 2 FAIL, and both substantive findings reproduced.

- **The census case was too weak.** Lane 2 refused `what_cannot_be_evaluated_is_still_not_counted_as_clean` because `--no-task-checks` is a *declined* measurement, not an impossible one, and the acceptance criterion is about the impossible one. It is right. A fourth case now uses a task with **no** `completion_check` whose lock says `failed`: nothing can evaluate it, and it must still be named as unmeasured rather than counted clean. Mutating `mod.rs` toward a blanket removal fails that case and leaves the other three green, so it measures the narrowness rather than the fix.
- **CLAUDE.md was in the diff.** Lane 3 refused the branch for a `paiml-implement:orch-routing` block that was already staged in the working tree before this branch existed and rode along into the first commit. Reverted here; whatever wrote it can land it on its own.

Lane 2's substantive finding survived only in its `.response`: a second, truncated emission overwrote its `structured_output` with an empty `findings` list, so `lane-reduce.sh` could not see the one real dissent in the round. Worth knowing before trusting a reduced verdict.

## Jidoka

`cargo test --workspace` is RED, and it is not this branch. `forjar-contracts` `coverage_map_enrichment` asserts a non-empty result set for the query `softmax`, an aprender kernel, against forjar's IaC corpus. It early-returns unless a sibling `aprender` is visible from `CARGO_MANIFEST_DIR/../..`, so it passes vacuously in CI and in any worktree outside `~/src`, and runs for real on a fleet workstation.

That cost a wrong conclusion before it yielded a right one: the first comparison ran the branch in `~/src/forjar` and main in a worktree under `/tmp`, so the branch "failed" and main "passed" and neither result meant anything. Re-running `origin/main` at 54e36f13 from `~/src/forjar-mainck` reproduced the failure with none of my changes present.

Filed on forjar#452 with the fix named: its immediate sibling `violations_enrichment` already carries the right `cfg_attr(not(feature = "aprender-corpus"), ignore = ...)`. Recorded in `.pmat/jidoka.jsonl`. Non-blocking for this ticket.

## Estimates

| field | value |
|---|---|
| K̂ | 4, `basis=first-run[U]` (`estimate.sh` ROWS=0) |
| K | 40 |
| actual | 18 orchestrator turns |

K̂ was wrong by a factor of four and the basis says why: no prior rows for this repo. The cost was not the fix, which is nine lines; it was the two review findings and the invalid comparison in the jidoka above.

## Gaps

- `cargo test --workspace` is red on a corpus-visible workstation until forjar#452 lands. CI is green because the test early-returns there, which is the part worth fixing.
- Gate F's mutation arm is still unmeasured on this host (PMAT-216), so no `cargo mutants` number is claimed here. The mutations reported above were run by hand and are named individually.
- The file, image and state-query paths still skip `Failed`. That is deliberate and pinned, not deferred. If a baseline-free observable is ever added for them, this decision is the one to revisit.

IMPL-PMAT-215-RECEIPT-END
