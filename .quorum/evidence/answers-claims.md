# Quorum evidence — PMAT-223 — the claims put to the lanes

Plan stage (one teamwork lane on the plan, before the fix was written; the lane ran as a single model — fan-out was measured as unknown, so it is read as one reading, not a panel):

P1. The reader's fix is the writer's base plus the machine guard; whatever else a live cache does is FJ-2701's shipped semantics, to be named in the CHANGELOG and filed, not fixed here.
P2. Only two readers of the recorded `input_hash`/`output_hash` exist: the planner (through the (machine, resource) probe map) and the cache reader.
P3. The remote binary case is RED on the base tree.

Diff stage, round 1 (three quorum lanes, base pinned at 643363b3, diff at dda3cf0e) and round 2 (three lanes, same base, diff at f6cb029a):

C1. `record_io_hashes` takes the machine and records neither hash for a machine this host does not answer for; an absent `input_hash` reads as `no recorded input hash` in `staleness_reason`, never as clean.
C2. `probe.rs` names the transport predicate exactly once, inside `probe_answers_for`; `probe_covers`, `record_io_hashes` and `check_task_input_cache` all go through it; the one-definition pin in `tests_unprobed.rs` still holds.
C3. `check_task_input_cache` refuses to answer for a machine this host does not answer for, hashes through `probe_resource` (the writer's base) instead of the state directory's parent, and a hit requires `staleness_reason == None`.
C4. `task_inputs_are_cached` returns false under `--force`.
C5. `settle_cached_row` writes `hash_desired_state(resolved)` into the lock row on a hit, from `prepare_wave_resources`, so the next plan reads `0 to change`.
C6. The binary falsifier drives the real binary with four tests; its doc comment says honestly which case is red on the base and which is red only against a mutant; the committed RED log agrees.
C7. Seven mutation logs under docs/audits/logs each show the named test failing with its hunk removed. (As put to round 1; corrected in round 2 to EIGHT logs over SIX mutations.)
C8. No reader of a remote row's recorded hashes regresses: the planner reads through the keyed map, the cache reader refuses a remote machine, and `verify`'s reader now finds only rows this host answers for; there is no fourth reader in src/.
C9. Every test citation in the contract's new rows and enforcement checks names a test that exists under that exact name.
C10. Every sentence of the CHANGELOG paragraph is true by reading the code, including that every `cache: true` task re-ran on every apply before this.
C11. `src/core/executor/mod.rs` stays at or under 500 lines and no added function exceeds cyclomatic complexity 10.
C12. The diff touches only what the ticket asks for; the one doc hunk outside the named sites corrects a sentence the diff made false, and no other doc sentence is now false and left standing.
