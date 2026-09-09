# Quorum evidence — PMAT-222 — the lanes

## Plan grill (phase 1) — one teamwork lane, sandboxed, read-only

- Mode: `/teamwork-preview` through the delegate; agy 1.1.28; 1288 s; exit 0; conversation conv-feca63c3; fan-out measured as 7 child conversations (agents-dir method).
- Verdict: implement-with-changes on eight findings. Three were refutations of the plan as written and are recorded as refuted claims: the tuple key (P1 → one map per machine, so a lookup by two `&str` allocates nothing), the call-site inventory (P2 → ten more test call sites, `api.rs`, `tests_api.rs`, `core/task/mod.rs`), and the write path (P3 → `record_io_hashes` hashes the controller's tree into every machine's lock; filed as forjar#501, not widened). The other five were confirmations or already satisfied (strip-ANSI, gate G ordering, intermediate signatures).
- The delegate hit its turn cap before writing its receipt; the lane's critique, `lane-1.json` and `lane-reduce.json` were on disk and were read directly. Recorded as a delegate gap, not a lane gap.
- Host tree byte-identical before and after.

## Diff review (phase 3) — three quorum lanes, --mode plan --sandbox, read-only, base pinned

- Diff: main (bc2819cc) ... 3d8a1606, 15 files, +516/−66; lanes read a `--shared` clone under the session scratchpad; no lane ran cargo (a workspace test was running on the host), so every behavioural statement in a lane is reasoning about the diff and was re-run by the orchestrator.
- Lane 1, conv-5da6b238: PASS, C1–C10 confirmed, 879 s.
- Lane 2, conv-fedfc0ec: PASS, C1–C10 confirmed, 987 s; named three rerun commands (the grep for id-keyed lookups, the count of the transport predicate, the line counts) — all three re-run, all agree.
- Lane 3, conv-c8638070: PASS, C1–C10 confirmed, 1305 s; under C9 named the one config shape that changes: a machine alias this host answers for that `probe_covers` misses now plans NoOp and is named, where before it planned Update through the id key. Judged the intended trade: the map answers for what was measured, the row is named, and the predicate's coverage is forjar#485/#495's concern.
- Reduction: `lane-reduce.sh` width 3, `--not-before` the run's own start; dedup 27 findings, uncovered none, dissent none.
- What the lanes could not measure: C4's RED-on-base (measured by the orchestrator at the RED commit: `left: Update, right: NoOp`), and C7's ceiling — which the workspace gate then REFUTED: rustfmt had wrapped the re-export and the line-counting ceiling test read 8. Three lanes confirmed a number by reading it; the gate measured it.
