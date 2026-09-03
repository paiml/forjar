# Quorum evidence — #449 (fix/destroy-snapshots-a-generation) — adjudicated claims

## CONFIRMED — 7 claims survived refutation

1. [probe] (explains-symptom) `destroy` recorded no generation, so `undo` after a destroy had nothing earlier than "current" to rewind to and the destroy→undo roundtrip named by `contracts/destroy-undo-roundtrip-v1.yaml` could not happen.
   - evidence: at the merge-base `cmd_destroy` removed the lock files (`src/cli/destroy.rs:52`, `src/cli/destroy.rs:61`) and returned (`src/cli/destroy.rs:255`) without the call `apply` makes after every run (`src/cli/apply_snapshot.rs:96`, `maybe_record_generation`, GH-376: failures included). Measured on the S4 dogfood sandbox with `policy.snapshot_generations: 3`: apply → generation 0; destroy → still only 0; undo → "generation 0 is current, so only 0 earlier generation(s) exist" (`src/cli/undo.rs:257`). Pinned by `destroy_records_a_generation` and `destroy_then_undo_restores_the_managed_file` in `tests/falsification_destroy_undo_roundtrip.rs`, both RED on the unfixed tree.

2. [design] The generation a destroy records carries the config that PRODUCED the destroyed state: the current config with every successfully destroyed resource removed. (Reworked after the agy lane refuted the first version — see REFUTED 2.)
   - evidence: `cmd_destroy` prunes `succeeded_resources` out of a clone of the config before the one `maybe_record_generation` call at its end (the region after `src/cli/destroy.rs:255` at the merge-base); `dry_run`, `snapshot_generations == 0` and `RECORDING_PAUSED` are honoured identically to apply. `undo_onto_the_destroy_generation_leaves_the_resources_destroyed` (apply → destroy → apply → undo → the file must be ABSENT) is RED with the unpruned config and green with this one.

3. [probe] `destroy` left the lock's BLAKE3 sidecar behind, so the first `apply` after a destroy was refused by the integrity check.
   - evidence: `cleanup_state_files` removed `state.lock.yaml` (`src/cli/destroy.rs:61`) and nothing else; `cleanup_succeeded_entries` rewrote the lock with `fs::write` (`src/cli/destroy.rs:86`) and never re-sealed it. The integrity check treats exactly that pairing as tampering (`src/core/state/integrity.rs:76`, `src/core/state/integrity.rs:167`: "lock file … is missing but its BLAKE3 sidecar survives"). Found by the fourth falsifier case, which died at its second apply with that error; pinned by `apply_after_destroy_is_not_refused_by_a_stale_sidecar`. Fix: the whole-machine path removes the sidecar too; the partial path re-seals through `write_b3_sidecar` (`src/core/state/integrity.rs:9`).

4. [probe] (pre-existing, independent of destroy) `undo` announced "will be destroyed" for every resource the target generation lacks and then destroyed nothing, because its replay is an `apply` and apply never removes.
   - evidence: the announcement at `src/cli/undo.rs:47` and `src/cli/undo.rs:61`; the replay at `src/cli/undo.rs:336` → `src/cli/undo.rs:342` is `cmd_apply` with `force`; no destroy call exists in `undo.rs` at the merge-base. Reproduced without any destroy by `undo_destroys_what_the_target_generation_does_not_hold` (apply a → apply a,b → undo: "b (local): will be destroyed" printed, b still on disk). With the pruned destroy generation of claim 2 this is also why the poisoned-rollback case stayed red: undo landed on the right generation and could not remove what the second apply had re-created.

5. [design] `undo` now destroys, before the rollback, every resource the live locks hold that the target generation's locks do not — with the CURRENT config's definitions, generation recording paused. [A]
   - evidence: `src/cli/undo_prune.rs` (`absent_from_target`, `narrowed_config`, `destroy_absent_from_target`) called from `cmd_undo` immediately before `rollback_to_generation` (`src/cli/undo.rs:303` at the merge-base). It runs before the rollback because the destroy needs the live locks (pre-hash for the destroy log) and the current config's resource definitions — the target generation, by construction, no longer declares them. The narrowed config keeps machines, params and secrets, drops `depends_on` edges into kept resources (the resolver rejects dangling edges), and is staged under the CURRENT generation's number so it cannot collide with the target's already-staged replay file. Neutering this one call turns cases 4 and 6 red and leaves the other four green (mutation observed).

6. [probe] `undo` and `apply` were not the generation defect.
   - evidence: the control — apply(one), apply(two), `undo --yes` — restores `one` with generations 0 and 1 on the merge-base and on the branch (`control_apply_apply_undo_restores_the_earlier_generation` green on both).

7. [design] The falsifiers cannot pass vacuously.
   - evidence: every case drives the built binary and asserts the BYTES at the managed paths (never the summary line) and the numbered entries under `state/generations`; the control fails on a broken `undo`, the generation case on a destroy that records nothing, the sidecar case on a surviving `.b3`, and cases 4 and 6 on an undo that only announces.

## REFUTED — 3 claims killed

1. [design] refuted 1/1 — "Record the generation BEFORE destroy touches the lock, so the snapshot holds the pre-destroy state directly."
   - corrected: that inverts apply's convention (a generation is the record of what a run PRODUCED, GH-376) and would make `undo` after a destroy rewind to a generation identical to current; recording after, as apply does, keeps one rule for both verbs and the control proves the rewind-to-previous mechanism.

2. [design] refuted 1/1 (agy lane) — "Recording the destroy's generation under the unchanged config is enough: undo rewinds to the previous generation and re-converges from its config."
   - corrected: the agy review showed that generation pairs an EMPTY lock with a config that still declares everything ("poisoned"): apply → destroy → apply → undo lands on it and `replay_generation` re-creates what the destroy removed. Taken: the recorded config is pruned to what survived the destroy (claim 2), and the poisoned-rollback case is in the falsifier.

3. [design] refuted 1/1 (agy lane) — "The first falsifier proved the roundtrip."
   - corrected: it only ever rewound to generation 0 and never replayed the destroy's own generation; cases 4–6 now do.

## KNOWN LIMITS (carried, not fixed here) [A]

- `snapshot_generations: 1` — the destroy's generation evicts generation 0 through `gc_generations`, so `undo` after that destroy has no target. Same behaviour as apply with the same policy; a policy floor is a separate ticket.
- `destroy` has no `--dry-run`; the `false` passed to `maybe_record_generation` is the truth of that verb today, not a mask.
- `undo`'s new destroy step is not resumable through `undo --resume` (it runs before the progress ledger is written, by design: the ledger is written after the rollback wipes the machine dir).
