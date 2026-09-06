# forjar state generations: ownership and stack-scoped restore (PMAT-162)

Status: DRAFT for quorum. Spec-first: no implementation lands before this document is merged and three review lanes pass it. Supersedes the refusal shipped in 1.26.0 (PMAT-161: `undo`, `undo --resume` and `rollback` refuse while a state dir holds more than one stack).

## 1. Problem

A state dir (`--state-dir`) is shared by several stacks in the fleet layout: `machines/<m>/forjar.yaml`, each with its own `name:` and its own machines, all applying into one `state/`. Since 1.26.0 the global lock (`forjar.lock.yaml`, schema 1.1) records a stamp per stack — `stacks[<name>] = {file, last_apply, generator, machines, outputs}` — and `apply`, `status` and the wrong-stack guard (`stack_conflict`) are per stack. Generations are not: `state/generations/<N>/` is one global monotonic sequence of WHOLE-DIR snapshots, numbered per state dir, with no record of which stack wrote each one. Restoring generation N replaces every machine dir and the global lock with their content at N. In a shared dir that reverts every other stack to a past it did not choose (measured on the PMAT-161 branch before the refusal: `undo` with bravo's config replayed whichever stack applied last, and a machine absent from the snapshot lost its lock dir).

## 2. Rules (each carries a falsifier in §9 and an anti-vacuity arm)

R1 **Every generation records its owner.** `state/generations/<N>/owner.yaml` (schema 1.2 of the generation record) carries `stack: <name>`, `file: <config path relative to the state dir, as the stamp records it>`, `machines: [<machine names the apply wrote>]`, `written_by: forjar <version>`. A generation without `owner.yaml` is a **legacy** generation.

R2 **One sequence, owner-tagged.** Generation numbers stay one global monotonic sequence per state dir (no per-stack counters): `forjar generations` lists `N | owner | machines | when`, and `--json` carries `owner`. Rationale: a single sequence keeps the pre-apply snapshot semantics of `apply --rollback-on-failure` and the existing `--generations N` arithmetic; ownership is a tag, not a namespace.

R3 **Selection by owner.** `undo` with no target selects the newest generation owned by the invoking config's name. `undo --generations N` / `rollback --generation N` whose target's owner is another stack refuses: `generation N was written by stack '<other>', not '<name>'`. `undo --resume` keys its ledger by (stack, machine) rather than by machine alone.

R4 **Restore is scoped to the owner's machine set.** Restoring a generation owned by S replaces ONLY the machine dirs listed in that generation's `owner.machines`, and in the global lock only `stacks[S]`, S's machine summaries and S's owned outputs (merged via `merge_outputs`). Every other machine dir, every other `stacks[*]` entry and every other stack's outputs are left exactly as they are now — restore is the identity on every non-owner stack's state (the `pv` contract in §8).

R5 **Legacy generations restore only in single-stack dirs.** A generation without `owner.yaml` may be restored only when the dir's `stacks` map has at most one entry (including a migrated 1.0 dir); otherwise the restore refuses naming the generation and the stacks, and says how to proceed (apply once per stack to create owned generations).

R6 **The pre-restore destroy is scoped the same way.** `undo`'s "destroy resources absent from the target" step considers only resources of the invoking stack's config (it already replays that config) and runs only after R3–R5 have accepted the restore. Nothing destructive happens before the refusal (1.26.0's PMAT-168 rule, kept).

R7 **Migration is additive.** Existing generations gain no `owner.yaml`; they are legacy (R5). New generations always carry it. Reading a dir with mixed generations needs no conversion. An older forjar ignores `owner.yaml` and restores whole-dir as before — the doc states that downgrading a shared dir is unsupported.

R8 **Compatibility with the 1.26.0 refusal.** The refusal is removed only where R3–R5 accept; the same message shape is kept for the cases that still refuse (foreign owner, legacy in a multi-stack dir).

## 3. Non-goals

Per-stack generation counters; cross-stack atomic restore; rewriting old generations; changing the per-machine lock format; any change to `apply`'s snapshot cadence.

## 4. Data

```yaml
# state/generations/7/owner.yaml   (schema 1.2)
schema: "1.2"
stack: bravo
file: ../machines/bravo/forjar.yaml
machines: [bravo-host]
written_by: forjar 1.27.0
```

## 5. Interfaces

- `generation::create_generation(state_dir, config)` writes `owner.yaml` from the config name, the stamp's relative file and the machines the apply wrote.
- `generation::owner(state_dir, n) -> Option<Owner>`; `generation::newest_owned_by(state_dir, name) -> Option<u32>`.
- `generation::rollback_to_generation(state_dir, n, yes, scope: RestoreScope)` where `RestoreScope::{Owner, WholeDirSingleStack}` is decided by R3–R5, never by a flag.
- `undo` and `rollback` call one decision function `restore_decision(lock, generation_owner, invoking_name) -> Result<RestoreScope, Refusal>` — the single place the rules live, unit-tested rule by rule.

## 6. Observability

`forjar generations` shows the owner column; a scoped restore prints `restored generation N for stack '<name>' (machines: …); left untouched: <other stacks>`; a refusal prints the rule id (R3/R5) with the sentence above.

## 7. Migration and rollout

Ships behind no flag. Single-stack dirs behave exactly as today (R5 makes every legacy generation restorable there; new ones are owned by the only stack). Shared dirs gain owned generations on their next applies; until each stack has applied once, `undo` for that stack refuses with the R5 sentence.

## 8. `pv` contract (to be written with the implementation, kind: pattern)

`contracts/generation-ownership-v1.yaml`: for every non-owner stack T, `fingerprint(T's machine dirs, stacks[T], T's outputs)` before a restore of an S-owned generation equals the fingerprint after — the identity property; plus the R3 and R5 refusals as rows with their exact sentences.

## 9. Falsifiers (each RED before the implementation, GREEN after; each with an anti-vacuity arm)

| rule | falsifier (binary-level, temp state dir) | anti-vacuity arm |
|---|---|---|
| R1 | apply alpha → `state/generations/<N>/owner.yaml` exists with `stack: alpha` and alpha's machine | a legacy generation dir written by hand has no owner.yaml and `generations --json` reports `owner: null` |
| R2 | apply alpha, apply bravo, apply alpha → numbers 1,2,3 in one sequence; owners alpha, bravo, alpha | `--generations 2` from alpha still means "two back" in the global sequence (refused as bravo's, R3) |
| R3 | alpha apply, bravo apply, `undo` with alpha's config → restores generation 1 (alpha's), not 2 | `undo --generations 1` with alpha's config against bravo's generation → refused naming bravo |
| R4 | alpha apply, bravo apply, alpha change, `undo` alpha → alpha's marker reverted; bravo's machine dir, `stacks[bravo]` and bravo's outputs byte-identical | single-stack dir: the whole dir is restored as today (`falsification_undo_actually_undoes` unchanged) |
| R5 | a dir with two stacks and a legacy generation → restore refused with the R5 sentence | the same legacy generation in a single-stack dir restores |
| R6 | alpha's current config declares a resource its target generation lacks, bravo present with a foreign-owner target → refusal, marker untouched | single-stack: the absent resource IS destroyed after the restore is accepted (today's behaviour) |
| R7 | a dir with old generations and new ones lists both; restoring a new one in a shared dir works; the old one refuses (R5) | no conversion file is written on read |

Mutations the implementation must survive: drop the `owner.yaml` write → R1 RED; select newest global instead of newest-owned → R3 RED; restore whole-dir → R4 RED; skip the legacy check → R5 RED; run destroy before the decision → R6 RED.

## 10. Open questions for the review lanes

1. Should `owner.machines` be the machines the apply WROTE or the machines the config DECLARES? (Proposed: wrote — it is what the snapshot holds.)
2. Should a stack be allowed to restore a generation whose owner has the same name but a different `file` (a rename), given `stack_conflict` already refuses the apply? (Proposed: refuse, same rule.)
3. Is one global sequence (R2) the right choice against the Kubernetes model (per-Deployment revisions) named by the CRUX panel's dissent? (Proposed: yes, for `--rollback-on-failure`'s pre-apply snapshot semantics; owner tags give the per-lineage view without a second counter.)
