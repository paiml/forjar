# Quorum evidence — PMAT-161 — refuter rulings

Three refuter lanes (conv-c0c81fb1, conv-f4257089, conv-7ecefc8d; 203–373 s) attacked every claim at 5044ab1f. All eight dispositions survived; eight round-1 complaints were refuted as stale (fixed); the single-stack claim was narrowed to observable behaviour; and lane 3 found two genuine gaps — the unguarded second retention path and the suffix match still reachable through `stack_conflict` — both fixed afterwards (D9, D10). The unanimous FAIL verdicts are the brief's REFUTED-implies-FAIL rule applied to stale complaints. Two lanes ruled on ids the dossier does not carry (the range `L1-C1..L3-C6` invited expansion) — those rows are discarded.

## Refuter 1 (verdict FAIL)

L1-C1: SURVIVES — tried to find refuse_multi_stack_restore after stage_target_config but it genuinely precedes it and compute_undo_diff.
L1-C2: SURVIVES — tried to find it refusing a different name with its own machines, but it correctly returns None.
L1-C3: SURVIVES — tried to verify if older forjars preserve the map, but they silently drop it due to lack of schema validation.
L1-C4: SURVIVES — tried to bypass the check, but the test explicitly asserts the warning string and refusal.
L1-C5: SURVIVES — tried to find a way rollback runs in a multi-stack dir, but rollback_on_failure_gate checks it upfront.
L1-C6: SURVIVES — tried to find a change for single-stack dirs, but the length check explicitly bypasses the refusal.
L1-C7: REFUTED — the defect was fixed; the refusal is now at apply.rs:59 via rollback_on_failure_gate before any drift gate or writes.
L1-F1: SURVIVES — same as L1-C1.
L1-F2: SURVIVES — same as L1-C2.
L1-F3: SURVIVES — same as L1-C3.
L1-F4: SURVIVES — same as L1-C4.
L1-F5: SURVIVES — same as L1-C5.
L1-F6: SURVIVES — same as L1-C6.
L1-F7: REFUTED — same as L1-C7.
L2-C1: SURVIVES — verified refuse_multi_stack_restore sits at line 305 and guards correctly.
L2-C2: SURVIVES — same as L1-C2.
L2-C3: SURVIVES — same as L1-C3.
L2-C4: SURVIVES — verified the test enforces the refusal.
L2-C5: SURVIVES — verified undo and rollback implementations match the book.
L2-C6: SURVIVES — same as L1-C6.
L2-F1: SURVIVES — same as L2-C1.
L2-F2: SURVIVES — same as L2-C2.
L2-F3: SURVIVES — same as L2-C3.
L2-F4: SURVIVES — same as L2-C4.
L2-F5: SURVIVES — same as L2-C5.
L2-F6: SURVIVES — same as L2-C6.
L3-C1: SURVIVES — same as L1-C1.
L3-C2: SURVIVES — same as L1-C2.
L3-C3: SURVIVES — same as L1-C3.
L3-C4: SURVIVES — verified the migration call is strictly necessary for the test.
L3-C5: SURVIVES — verified CHANGELOG description aligns with code behavior.
L3-C6: SURVIVES — single-stack dir output is identical and unchanged in observable behavior.
L3-F1: SURVIVES — same as L3-C1.
L3-F2: SURVIVES — same as L3-C2.
L3-F3: SURVIVES — same as L3-C3.
L3-F4: SURVIVES — same as L3-C4.
L3-F5: SURVIVES — same as L3-C5.
L3-F6: SURVIVES — same as L3-C6.
T1: SURVIVES — verified refuse_multi_stack_restore runs before any destructive step.
T2: SURVIVES — verified legacy 1.0 schema initialization works seamlessly.
T3: REFUTED — fixed in PMAT-175; same_config_file now canonicalizes both paths and compares them exactly without ParentDir stripping.
T4: REFUTED — fixed in PMAT-176; claim

Findings:
- R1-F1 [asserted] src/cli/apply.rs:59 — L1-C7 (proposed fix: Moved the refusal to apply_preflight::rollback_on_failure_gate before the drift gate and writes.)
- R1-F2 [asserted] src/cli/apply.rs:59 — L1-F7 (proposed fix: Moved the refusal to apply_preflight::rollback_on_failure_gate before the drift gate and writes.)
- R1-F3 [cited] src/core/state/stamp/identity.rs:76 — T3 (proposed fix: same_config_file now compares exact canonicalized paths instead of stripping ParentDir and suffix matching.)
- R1-F4 [cited] src/core/state/stamp/rename.rs:111 — T4 (proposed fix: claimed_machines now intersects the union of previous and written machines with the declared set.)
- R1-F5 [cited] docs/roadmaps/roadmap.yaml:1882 — T6 (proposed fix: Updated roadmap backlog text to match the shipped decision about upfront refusal.)
- R1-F6 [cited] src/cli/apply.rs:59 — U1 (proposed fix: rollback_on_failure_gate is executed before any writes or drift gates.)
- R1-F7 [cited] src/cli/apply_snapshot.rs:164 — U2 (proposed fix: gc_old_snapshots checks shared_stack_count and skips retention in shared dirs with a note.)
- R1-F8 [cited] src/core/state/stamp/rename.rs:44 — U4 (proposed fix: retire_renamed uses .filter() to match and remove all matching entries instead of just the first.)
- R1-F9 [measured] src/cli/apply_snapshot.rs:143 — F3 (proposed fix: Observable behavior is unchanged, but execution path now loads the global lock via shared_stack_count.)

## Refuter 2 (verdict FAIL)

L1-C1: SURVIVES — guard remains before stage.
L1-C2: SURVIVES — conflict check correctly allows distinct names.
L1-C3: SURVIVES — schema downgrade data loss remains by design.
L1-C4: SURVIVES — test correctly fails without check.
L1-C5: SURVIVES — help string aligns with implementation.
L1-C6: NARROWED — Single-stack output is unchanged, but the path evaluates `rollback_on_failure_gate`.
L1-C7: REFUTED — refusal happens in preflight before first byte.
L1-F1: SURVIVES — guard remains before stage.
L1-F2: SURVIVES — conflict check correctly allows distinct names.
L1-F3: SURVIVES — schema downgrade data loss remains by design.
L1-F4: SURVIVES — test correctly fails without check.
L1-F5: SURVIVES — help string aligns with implementation.
L1-F6: NARROWED — Single-stack output is unchanged, but the path evaluates `rollback_on_failure_gate`.
L1-F7: REFUTED — refusal happens in preflight before first byte.
L2-C1: SURVIVES — guard remains before stage.
L2-C2: SURVIVES — conflict check correctly allows distinct names.
L2-C3: SURVIVES — schema downgrade data loss remains by design.
L2-C4: SURVIVES — test correctly fails without check.
L2-C5: SURVIVES — book sentence aligns with implementation.
L2-C6: SURVIVES — rollback/undo operate without refusal.
L2-F1: SURVIVES — guard remains before stage.
L2-F2: SURVIVES — conflict check correctly allows distinct names.
L2-F3: SURVIVES — schema downgrade data loss remains by design.
L2-F4: SURVIVES — test correctly fails without check.
L2-F5: SURVIVES — book sentence aligns with implementation.
L2-F6: SURVIVES — rollback/undo operate without refusal.
L3-C1: SURVIVES — guard remains before stage.
L3-C2: SURVIVES — conflict check correctly allows distinct names.
L3-C3: SURVIVES — schema downgrade data loss remains by design.
L3-C4: SURVIVES — test correctly fails without check.
L3-C5: SURVIVES — documentation aligns with implementation.
L3-C6: REFUTED — the single-stack path is not completely unchanged (evaluates `rollback_on_failure_gate`).
L3-F1: SURVIVES — guard remains before stage.
L3-F2: SURVIVES — conflict check correctly allows distinct names.
L3-F3: SURVIVES — schema downgrade data loss remains by design.
L3-F4: SURVIVES — test correctly fails without check.
L3-F5: SURVIVES — documentation aligns with implementation.
L3-F6: REFUTED — the single-stack path is not completely unchanged (evaluates `rollback_on_failure_gate`).
T1: SURVIVES — refusal precedes staging.
T2: SURVIVES — 1.0 schema properly migrated.
T3: REFUTED — 

Findings:
- R2-F1 [measured] src/cli/apply.rs:63 — L1-C6 (proposed fix: Single-stack behavior is unchanged in output and intent, but the execution path evaluates `rollback_on_failure_gate`.)
- R2-F2 [cited] src/cli/apply.rs:63 — L1-C7 (proposed fix: The defect INV-REFUSAL-IS-BEFORE-THE-FIRST-BYTE is fixed; the gate runs before any writes.)
- R2-F3 [measured] src/cli/apply.rs:63 — L1-F6 (proposed fix: Single-stack behavior is unchanged in output and intent, but the execution path evaluates `rollback_on_failure_gate`.)
- R2-F4 [cited] src/cli/apply.rs:63 — L1-F7 (proposed fix: The defect INV-REFUSAL-IS-BEFORE-THE-FIRST-BYTE is fixed; the gate runs before any writes.)
- R2-F5 [measured] src/cli/apply.rs:63 — L3-C6 (proposed fix: A single-stack dir does not remain completely unchanged in execution path (it evaluates `rollback_on_failure_gate`).)
- R2-F6 [measured] src/cli/apply.rs:63 — L3-F6 (proposed fix: A single-stack dir does not remain completely unchanged in execution path (it evaluates `rollback_on_failure_gate`).)
- R2-F7 [cited] src/core/state/stamp/identity.rs:76 — T3 (proposed fix: `same_config_file` correctly uses `stamped_config_file` and does not strip `ParentDir`.)
- R2-F8 [cited] src/core/state/stamp/rename.rs:89 — T4 (proposed fix: `next_stamp` unions `previous.machines` via `claimed_machines`.)
- R2-F9 [cited] docs/roadmaps/roadmap.yaml:1882 — T6 (proposed fix: The roadmap backlog accurately reflects the shipped decision for PMAT-162.)
- R2-F10 [cited] src/cli/apply.rs:63 — U1 (proposed fix: `--rollback-on-failure` is refused before writing by `rollback_on_failure_gate`.)
- R2-F11 [cited] src/cli/apply_snapshot.rs:178 — U2 (proposed fix: `gc_old_snapshots` correctly skips retention with a note for shared dirs.)
- R2-F12 [cited] src/core/state/stamp/rename.rs:35 — U4 (proposed fix: `retire_renamed` uses `.filter()` and removes all matching entries.)
- R2-F13 [measured] src/cli/apply.rs:63 — F3 (proposed fix: A single-stack dir is unchanged in output and intent, but the execution path evaluates `rollback_on_failure_gate`.)

## Refuter 3 (verdict FAIL)

L1-F1: SURVIVES — taken at face value.
L1-F2: SURVIVES — checked stack_conflict, correct logic.
L1-F3: SURVIVES — schema downgrade behavior confirmed.
L1-F4: SURVIVES — test correctly asserts conflict.
L1-F5: SURVIVES — help string aligns with implementation.
L1-F6: REFUTED — the single-stack path is not byte-identical (executes apply.rs:64 rollback_on_failure_gate).
L1-F7: REFUTED — the defect was fixed in 00af3c08 (PMAT-174).
L2-F1: SURVIVES — confirmed guard location.
L2-F2: SURVIVES — logic is correct.
L2-F3: SURVIVES — confirmed legacy handling.
L2-F4: SURVIVES — test passes and validates guard.
L2-F5: SURVIVES — documentation aligns.
L2-F6: REFUTED — the single-stack path executes new code lines.
L3-F1: SURVIVES — confirmed guard location.
L3-F2: SURVIVES — logic is correct.
L3-F3: SURVIVES — schema downgrade behaves as documented.
L3-F4: SURVIVES — test validates migration.
L3-F5: SURVIVES — changelog aligns.
L3-F6: REFUTED — the single-stack path executes new code lines.
T1: SURVIVES — tested guard location.
T2: SURVIVES — migration logic is solid.
T3: NARROWED — same_config_file uses exact match only when state_dir is passed; stack_conflict passes None and still uses unattributed_match (ends_with).
T4: REFUTED — fixed in be1309f5 (PMAT-176).
T5: SURVIVES — verified status block logic.
T6: REFUTED — fixed in 22a359f7.
U1: REFUTED — fixed in 00af3c08.
U2: NARROWED — gc_old_snapshots fixed, but gc_generations still trims other stacks' generation snapshots.
U3: SURVIVES — recognized as limitation.
U4: REFUTED — fixed in be1309f5.
X1: SURVIVES — Terraform behavior true.
X2: SURVIVES — Pulumi behavior true.
X3: SURVIVES — Nix behavior true.
X4: SURVIVES — Kubernetes behavior true.
X5: SURVIVES — Ansible behavior true.
D1: SURVIVES — tested apply --rollback-on-failure; refuses before write.
D2: NARROWED — exact match only when state_dir is Some; stack_written_from_other_file passes None.
D3: SURVIVES — correctly keeps declared machines.
D4: REFUTED — 1c6d0d64 fixed gc_old_snapshots, but gc_generations STILL trims other stacks' generation snapshots in a shared dir.
D5: SURVIVES — tested retire_renamed, uses filter correctly.
D6: SURVIVES — correctly documented as limit.
D7: SURVIVES — fixed in 22a359f7.
D8: SURVIVES — fixed in 00af3c08.
F1: SURVIVES — tests verified.
F2: SURVIVES — mutations verified.
F3: REFUTED — single-stack path executes new lines (apply.rs:64).
F4: SURVIVES — coverage verified.

Findings:
- R3-F1 [measured] src/cli/generation/mod.rs:143 — gc_old_snapshots trimmed other stacks' generation snapshots in a shared dir (U2): CONFIRMED. FIXED in 1c6d0d64 (proposed fix: gc_generations lacks shared_stack_count check)
- R3-F2 [measured] src/cli/apply.rs:64 — A single-stack dir is unchanged (proposed fix: Single stack path executes new lines like rollback_on_failure_gate)
- R3-F3 [measured] src/cli/apply.rs:64 — Single-stack behavior is unchanged (proposed fix: Single stack path executes new lines like rollback_on_failure_gate)
- R3-F4 [measured] src/cli/apply.rs:64 — For a single-stack dir, undo and rollback behave exactly as before (proposed fix: Single stack path executes new lines like rollback_on_failure_gate)
- R3-F5 [measured] src/cli/apply.rs:64 — A single-stack dir remains completely unchanged (proposed fix: Single stack path executes new lines like rollback_on_failure_gate)
- R3-F6 [measured] src/core/state/stamp/identity.rs:58 — stamp::same_config_file skipped ParentDir components and used ends_with... FIXED in be1309f5... the two stamped paths are compared exactly as stamped_config_file produces them (proposed fix: same_config_file is exact only if state_dir is passed; unattributed_match still uses ends_with)
- R3-F7 [measured] src/core/state/stamp/identity.rs:58 — CRITICAL: same_config_file incorrectly strips ParentDir components... triggering wrong-file rename retirements (proposed fix: same_config_file is exact only if state_dir is passed; unattributed_match still uses ends_with)
- R3-F8 [measured] src/cli/apply_preflight.rs:88 — the diff has defect INV-REFUSAL-IS-BEFORE-THE-FIRST-BYTE (proposed fix: Fixed by PMAT-174)
- R3-F9 [measured] src/core/state/stamp/rename.rs:133 — HIGH-SEVERITY: Scoped apply (--machine) drops untargeted machines from the state stamp (proposed fix: Fixed by PMAT-176)
- R3-F10 [measured] docs/roadmaps/roadmap.yaml:1882 — The roadmap backlog retains stale, contradictory pre-pivot text about per-stack undo (proposed fix: Fixed by 22a359f7)
- R3-F11 [measured] src/cli/apply_preflight.rs:88 — --rollback-on-failure writes to the state dir before the refusal (proposed fix: Fixed by PMAT-174)
- R3-F12 [measured] src/cli/generation/mod.rs:143 — generation snapshots (gc_old_snapshots) deletes snapshots from other stacks in a shared dir (proposed fix: gc_old_snapshots fixed, but gc_generations still deletes them)
- R3-F13 [measured] src/core/state/stamp/rename.rs:33 — retire_renamed uses .find() which only retires the first matching entry, leaving duplicates behind (proposed fix: Fixed in be1309f5)
