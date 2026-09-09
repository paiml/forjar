# Quorum evidence — PMAT-221 — the lanes

## Plan grill (phase 1) — one teamwork lane, sandboxed, read-only

- Mode: /teamwork-preview through the delegate; agy 1.1.28; 309 s; exit 0; conversation conv-c0b0dbb6.
- Fan-out MEASURED as unknown: no teamwork project directory was created and the lane reported one turn. This was one model answering once, and it is recorded as a single-lane review, never as agreement.
- Verdict: do-not-implement-as-written, on three objections (struct-literal breakage, the plan-file preview surface, the resource-id-keyed probe map). Each was re-checked by the orchestrator against the tree at 1102238b and is recorded as a refuted claim with its correction in the judges digest. The lane confirmed the seal-compatibility argument, the NoOp-only rule and the wording.
- Host tree byte-identical before and after; no clone left.

## Diff review (phase 3) — three quorum lanes, --mode plan --sandbox, read-only, base pinned

- Diff: main (4fd70763) ... faf5d77f, 1967 lines, served as a file; lanes read the branch through git show and a --shared clone under the session scratchpad; no lane ran cargo (target/ is ~213 GB against ~110 GB free), so every behavioural statement in a lane is cited or asserted and every number in the receipt is an orchestrator rerun.
- Lane 1, conv-3f70e5ed: FAIL — claims 1–9 confirmed with file:line, claim 10 refuted on the FALSIFY-PQ-009 citation; states that fixing the citation makes the diff pass.
- Lane 2, conv-6193ebcb: PASS — all ten confirmed; names the same uncited plan-file test as a gap to close; host status and disk unchanged; deleted its clone.
- Lane 3, conv-aced4c1e: PASS — all ten confirmed; flags the same gap; no build residue. A first dispatch of lane 3 was killed by the delegate's own ten-minute tool cap while still running and was moved OUT of the reduction set; the reduced lane 3 is a clean detached re-run.
- Reduction (lane-reduce.sh, width 3, not-before the run's own start): 2 PASS / 1 FAIL, agreed=false, dissent carried verbatim; the whole disagreement is claim 10.
- Adjudication the lanes left to the orchestrator: raw serialisation surfaces (parallel_multi_stack, multi_config) carry the field only when non-empty and never the prose; they never carried the forjar#342 disclosure either, and are named in the receipt's gaps rather than widened here.
- One lane clone was left under the session scratchpad (never in the repository) and was removed; the host tree was byte-identical to the session-start snapshot after every round.

## Orchestrator reruns (the only measured numbers)

- Acceptance: cargo test --test falsification_planner_names_what_it_did_not_probe — RED at 1102238b (6 failed, each at its first disclosure assertion, every precondition passing), GREEN at faf5d77f (6 passed).
- tests/falsification_plan_json_discloses_its_blind_spot: 5 passed (one new verb case).
- cargo test --lib planner::tests_unprobed: 8 passed; the touched lib modules: 135 passed.
- Gate: cargo clippy --all-targets -D warnings exit 0; rustfmt stable --check exit 0; cargo test --workspace exit 0, 314 binaries, 19,603 passed, 0 failed (12 min, detached).
- Gate G: scripts/dogfood/contracts.sh PASS after the citation fix.
