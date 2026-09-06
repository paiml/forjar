# forjar state generations: ownership and stack-scoped restore (PMAT-162)

Status: DRAFT v3 for quorum (v1 and v2 each reviewed by three lanes on 2026-09-06; every finding folded in, see §11). Spec-first: no implementation lands before this document is merged and three review lanes pass it. Supersedes the refusal shipped in 1.26.0 (PMAT-161: `undo`, `undo --resume` and `rollback` refuse while a state dir holds more than one stack).

## 1. Problem

A state dir (`--state-dir`) is shared by several stacks in the fleet layout: `machines/<m>/forjar.yaml`, each with its own `name:` and its own machines, all applying into one `state/`. Since 1.26.0 the global lock (`forjar.lock.yaml`, schema 1.1) records a stamp per stack — `stacks[<name>] = {file, last_apply, generator, machines, outputs}` — and `apply`, `status` and the wrong-stack guard (`stack_conflict`) are per stack. Generations are not: `state/generations/<N>/` is one global monotonic sequence of WHOLE-DIR snapshots, numbered per state dir, with no record of which stack wrote each one. Restoring generation N replaces every machine dir and the global lock with their content at N. In a shared dir that reverts every other stack to a past it did not choose (measured on the PMAT-161 branch before the refusal: `undo` with bravo's config replayed whichever stack applied last, and a machine absent from the snapshot lost its lock dir).

## 2. Rules (each carries a falsifier in §9 and an anti-vacuity arm)

R1 **Every generation records its owner in the generation's existing metadata.** `state/generations/<N>/.generation.yaml` (`GenerationMeta`, already skipped by `restore_generation_to_state`) gains `owner: {stack: <name>, file: <config path relative to the state dir, as the stamp records it>, machines: [<machine names the apply wrote>]}` and `schema: "1.2"`. No new file: a separate `owner.yaml` would be copied into the live state dir by an older forjar on restore (review finding, generation.rs restore skip-list). A generation whose metadata has no `owner` is a **legacy** generation.

R2 **One sequence, owner-tagged, append-only, walked by lineage.** Generation numbers stay one global monotonic sequence per state dir (no per-stack counters): `forjar generations` lists `N | owner | machines | when`, and `--json` carries `owner`. A restore never rewinds the shared `current` pointer: it APPENDS a revert generation (owner: the invoking lineage; content: the invoking stack's machines as of the restored generation, every other machine as of now; metadata `restores: M`) and moves `current` forward to it — rewinding would truncate every other lineage's timeline (v1 review). Storage is Kubernetes-style (append); navigation is Nix-style (a pointer that walks back): a lineage's POSITION is `G.restores` when its newest generation G is a revert generation, else G. `undo` with no target restores the lineage generation before the position; `--generations K` walks K non-revert lineage generations back from the position; so two undos go two back and never toggle (v2 review: with naive K-back arithmetic over a log holding reverts, the second undo was a redo). Redo is a non-goal (§3): `rollback --generation N` is the explicit way forward. `apply --rollback-on-failure` is unaffected: it records the pre-apply number and restores the failing stack's own machines from it (R4).

R3 **Selection by lineage, where a lineage is the config FILE.** A generation belongs to the invoking stack when its `owner.file` (relative to the state dir, as the stamp records it) equals the invoking config's stamped file; the owner's NAME is display only, so a renamed stack keeps every generation it wrote under its old name (v2 review: keying on the name contradicted R9). `undo` with no target selects by R2's position rule within that lineage; "no target" is expressible (`cmd_undo`'s `generations: u32` defaulting to 1 becomes `Option<u32>`). `undo --generations K` / `rollback --generation N` whose target belongs to another lineage refuses: `generation N belongs to lineage <file> (stack '<other>'), not to <file>`. The same name from a different file is simply another lineage and is refused by that rule. `undo --resume` keys its ledger by (lineage, machine) rather than by machine alone.

R4 **Restore is scoped to the INVOKING stack's machine set, and it is atomic.** Restoring generation N for stack S replaces ONLY S's machine dirs — the machines S's config declares (review: `load_machine_locks` keys on `config.machines`), each taken from N's snapshot when the snapshot holds it and left untouched (and named in the output) when it does not — and in the global lock only `stacks[S]`, S's machine summaries and S's owned outputs (merged via `merge_outputs`, never copied whole). Every other machine dir, every other `stacks[*]` entry and every other stack's outputs are left exactly as they are now — restore is the identity on every non-invoking stack's state (the `pv` contract in §8). The owner tag selects (R3) and gates legacy (R5); it does not define the scope, which is why `apply --rollback-on-failure` is sound: it restores the failing stack's own machines from the pre-apply generation whatever that generation's owner is. Atomicity, honestly: the machine dirs and `forjar.lock.yaml` are siblings directly under the state dir (`lock_file_path`, `global_lock_path` in src/core/state/mod.rs) and the only existing atomic switch is the single `generations/current` symlink, so a SET of machine dirs plus the lock cannot be one `rename(2)` (v2 review, unanimous). The scoped restore is therefore a JOURNALED transaction: (i) write `state/.restore/<txid>.journal` (lineage, target generation, the ordered list of machine dirs to switch, the new global lock content); (ii) stage every new machine dir under `state/.restore/<txid>/<machine>`; (iii) switch each machine dir with two renames (live → `.restore/<txid>/old-<machine>`, staged → live), each rename atomic on its own; (iv) write the global lock through `save_global_lock` (merged, with its `.b3` sidecar); (v) append the revert generation (R2) and remove the journal. A leftover journal means an interrupted restore: every verb that reads the state dir refuses with the journal's txid and the recovery command, and `undo --resume` (whose ledger this journal extends) completes the switch forward; nothing is ever half-switched silently.

R5 **Legacy generations restore only in single-stack dirs.** A generation without `owner.yaml` may be restored only when the dir's `stacks` map has at most one entry (including a migrated 1.0 dir); otherwise the restore refuses naming the generation and the stacks, and says how to proceed (apply once per stack to create owned generations).

R6 **The pre-restore destroy is scoped the same way and keyed by (machine, resource id).** `undo`'s "destroy resources absent from the target" step considers only resources of the invoking stack's config (it already replays that config), keyed by `(machine, resource_id)` rather than by id alone (review finding: `absent_from_target` returns a flat id list), and runs only after R3–R5 have accepted the restore. Nothing destructive happens before the refusal (1.26.0's PMAT-168 rule, kept).

R7 **Migration is additive.** Existing generations gain no `owner`; they are legacy (R5). New generations always carry it inside `.generation.yaml`, which older readers already skip on restore and parse with unknown fields ignored. Reading a dir with mixed generations needs no conversion and writes nothing. An older forjar restores whole-dir as before — the doc states that downgrading a shared dir is unsupported.

R8 **Compatibility with the 1.26.0 refusal, and one decision function.** The refusal is removed only where R3–R5 accept; the same message shape is kept for the cases that still refuse (foreign owner, rename, legacy in a multi-stack dir). EVERY path that writes the state dir from a generation — `undo`, `undo --resume`, `rollback`, and `apply --rollback-on-failure` (`helpers_state::maybe_rollback_generation`, which today calls `rollback_to_generation` directly with no scope — the one bypass all three review lanes named) — calls `restore_decision` first; a direct call to the primitive from anywhere else is a defect the §9 mutation table catches.

R9 **A rename of a stack is one lineage.** (Shipped in 1.26.0 under PMAT-171: a stamp with the invoking config's `file` but another `name` is the same stack renamed; on apply it is retired into the new name, machines and outputs carried, so the multi-stack count never counts a retired name.) Generations: the old name's generations are selectable by the new name only through their `file`. The interlock test `a_renamed_stack_is_unblocked_by_one_apply` exercises a real undo, not a no-op.

## 3. Non-goals

Per-stack generation counters; cross-stack atomic restore; rewriting old generations; changing the per-machine lock format; any change to `apply`'s snapshot cadence.

## 4. Data

```yaml
# state/generations/7/.generation.yaml   (GenerationMeta, schema 1.2 — existing file, new field)
schema: "1.2"
number: 7
created: <iso8601>
owner:
  stack: bravo
  file: ../machines/bravo/forjar.yaml
  machines: [bravo-host]
written_by: forjar 1.27.0
```

## 5. Interfaces

- `generation::create_generation(state_dir, config)` writes `owner` into `.generation.yaml` from the config name, the stamp's relative file and the machines the apply wrote.
- `generation::owner(state_dir, n) -> Option<Owner>`; `generation::newest_owned_by(state_dir, name) -> Option<u32>`.
- `generation::rollback_to_generation(state_dir, n, yes, scope: RestoreScope)` where `RestoreScope::{Stack{name, machines}, WholeDirSingleStack}` is decided by R3–R5, never by a flag; the primitive refuses to run without a decision (it takes the decision value, not a bool).
- `undo`, `undo --resume`, `rollback` and `maybe_rollback_generation` call one decision function `restore_decision(lock, generation_meta, invoking: (name, file, declared_machines)) -> Result<RestoreScope, Refusal>` — the single place the rules live, unit-tested rule by rule.

## 6. Observability

`forjar generations` shows the owner column; a scoped restore prints `restored generation N for stack '<name>' (machines: …); left untouched: <other stacks>`; a refusal prints the rule id (R3/R5) with the sentence above.

## 7. Migration and rollout

Ships behind no flag. Single-stack dirs behave exactly as today (R5 makes every legacy generation restorable there; new ones are owned by the only stack). Shared dirs gain owned generations on their next applies; until each stack has applied once, `undo` for that stack refuses with the R5 sentence.

## 8. `pv` contract (to be written with the implementation, kind: pattern)

`contracts/generation-ownership-v1.yaml`: for every non-owner stack T, `fingerprint(T's machine dirs, stacks[T], T's outputs)` before a restore of an S-owned generation equals the fingerprint after — the identity property; plus the R3 and R5 refusals as rows with their exact sentences.

## 9. Falsifiers (each RED before the implementation, GREEN after; each with an anti-vacuity arm)

| rule | falsifier (binary-level, temp state dir) | anti-vacuity arm |
|---|---|---|
| R1 | apply alpha → `state/generations/<N>/.generation.yaml` carries `owner.stack: alpha` and alpha's machine | a legacy generation dir written by hand has no `owner` and `generations --json` reports `owner: null` |
| R2 | apply alpha, apply bravo, apply alpha → numbers 1,2,3 in one sequence; owners alpha, bravo, alpha; `undo` alpha appends generation 4 (owner alpha) and `current` = 4, never 1 | `--generations 2` from alpha still means "two back" in the global sequence (refused as bravo's, R3) |
| R3 | alpha apply (1), bravo apply (2), bravo apply (3); `undo` with alpha's config → restores generation 1 (alpha's lineage), never 2 or 3 — RED today because the 1.26.0 guard refuses outright (v2 review: the v2 row was already green through the `generations=1` default) | `undo` with bravo's config targets 2, its own previous, not 1; `rollback --generation 1` with bravo's config → refused naming alpha's lineage |
| R2 | alpha apply (1), alpha change (2), alpha change (3); `undo` → appends 4 restoring 2; `undo` again → appends 5 restoring 1 (two back, no toggle); `undo` again → refused, nothing earlier in the lineage | `forjar generations --json` lists 4 and 5 with `restores: 2` and `restores: 1`; `current` is 5 |
| R4 | alpha apply, bravo apply, alpha change, `undo` alpha → alpha's marker reverted; bravo's machine dir, `stacks[bravo]` and bravo's outputs byte-identical | single-stack dir: the whole dir is restored as today (`falsification_undo_actually_undoes` unchanged) |
| R5 | a dir with two stacks and a legacy generation → restore refused with the R5 sentence | the same legacy generation in a single-stack dir restores |
| R6 | alpha's current config declares a resource its target generation lacks, bravo present with a foreign-owner target → refusal, marker untouched | single-stack: the absent resource IS destroyed after the restore is accepted (today's behaviour) |
| R7 | a dir with old generations and new ones lists both; restoring a new one in a shared dir works; the old one refuses (R5) | no conversion file is written on read; a restore by an older forjar copies no `owner` file into the live dir |
| R8 | `apply --rollback-on-failure` for alpha in a shared dir whose pre-apply generation is bravo-owned restores alpha's machines only; bravo byte-identical | single-stack `--rollback-on-failure` unchanged |
| R9 | alpha apply (1), bravo apply (2), rename alpha → alpha2 (same file), apply (3); `undo` with alpha2's config → targets 1 across the rename (same lineage file) and succeeds; bravo byte-identical — RED today (multi-stack guard) | the single-stack rename undo (shipped, PMAT-171) stays green; a different stack with the same name from another file is still refused (`stack_conflict`) |

Mutations the implementation must survive: drop the `owner` write → R1 RED; select by name instead of file, or newest global instead of newest-in-lineage → R3 RED; restore whole-dir, copy the global lock instead of merging it, or skip the journal → R4 RED; rewind `current` instead of appending, or walk K over the raw sequence instead of the lineage position → R2 RED; skip the legacy check → R5 RED; run destroy before the decision, or key it by id alone → R6 RED; call the primitive from `maybe_rollback_generation` or from the `rollback` verb (`dispatch_misc.rs`) without a decision → R8 RED; count a retired name → R9 RED.

## 10. Decisions taken from the v1 review (were open questions)

1. `owner.machines` records the machines the apply WROTE (what the snapshot holds); the restore SCOPE is the invoking config's DECLARED machines (R4), each taken from the snapshot only when present — both lane positions, each where it is right.
2. A restore across a rename is REFUSED (R3); a rename is handled at apply time as one lineage (R9, PMAT-171).
3. One global sequence, append-only (R2): the per-lineage view comes from the owner tag; a restore appends rather than rewinds, which removes the timeline-truncation objection without a second counter.

## 11. Review record

**v2 review** (conv-fc3442f7, conv-b97eddc8, conv-c8ee3f96; 3/3 rejected with the same three defects, all folded into v3): R4 claimed a one-rename atomic switch the sibling layout cannot give — replaced by the journaled transaction; R3 keyed selection on the name and so contradicted R9's file-keyed lineage — selection is now by file; R2's K-back arithmetic over an append-only log toggled on the second undo — replaced by the lineage-position rule; the v2 falsifier rows for R3 and R9 were already green today — both replaced by multi-stack cases that the 1.26.0 guard refuses. One lane's `rollback`-verb bypass claim is a statement about today's code (`dispatch_misc.rs` calls the primitive directly); R8 enumerates that verb and the primitive's signature makes the bypass structurally impossible. Dissent on whether `--rollback-on-failure` survives R2 is settled by R4: the scope is the failing stack's own declared machines, whatever the pre-apply generation's owner.

**v1 review**

Three lanes (conv-d1483f00, conv-17a7ff24, conv-d2f80a07; 133–210 s each) returned do-not-implement-as-written with additive findings only; every finding is folded into R1–R9 above: ownership inside `.generation.yaml` (R1/R7), append-only restore (R2), `Option<u32>` target and rename refusal (R3), atomic scoped restore with a merged global lock (R4), `(machine, id)` destroy keying (R6), the `maybe_rollback_generation` bypass (R8). Dissent on the sequence model (per-stack counters) is resolved by R2's append-only rule together with R4's invoking-stack scope, which answers the corruption scenario the dissenting lane described (a failed alpha apply after bravo's: alpha's own machines are restored from the pre-apply snapshot regardless of its owner).
