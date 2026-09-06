# forjar state generations: ownership and stack-scoped restore (PMAT-162)

Status: DRAFT v2 for quorum (v1 reviewed by three lanes 2026-09-06; every finding folded in below, see §11). Spec-first: no implementation lands before this document is merged and three review lanes pass it. Supersedes the refusal shipped in 1.26.0 (PMAT-161: `undo`, `undo --resume` and `rollback` refuse while a state dir holds more than one stack).

## 1. Problem

A state dir (`--state-dir`) is shared by several stacks in the fleet layout: `machines/<m>/forjar.yaml`, each with its own `name:` and its own machines, all applying into one `state/`. Since 1.26.0 the global lock (`forjar.lock.yaml`, schema 1.1) records a stamp per stack — `stacks[<name>] = {file, last_apply, generator, machines, outputs}` — and `apply`, `status` and the wrong-stack guard (`stack_conflict`) are per stack. Generations are not: `state/generations/<N>/` is one global monotonic sequence of WHOLE-DIR snapshots, numbered per state dir, with no record of which stack wrote each one. Restoring generation N replaces every machine dir and the global lock with their content at N. In a shared dir that reverts every other stack to a past it did not choose (measured on the PMAT-161 branch before the refusal: `undo` with bravo's config replayed whichever stack applied last, and a machine absent from the snapshot lost its lock dir).

## 2. Rules (each carries a falsifier in §9 and an anti-vacuity arm)

R1 **Every generation records its owner in the generation's existing metadata.** `state/generations/<N>/.generation.yaml` (`GenerationMeta`, already skipped by `restore_generation_to_state`) gains `owner: {stack: <name>, file: <config path relative to the state dir, as the stamp records it>, machines: [<machine names the apply wrote>]}` and `schema: "1.2"`. No new file: a separate `owner.yaml` would be copied into the live state dir by an older forjar on restore (review finding, generation.rs restore skip-list). A generation whose metadata has no `owner` is a **legacy** generation.

R2 **One sequence, owner-tagged, append-only.** Generation numbers stay one global monotonic sequence per state dir (no per-stack counters): `forjar generations` lists `N | owner | machines | when`, and `--json` carries `owner`. A restore never rewinds the `current` pointer: it APPENDS a new generation (owner: the invoking stack; content: the invoking stack's machines as of the restored generation, every other machine as of now) and moves `current` forward to it. Rewinding the shared pointer would truncate every other stack's timeline (review finding). Rationale for one sequence: `apply --rollback-on-failure`'s pre-apply number, the `--generations N` arithmetic and the resume ledger all assume one sequence; ownership is a tag, not a namespace.

R3 **Selection by owner.** `undo` with no target selects the newest generation owned by the invoking config's name; "no target" must be expressible (today `cmd_undo` takes `generations: u32` defaulting to 1 — it becomes `Option<u32>`, review finding). `undo --generations N` / `rollback --generation N` whose target's owner is another stack refuses: `generation N was written by stack '<other>', not '<name>'`. A target whose owner has the invoking name but a different `file` (a rename) is refused the same way — `stage_target_config` resolves relative paths against the current file, so replay across a rename is not sound (review finding, §10 Q2). `undo --resume` keys its ledger by (stack, machine) rather than by machine alone.

R4 **Restore is scoped to the INVOKING stack's machine set, and it is atomic.** Restoring generation N for stack S replaces ONLY S's machine dirs — the machines S's config declares (review: `load_machine_locks` keys on `config.machines`), each taken from N's snapshot when the snapshot holds it and left untouched (and named in the output) when it does not — and in the global lock only `stacks[S]`, S's machine summaries and S's owned outputs (merged via `merge_outputs`, never copied whole). Every other machine dir, every other `stacks[*]` entry and every other stack's outputs are left exactly as they are now — restore is the identity on every non-invoking stack's state (the `pv` contract in §8). The owner tag selects (R3) and gates legacy (R5); it does not define the scope, which is why `apply --rollback-on-failure` is sound: it restores the failing stack's own machines from the pre-apply generation whatever that generation's owner is. Atomicity: today `restore_generation_to_state` does `remove_dir_all` then a recursive copy in place (review finding); the scoped restore stages the new machine dirs and the rewritten global lock in a temp dir under the state dir and switches with the existing atomic rename/symlink primitive; a crash leaves either the old or the new state, never a mix.

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
| R3 | alpha apply, bravo apply, `undo` with alpha's config → restores generation 1 (alpha's), not 2 | `undo --generations 1` with alpha's config against bravo's generation → refused naming bravo |
| R4 | alpha apply, bravo apply, alpha change, `undo` alpha → alpha's marker reverted; bravo's machine dir, `stacks[bravo]` and bravo's outputs byte-identical | single-stack dir: the whole dir is restored as today (`falsification_undo_actually_undoes` unchanged) |
| R5 | a dir with two stacks and a legacy generation → restore refused with the R5 sentence | the same legacy generation in a single-stack dir restores |
| R6 | alpha's current config declares a resource its target generation lacks, bravo present with a foreign-owner target → refusal, marker untouched | single-stack: the absent resource IS destroyed after the restore is accepted (today's behaviour) |
| R7 | a dir with old generations and new ones lists both; restoring a new one in a shared dir works; the old one refuses (R5) | no conversion file is written on read; a restore by an older forjar copies no `owner` file into the live dir |
| R8 | `apply --rollback-on-failure` for alpha in a shared dir whose pre-apply generation is bravo-owned restores alpha's machines only; bravo byte-identical | single-stack `--rollback-on-failure` unchanged |
| R9 | apply as alpha, rename to alpha2 (same file), apply, `undo` → succeeds, one stamp | a genuinely different stack with the same name from another file is still refused (`stack_conflict`) |

Mutations the implementation must survive: drop the `owner` write → R1 RED; select newest global instead of newest-owned → R3 RED; restore whole-dir, or copy the global lock instead of merging it → R4 RED; rewind `current` instead of appending → R2 RED; skip the legacy check → R5 RED; run destroy before the decision, or key it by id alone → R6 RED; call the primitive from `maybe_rollback_generation` without a decision → R8 RED; count a retired name → R9 RED.

## 10. Decisions taken from the v1 review (were open questions)

1. `owner.machines` records the machines the apply WROTE (what the snapshot holds); the restore SCOPE is the invoking config's DECLARED machines (R4), each taken from the snapshot only when present — both lane positions, each where it is right.
2. A restore across a rename is REFUSED (R3); a rename is handled at apply time as one lineage (R9, PMAT-171).
3. One global sequence, append-only (R2): the per-lineage view comes from the owner tag; a restore appends rather than rewinds, which removes the timeline-truncation objection without a second counter.

## 11. v1 review record

Three lanes (conv-d1483f00, conv-17a7ff24, conv-d2f80a07; 133–210 s each) returned do-not-implement-as-written with additive findings only; every finding is folded into R1–R9 above: ownership inside `.generation.yaml` (R1/R7), append-only restore (R2), `Option<u32>` target and rename refusal (R3), atomic scoped restore with a merged global lock (R4), `(machine, id)` destroy keying (R6), the `maybe_rollback_generation` bypass (R8). Dissent on the sequence model (per-stack counters) is resolved by R2's append-only rule together with R4's invoking-stack scope, which answers the corruption scenario the dissenting lane described (a failed alpha apply after bravo's: alpha's own machines are restored from the pre-apply snapshot regardless of its owner).
