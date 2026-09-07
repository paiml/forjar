# Quorum evidence — PMAT-161 — adjudicated claims (majority of three judges)

67 claim ids (three blind claim lanes with their findings, the teamwork review, the plan-mode fallback, the crux rows, ten orchestrator dispositions and four measured claims) were put to three refuters (round 3, per-lane clones, HEAD 5044ab1f) and then to three judges (per-lane clones, HEAD 8182135c; conv-ee411eec 365 s, conv-3e2a2d4b 169 s, conv-2b9ffc36 184 s). All three judges returned PASS: no claim asserts a defect unfixed at HEAD. The rule is a majority of the three; CONFIRMED-AS-NARROWED counts as survived. Two judges labelled the ten dispositions REFUTED-as-stale and one CONFIRMED — the same commits, the same code state — and the orchestrator records a disposition as CONFIRMED when its sentence is true of HEAD (it names its fix commit) and the complaint it answers as REFUTED-as-stale. Judge 3 omitted the C-family ids; its rulings agree with the others wherever they overlap. Claim text is reproduced as posed at 41c0eac2; a citation whose line no longer resolves in the base tree or the pushed tree is kept as a bare path.

## REFUTED — 8 claims killed (each 3-0, all stale: the defect was real and its fix is in the diff)

1. [plan-fallback] U1 — src/cli/apply.rs:174 — --rollback-on-failure writes to the state dir before the refusal (proposed fix: Check refuse_multi_stack_restore at the beginning of cmd_apply if rollback_on_failure is enabled.)
   - evidence: REFUTED as stale — stale: fixed in 00af3c08; the sentence above is reproduced as the lane posed it, and the fix commit's tests are named in the matching D-row.

2. [plan-fallback] U2 — src/cli/apply_snapshot.rs:118 — generation snapshots (gc_old_snapshots) deletes snapshots from other stacks in a shared dir (proposed fix: Scope generation snapshots to the stack name or segregate them by stack in the shared dir.)
   - evidence: REFUTED as stale — stale: fixed in 1c6d0d64 and 6c5d9416; the sentence above is reproduced as the lane posed it, and the fix commit's tests are named in the matching D-row.

3. [plan-fallback] U4 — src/core/state/stamp/rename.rs:33 — retire_renamed uses .find() which only retires the first matching entry, leaving duplicates behind (proposed fix: Use .retain() or a loop to remove all matching entries instead of just the first.)
   - evidence: REFUTED as stale — stale: fixed in be1309f5; the sentence above is reproduced as the lane posed it, and the fix commit's tests are named in the matching D-row.

4. [resolver] L1-C7 — the diff has defect INV-REFUSAL-IS-BEFORE-THE-FIRST-BYTE at src/cli/apply.rs:317.
   - evidence: REFUTED as stale — stale: fixed in 00af3c08 (PMAT-174); the sentence above is reproduced as the lane posed it, and the fix commit's tests are named in the matching D-row.

5. [resolver] L1-F7 — src/cli/apply.rs:317 — the diff has defect INV-REFUSAL-IS-BEFORE-THE-FIRST-BYTE (proposed fix: Check refuse_multi_stack_restore at the beginning of cmd_apply_scoped when rollback_on_failure is true, before any resources are destroyed.)
   - evidence: REFUTED as stale — stale: fixed in 00af3c08; the sentence above is reproduced as the lane posed it, and the fix commit's tests are named in the matching D-row.

6. [teamwork] T3 — src/core/state/stamp/mod.rs:262 — CRITICAL: `same_config_file` incorrectly strips `ParentDir` components, causing relative paths like `../forjar.yaml` to collapse into a wildcard matching any relative `*/forjar.yaml`, triggering wrong-file rename retirements.
   - evidence: REFUTED as stale — stale: fixed in be1309f5 and 507b6103; the sentence above is reproduced as the lane posed it, and the fix commit's tests are named in the matching D-row.

7. [teamwork] T4 — src/core/state/stamp/rename.rs:58 — HIGH-SEVERITY: Scoped apply (`apply --machine`) drops untargeted machines from the state stamp because `next_stamp` does not union `previous.machines`, allowing sibling stacks to bypass the ownership guard.
   - evidence: REFUTED as stale — stale: fixed in be1309f5; the sentence above is reproduced as the lane posed it, and the fix commit's tests are named in the matching D-row.

8. [teamwork] T6 — docs/roadmaps/roadmap.yaml:1882 — The code correctly implements the widened criteria (upfront restore refusal), but the roadmap backlog retains stale, contradictory pre-pivot text about per-stack undo.
   - evidence: REFUTED as stale — stale: fixed in 22a359f7; the sentence above is reproduced as the lane posed it, and the fix commit's tests are named in the matching D-row.

## CONFIRMED — 59 claims survived refutation (44 as written, 15 as narrowed)

1. [cli] L2-C1 — The multi-stack guard `refuse_multi_stack_restore(state_dir, Some(target))` sits at `src/cli/undo.rs:305`, running after checking the `--yes` confirmation and printing the changes diff, but before `stage_target_config` and `destroy_absent_from_target`.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

2. [cli] L2-C2 — `stack_conflict` at `src/core/state/stamp/mod.rs:323` refuses the same config name from a different `-f` or a machine another stack owns, but lets through a different config name with its own machines.
   - corrected: `stack_conflict` at `src/core/state/stamp/mod.rs:271` refuses the same config name from a different `-f` or a machine another stack owns, but lets through a different config name with its own machines. (the judges relocated stack_conflict to src/core/state/stamp/mod.rs:271)
   - evidence: CONFIRMED-AS-NARROWED by the judges (lane 1 for the stack_conflict line, all three for the single-stack family); the substance held under three refuters.

3. [cli] L2-C3 — A 1.0 lock on load migrates its stamp to the `stacks` map with no recorded `-f` (`file: None`), and after a 1.1 save, an older forjar ignores and drops the `stacks` map on re-save as documented in `src/core/types/state_types.rs:26`.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

4. [cli] L2-C4 — Removing the `restore::refuse_multi_stack_restore(state_dir, Some(generation))?;` call at `src/cli/generation/mod.rs:83` makes the test `a_state_dir_holding_several_stacks_refuses_every_restore` fail.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

5. [cli] L2-C5 — The book sentence 'undo and rollback are not part of that support: both refuse outright while the shared dir holds more than one stack' at `docs/book/src/08-state-management.md:509` is true of the code's refusal implementation.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

6. [cli] L2-C6 — For a single-stack dir, `undo` and `rollback` behave exactly as before without being refused, evidenced by `multi_stack_restore_refusal` at `src/core/state/stamp/mod.rs:160` explicitly returning `None` when `names.len() <= 1`.
   - corrected: For a single-stack dir, `undo` and `rollback` behave exactly as before without being refused, evidenced by `multi_stack_restore_refusal` at `src/core/state/stamp/mod.rs:160` explicitly returning `None` when `names.len() <= 1`. — narrowed by the judges: the observable output, exit code and state dir are unchanged, while the unscoped path now evaluates rollback_on_failure_gate (src/cli/apply.rs:63) and shared_stack_count, which return early.
   - evidence: CONFIRMED-AS-NARROWED by the judges (lane 1 for the stack_conflict line, all three for the single-stack family); the substance held under three refuters.

7. [cli] L2-F1 — src/cli/undo.rs:305 — The multi-stack guard refuse_multi_stack_restore sits in cmd_undo and runs after printing the diff and checking the --yes flag but before stage_target_config and destroy_absent_from_target.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

8. [cli] L2-F2 — src/core/state/stamp/mod.rs:323 — stack_conflict refuses the same config name from a different -f or a machine another stack owns, but lets through a different config name with its own machines.
   - corrected: src/core/state/stamp/mod.rs:271 — stack_conflict refuses the same config name from a different -f or a machine another stack owns, but lets through a different config name with its own machines. (the judges relocated stack_conflict to src/core/state/stamp/mod.rs:271)
   - evidence: CONFIRMED-AS-NARROWED by the judges (lane 1 for the stack_conflict line, all three for the single-stack family); the substance held under three refuters.

9. [cli] L2-F3 — src/core/types/state_types.rs:26 — A 1.0 lock on load migrates its single stamp into the stacks map with no recorded -f, and after a 1.1 save, an older forjar parses it but drops the stacks map when re-saving.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

10. [cli] L2-F4 — src/cli/generation/mod.rs:83 — Removing the restore::refuse_multi_stack_restore call makes the a_state_dir_holding_several_stacks_refuses_every_restore test fail.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

11. [cli] L2-F5 — docs/book/src/08-state-management.md:509 — The document sentence 'undo and rollback are not part of that support: both refuse outright while the shared dir holds more than one stack' is true of the code.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

12. [cli] L2-F6 — src/core/state/stamp/mod.rs:160 — For a single-stack dir, undo and rollback behave exactly as before without being refused, because multi_stack_restore_refusal returns None.
   - corrected: src/core/state/stamp/mod.rs:160 — For a single-stack dir, undo and rollback behave exactly as before without being refused, because multi_stack_restore_refusal returns None. — narrowed by the judges: the observable output, exit code and state dir are unchanged, while the unscoped path now evaluates rollback_on_failure_gate (src/cli/apply.rs:63) and shared_stack_count, which return early.
   - evidence: CONFIRMED-AS-NARROWED by the judges (lane 1 for the stack_conflict line, all three for the single-stack family); the substance held under three refuters.

13. [crux] X1 — Terraform:? — Terraform identifies stacks via workspaces with isolated state files and lineage IDs. It relies on operators not to import the same resource into multiple workspaces, as it lacks cross-workspace locks on physical resources. CLI workspace renames require manual state moves [X].
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

14. [crux] X2 — Pulumi:? — Pulumi identifies deployments by project and stack name. It natively supports stack renaming (preserving lineage) but, like Terraform, does not inherently prevent two stacks from importing and fighting over the same physical resource [X].
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

15. [crux] X3 — Nix:? — Nix identifies stacks using profile symlinks pointing to specific generations. Multiple profiles can exist on a machine, but renaming is not natively modeled beyond updating symlinks [X].
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

16. [crux] X4 — Kubernetes:? — Kubernetes relies on namespaces for isolation and ownerReferences to explicitly prevent controllers from adopting and fighting over the same child resources. Resources cannot be natively renamed [X].
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

17. [crux] X5 — Ansible:? — Ansible is stateless, operating without a central state file or stack identity. It does not detect wrong stacks or track resource ownership, allowing playbooks to overwrite each other [X].
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

18. [disposition] D1 — — `apply --rollback-on-failure` ran the whole apply and reached the multi-stack refusal only inside `maybe_rollback_generation`, which swallowed it into a warning (L1-C7, U1, the delegate's note): CONFIRMED. FIXED in 00af3c08 (PMAT-174): when `rollback_on_failure` is set the pre-flight calls `refuse_multi_stack_restore` before the drift gate, the SSH sockets and any write, and `maybe_rollback_generation` propagates the refusal as an Err. Test: `rename_cases::rollback_on_failure_is_refused_before_the_apply_writes_anything`.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

19. [disposition] D10 — — the refuters found `stack_conflict` calling `same_config_file` with no state dir, falling into the suffix match PMAT-175 removed from the rename path: CONFIRMED. FIXED in 507b6103 (PMAT-183): the state dir is a required argument of `stack_conflict`, `machine_owner`, `same_config_file` and `records_config_file`; `unattributed_match` is deleted; both callers pass it.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

20. [disposition] D2 — — `stamp::same_config_file` skipped ParentDir components and used `ends_with`, so `../forjar.yaml` matched any `*/forjar.yaml` and `retire_renamed` could retire another stack (teamwork, with an executed reproducer): CONFIRMED. FIXED in be1309f5 (PMAT-175): the two stamped paths are compared exactly as `stamped_config_file` produces them; a file-less entry never matches.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

21. [disposition] D3 — — a scoped `apply -m X` rewrote the stamp's machines from the filtered results, releasing the stack's other machines (teamwork): CONFIRMED. FIXED in be1309f5 (PMAT-176): machines = (previous ∪ written) ∩ declared, the declared set passed from the config at the apply_output call site (`machines.retain(|m| declared.contains(m))`).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

22. [disposition] D4 — — `gc_old_snapshots` trimmed other stacks' generation snapshots in a shared dir (U2): CONFIRMED. FIXED in 1c6d0d64 (PMAT-177): retention is skipped with a `note:` while the global lock holds more than one stack; single-stack retention unchanged.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

23. [disposition] D5 — — `retire_renamed` used `.find()` and retired only the first matching entry (U4): CONFIRMED. FIXED in be1309f5: every matching entry is retired (`stamp/rename.rs`, doc comment at line 23).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

24. [disposition] D6 — — an older forjar re-saving a 1.1 lock drops the `stacks` map (U3 calls it a defect; L1-C3, L2-C3, L3-C3 call it benign): REJECTED as a defect. It is the documented limit (CHANGELOG, the book page): the new reader fails closed on an unknown schema, and downgrading a shared state dir is unsupported. No code change.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

25. [disposition] D7 — — the roadmap kept stale pre-pivot text about per-stack undo (teamwork): CONFIRMED. FIXED in 22a359f7: the acceptance criterion states the shipped decision.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

26. [disposition] D8 — — L1-C7 (`INV-REFUSAL-IS-BEFORE-THE-FIRST-BYTE at apply.rs:317`) is D1 stated from the apply side; same fix, same test.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

27. [disposition] D9 — — the refuters found a second retention path, `gc_generations` (src/cli/generation/mod.rs), that PMAT-177 did not guard: CONFIRMED. FIXED in 6c5d9416 (PMAT-182): one helper, `stamp::retention::skip_note`, serves both sweeps; `gc_generations` removes nothing and prints one note while the lock holds more than one stack.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

28. [measured] F1 — (updated at 507b6103: falsification_state_stamp_per_name 18 passed; `cargo test --lib` 13482 passed, 0 failed, 4 ignored; fmt and clippy clean) — Acceptance at fa9c0061: tests/falsification_state_stamp_per_name 14 passed, falsification_undo_replay_fidelity 6, falsification_undo_state_dir_interlock 6, falsification_undo_actually_undoes 8; `cargo test --lib -- core::state undo generation state_identity apply_snapshot cov_apply_b` 440 passed; `pv validate contracts/undo-refuses-multi-stack-state-dir-v1.yaml` valid; `cargo fmt --all -- --check` clean; `cargo clippy --all-targets -- -D warnings` clean.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

29. [measured] F2 — — Three mutations of the guard at 5044ab1f, each restored afterwards: the multi-stack threshold disabled (`names.len() <= 1` → `<= 99` in src/core/state/stamp/mod.rs) turns 4 of 14 tests red (`rollback_on_failure_is_refused_before_the_apply_writes_anything`, `a_refused_undo_destroys_nothing_on_the_way_to_refusing`, `a_state_dir_holding_several_stacks_refuses_every_restore`, `a_renamed_stack_is_one_lineage_not_two_stacks`); the threshold moved to three stacks (`<= 2`) turns the two-stack arm of `a_renamed_stack_is_one_lineage_not_two_stacks` red; the guard call removed from `cmd_undo` (src/cli/undo.rs:305 commented out) turns `a_refused_undo_destroys_nothing_on_the_way_to_refusing` red. No `stack_conflict` test moved under any mutation (discrimination).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

30. [measured] F3 — — A single-stack dir's OBSERVABLE behaviour is unchanged (the refuters narrowed the claim: an unscoped single-stack run now evaluates `rollback_on_failure_gate` and `shared_stack_count`, which return early; its output, exit code and state dir are identical): `a_single_stack_dir_behaves_exactly_as_before` (apply, apply, undo, no warning), the six interlock tests and the eight `undo_actually_undoes` tests pass with intent unchanged; `status` prints its multi-stack block only when `stacks.len() > 1` (src/cli/status_core.rs); `multi_stack_restore_refusal` returns None for one stack (src/core/state/stamp/mod.rs:168).
   - corrected: — A single-stack dir's OBSERVABLE behaviour is unchanged (the refuters narrowed the claim: an unscoped single-stack run now evaluates `rollback_on_failure_gate` and `shared_stack_count`, which return early; its output, exit code and state dir are identical): `a_single_stack_dir_behaves_exactly_as_before` (apply, apply, undo, no warning), the six interlock tests and the eight `undo_actually_undoes` tests pass with intent unchanged; `status` prints its multi-stack block only when `stacks.len() > 1` (src/cli/status_core.rs); `multi_stack_restore_refusal` returns None for one stack (src/core/state/stamp/mod.rs:168). — narrowed by the judges: the observable output, exit code and state dir are unchanged, while the unscoped path now evaluates rollback_on_failure_gate (src/cli/apply.rs:63) and shared_stack_count, which return early.
   - evidence: CONFIRMED-AS-NARROWED by the judges (lane 1 for the stack_conflict line, all three for the single-stack family); the substance held under three refuters.

31. [measured] F4 — — Coverage on origin/main measured with `cargo llvm-cov --locked --ignore-run-fail --summary-only` on a clean clone: lines 96.54% for the forjar crate, 96.41% for the workspace — above the 95 floor before this branch; the branch adds 40+ tests and removes none.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

32. [plan-fallback] U3 — src/core/types/state_types.rs:1 — Older forjars will silently erase the stacks map because they lack check_schema and deserialize 1.1 as 1.0 (proposed fix: N/A (Older forjars cannot be updated, but this should be explicitly handled or warned about).)
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

33. [resolver] L1-C1 — The multi-stack guard sits at src/cli/undo.rs:305, and on the undo path `stage_target_config` runs after it while `compute_undo_diff` runs before it.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

34. [resolver] L1-C2 — `stack_conflict` at src/core/state/stamp/mod.rs:323 refuses the same name from a different config file or a machine another stack owns, but lets through a different name with its own machines.
   - corrected: `stack_conflict` at src/core/state/stamp/mod.rs:271 refuses the same name from a different config file or a machine another stack owns, but lets through a different name with its own machines. (the judges relocated stack_conflict to src/core/state/stamp/mod.rs:271)
   - evidence: CONFIRMED-AS-NARROWED by the judges (lane 1 for the stack_conflict line, all three for the single-stack family); the substance held under three refuters.

35. [resolver] L1-C3 — A 1.0 lock is migrated in memory on load by `state::stamp::migrate`, and an older forjar parsing a 1.1 file drops the `stacks` map when it re-saves while keeping `name`, `machines`, and `outputs` at src/core/types/state_types.rs:21.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

36. [resolver] L1-C4 — The test `a_machine_another_stack_owns_is_a_conflict` in src/core/state/tests_stack_stamp.rs:213 fails if the conflict check at src/core/state/stamp/mod.rs:332 is bypassed.
   - corrected: The test `a_machine_another_stack_owns_is_a_conflict` in src/core/state/tests_stack_stamp.rs:213 fails if the conflict check at src/core/state/stamp/mod.rs:271 is bypassed. (the judges relocated stack_conflict to src/core/state/stamp/mod.rs:271)
   - evidence: CONFIRMED-AS-NARROWED by the judges (lane 1 for the stack_conflict line, all three for the single-stack family); the substance held under three refuters.

37. [resolver] L1-C5 — The help string at src/cli/commands/apply_args.rs:60 stating that rollback refuses outright for multi-stack dirs is true of the code.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

38. [resolver] L1-C6 — Single-stack behavior is unchanged because `multi_stack_restore_refusal` at src/core/state/stamp/mod.rs:161 returns `None` when `names.len() <= 1`.
   - corrected: Single-stack behavior is unchanged because `multi_stack_restore_refusal` at src/core/state/stamp/mod.rs:161 returns `None` when `names.len() <= 1`. — narrowed by the judges: the observable output, exit code and state dir are unchanged, while the unscoped path now evaluates rollback_on_failure_gate (src/cli/apply.rs:63) and shared_stack_count, which return early.
   - evidence: CONFIRMED-AS-NARROWED by the judges (lane 1 for the stack_conflict line, all three for the single-stack family); the substance held under three refuters.

39. [resolver] L1-F1 — src/cli/undo.rs:305 — The multi-stack guard sits at src/cli/undo.rs:305, and on the undo path `stage_target_config` runs after it while `compute_undo_diff` runs before it.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

40. [resolver] L1-F2 — src/core/state/stamp/mod.rs:323 — `stack_conflict` at src/core/state/stamp/mod.rs:323 refuses the same name from a different config file or a machine another stack owns, but lets through a different name with its own machines.
   - corrected: src/core/state/stamp/mod.rs:271 — `stack_conflict` at src/core/state/stamp/mod.rs:271 refuses the same name from a different config file or a machine another stack owns, but lets through a different name with its own machines. (the judges relocated stack_conflict to src/core/state/stamp/mod.rs:271)
   - evidence: CONFIRMED-AS-NARROWED by the judges (lane 1 for the stack_conflict line, all three for the single-stack family); the substance held under three refuters.

41. [resolver] L1-F3 — src/core/types/state_types.rs:21 — A 1.0 lock is migrated in memory on load by `state::stamp::migrate`, and an older forjar parsing a 1.1 file drops the `stacks` map when it re-saves while keeping `name`, `machines`, and `outputs` at src/core/types/state_types.rs:21.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

42. [resolver] L1-F4 — src/core/state/tests_stack_stamp.rs:213 — The test `a_machine_another_stack_owns_is_a_conflict` fails if the conflict check at src/core/state/stamp/mod.rs:332 is bypassed.
   - corrected: src/core/state/tests_stack_stamp.rs:213 — The test `a_machine_another_stack_owns_is_a_conflict` fails if the conflict check at src/core/state/stamp/mod.rs:271 is bypassed. (the judges relocated stack_conflict to src/core/state/stamp/mod.rs:271)
   - evidence: CONFIRMED-AS-NARROWED by the judges (lane 1 for the stack_conflict line, all three for the single-stack family); the substance held under three refuters.

43. [resolver] L1-F5 — src/cli/commands/apply_args.rs:60 — The help string at src/cli/commands/apply_args.rs:60 stating that rollback refuses outright for multi-stack dirs is true of the code.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

44. [resolver] L1-F6 — src/core/state/stamp/mod.rs:161 — Single-stack behavior is unchanged because `multi_stack_restore_refusal` at src/core/state/stamp/mod.rs:161 returns `None` when `names.len() <= 1`.
   - corrected: src/core/state/stamp/mod.rs:161 — Single-stack behavior is unchanged because `multi_stack_restore_refusal` at src/core/state/stamp/mod.rs:161 returns `None` when `names.len() <= 1`. — narrowed by the judges: the observable output, exit code and state dir are unchanged, while the unscoped path now evaluates rollback_on_failure_gate (src/cli/apply.rs:63) and shared_stack_count, which return early.
   - evidence: CONFIRMED-AS-NARROWED by the judges (lane 1 for the stack_conflict line, all three for the single-stack family); the substance held under three refuters.

45. [teamwork] T1 — src/cli/undo.rs:305 — `refuse_multi_stack_restore` strictly precedes all staging, pruning, and generation snapshots, preventing cross-stack corruption during restores.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

46. [teamwork] T2 — src/core/state/stamp/mod.rs:202 — `stamp::migrate` correctly initializes legacy 1.0 schema entries with `file: None` and provides a one-apply grace period without spurious warnings.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

47. [teamwork] T5 — src/cli/status_core.rs:131 — Multi-stack summary blocks are strictly gated by `stacks.len() > 1`, ensuring single-stack directories remain byte-identical to pre-PMAT-161 output.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

48. [tests-docs] L3-C1 — The multi-stack guard sits at src/cli/undo.rs:305 on the undo path, running right after the read-only dry-run/yes checks and before any destructive operations like destroy_absent_from_target.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

49. [tests-docs] L3-C2 — The stack_conflict function at src/core/state/stamp/mod.rs:323 refuses the same name applied from a different config file or a machine another stack owns, but lets through different names with their own machines.
   - corrected: The stack_conflict function at src/core/state/stamp/mod.rs:271 refuses the same name applied from a different config file or a machine another stack owns, but lets through different names with their own machines. (the judges relocated stack_conflict to src/core/state/stamp/mod.rs:271)
   - evidence: CONFIRMED-AS-NARROWED by the judges (lane 1 for the stack_conflict line, all three for the single-stack family); the substance held under three refuters.

50. [tests-docs] L3-C3 — A 1.0 lock is migrated in memory on load into a 1.1 stacks map at src/core/state/stamp/mod.rs:202, and an older forjar ignores this new field but keeps the single name stamp when re-saving as documented at src/core/types/state_types.rs:26.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

51. [tests-docs] L3-C4 — The test legacy_single_stamp_lock_migrates_on_load at src/core/state/tests_stack_stamp.rs:119 fails if the migration call at src/core/state/mod.rs:108 is removed.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

52. [tests-docs] L3-C5 — The document sentence 'Until stack-scoped restore lands, all three refuse outright whenever the state dir holds more than one stack, regardless of which name's generation was asked for' at CHANGELOG.md:102 is true of the code.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

53. [tests-docs] L3-C6 — A single-stack dir remains completely unchanged—producing no apply warnings, allowing undo, and hiding per-stack status—as evidenced by the test at tests/falsification_state_stamp_per_name.rs:351.
   - corrected: A single-stack dir remains completely unchanged—producing no apply warnings, allowing undo, and hiding per-stack status—as evidenced by the test at tests/falsification_state_stamp_per_name.rs:351. — narrowed by the judges: the observable output, exit code and state dir are unchanged, while the unscoped path now evaluates rollback_on_failure_gate (src/cli/apply.rs:63) and shared_stack_count, which return early.
   - evidence: CONFIRMED-AS-NARROWED by the judges (lane 1 for the stack_conflict line, all three for the single-stack family); the substance held under three refuters.

54. [tests-docs] L3-F1 — src/cli/undo.rs:305 — C1: The multi-stack guard sits at src/cli/undo.rs:305 on the undo path, running right after the read-only dry-run/yes checks and before any destructive operations like destroy_absent_from_target.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

55. [tests-docs] L3-F2 — src/core/state/stamp/mod.rs:323 — C2: The stack_conflict function at src/core/state/stamp/mod.rs:323 refuses the same name applied from a different config file or a machine another stack owns, but lets through different names with their own machines.
   - corrected: src/core/state/stamp/mod.rs:271 — C2: The stack_conflict function at src/core/state/stamp/mod.rs:271 refuses the same name applied from a different config file or a machine another stack owns, but lets through different names with their own machines. (the judges relocated stack_conflict to src/core/state/stamp/mod.rs:271)
   - evidence: CONFIRMED-AS-NARROWED by the judges (lane 1 for the stack_conflict line, all three for the single-stack family); the substance held under three refuters.

56. [tests-docs] L3-F3 — src/core/state/stamp/mod.rs:202 — C3: A 1.0 lock is migrated in memory on load into a 1.1 stacks map at src/core/state/stamp/mod.rs:202, and an older forjar ignores this new field but keeps the single name stamp when re-saving as documented at src/core/types/state_types.rs:26.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

57. [tests-docs] L3-F4 — src/core/state/tests_stack_stamp.rs:119 — C4: The test legacy_single_stamp_lock_migrates_on_load at src/core/state/tests_stack_stamp.rs:119 fails if the migration call at src/core/state/mod.rs:108 is removed.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

58. [tests-docs] L3-F5 — CHANGELOG.md:102 — C5: The document sentence 'Until stack-scoped restore lands, all three refuse outright whenever the state dir holds more than one stack, regardless of which name's generation was asked for' at CHANGELOG.md:102 is true of the code.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean per-lane clone; the mechanism was read at HEAD and matches the sentence as posed.

59. [tests-docs] L3-F6 — tests/falsification_state_stamp_per_name.rs:351 — C6: A single-stack dir remains completely unchanged—producing no apply warnings, allowing undo, and hiding per-stack status—as evidenced by the test at tests/falsification_state_stamp_per_name.rs:351.
   - corrected: tests/falsification_state_stamp_per_name.rs:351 — C6: A single-stack dir remains completely unchanged—producing no apply warnings, allowing undo, and hiding per-stack status—as evidenced by the test at tests/falsification_state_stamp_per_name.rs:351. — narrowed by the judges: the observable output, exit code and state dir are unchanged, while the unscoped path now evaluates rollback_on_failure_gate (src/cli/apply.rs:63) and shared_stack_count, which return early.
   - evidence: CONFIRMED-AS-NARROWED by the judges (lane 1 for the stack_conflict line, all three for the single-stack family); the substance held under three refuters.

## Tables as returned

### Judge 1 (verdict PASS)

L1-C1 | CONFIRMED | The multi-stack guard sits at src/cli/undo.rs:305, and on the undo path `stage_target_config` runs after it while `compute_undo_diff` runs before it.
L1-C2 | CONFIRMED-AS-NARROWED | stack_conflict at src/core/state/stamp/mod.rs:271 refuses the same name from a different config file or a machine another stack owns, but lets through a different name with its own machines.
L1-C3 | CONFIRMED | A 1.0 lock is migrated in memory on load by `state::stamp::migrate`, and an older forjar parsing a 1.1 file drops the `stacks` map when it re-saves while keeping `name`, `machines`, and `outputs` at src/core/types/state_types.rs:21.
L1-C4 | CONFIRMED | The test `a_machine_another_stack_owns_is_a_conflict` in src/core/state/tests_stack_stamp.rs:213 fails if the conflict check at src/core/state/stamp/mod.rs:332 is bypassed.
L1-C5 | CONFIRMED | The help string at src/cli/commands/apply_args.rs:60 stating that rollback refuses outright for multi-stack dirs is true of the code.
L1-C6 | CONFIRMED-AS-NARROWED | Single-stack behavior is unchanged in output, but the path evaluates rollback_on_failure_gate.
L1-C7 | REFUTED | stale: fixed in 00af3c08
L1-F1 | CONFIRMED | src/cli/undo.rs:305 — The multi-stack guard sits at src/cli/undo.rs:305, and on the undo path `stage_target_config` runs after it while `compute_undo_diff` runs before it.
L1-F2 | CONFIRMED-AS-NARROWED | stack_conflict at src/core/state/stamp/mod.rs:271 refuses the same name from a different config file or a machine another stack owns, but lets through a different name with its own machines.
L1-F3 | CONFIRMED | src/core/types/state_types.rs:21 — A 1.0 lock is migrated in memory on load by `state::stamp::migrate`, and an older forjar parsing a 1.1 file drops the `stacks` map when it re-saves while keeping `name`, `machines`, and `outputs` at src/core/types/state_types.rs:21.
L1-F4 | CONFIRMED | src/core/state/tests_stack_stamp.rs:213 — The test `a_machine_another_stack_owns_is_a_conflict` fails if the conflict check at src/core/state/stamp/mod.rs:332 is bypassed.
L1-F5 | CONFIRMED | src/cli/commands/apply_args.rs:60 — The help string at src/cli/commands/apply_args.rs:60 stating that rollback refuses outright for multi-stack dirs is true of the code.
L1-F6 | CONFIRMED-AS-NARROWED | Single-stack behavior is unchanged in output, but the path evaluates rollback_on_failure_gate.
L1-F7 | REFUTED | stale: fixed in 00af3c08
L2-C1 | CONFIRMED | The multi-stack guard `refuse_multi_stack_restore(state_dir, Some(target))` sits at `src/cli/undo.rs:305`, running after checking the `--yes` confirmation and printing the changes diff, but before `stage_target_config` and `destroy_absent_from_target`.
L2-C2 | CONFIRMED-AS-NARROWED | stack_conflict at src/core/state/stamp/mod.rs:271 refuses the same name from a different config file or a machine another stack owns, but lets through a different name with its own machines.
L2-C3 | CONFIRMED | A 1.0 lock on load migrates its stamp to the `stacks` map with no recorded `-f` (`file: None`), and after a 1.1 save, an older forjar ignores and drops the `stacks` map on re-save as documented in `src/core/types/state_types.rs:26`.
L2-C4 | CONFIRMED | Removing the `restore::refuse_multi_stack_restore(state_dir, Some(generation))?;` call at `src/cli/generation/mod.rs:83` makes the test `a_state_dir_holding_several_stacks_refuses_every_restore` fail.
L2-C5 | CONFIRMED | The book sentence 'undo and rollback are not part of that support: both refuse outright while the shared dir holds more than one stack' at `docs/book/src/08-state-management.md:509` is true of the code's refusal implementation.
L2-C6 | CONFIRMED-AS-NARROWED | For a single-stack dir, undo and rollback behave exactly as before without being refused, though the path evaluates rollback_on_failure_gate.
L2-F1 | CONFIRMED | src/cli/undo.rs:305 — The multi-stack guard refuse_multi_stack_restore sits in cmd_undo and runs after printing the diff and checking the --yes flag but before stage_target_config and destroy_absent_from_target.
L2-F2 | CONFIRMED-AS-NARROWED | stack_conflict at src/core/state/stamp/mod.rs:271 refuses the same name from a different config file or a machine another stack owns, but lets through a different name with its own machines.
L2-F3 | CONFIRMED | src/core/types/state_types.rs:26 — A 1.0 lock on load migrates its single stamp into the stacks map with no recorded -f, and after a 1.1 save, an older forjar parses it but drops the stacks map when re-saving.
L2-F4 | CONFIRMED | src/cli/generation/mod.rs:83 — Removing the restore::refuse_multi_stack_restore call makes the a_state_dir_holding_several_stacks_refuses_every_restore test fail.
L2-F5 | CONFIRMED | docs/book/src/08-state-management.md:509 — The document sentence 'undo and rollback are not part of that support: both refuse outright while the shared dir holds more than one stack' is true of the code.
L2-F6 | CONFIRMED-AS-NARROWED | For a single-stack dir, undo and rollback behave exactly as before without being refused, though the path evaluates rollback_on_failure_gate.
L3-C1 | CONFIRMED | The multi-stack guard sits at src/cli/undo.rs:305 on the undo path, running right after the read-only dry-run/yes checks and before any destructive operations like destroy_absent_from_target.
L3-C2 | CONFIRMED-AS-NARROWED | The stack_conflict function at src/core/state/stamp/mod.rs:271 refuses the same name applied from a different config file or a machine another stack owns, but lets through different names with their own machines.
L3-C3 | CONFIRMED | A 1.0 lock is migrated in memory on load into a 1.1 stacks map at src/core/state/stamp/mod.rs:202, and an older forjar ignores this new field but keeps the single name stamp when re-saving as documented at src/core/types/state_types.rs:26.
L3-C4 | CONFIRMED | The test legacy_single_stamp_lock_migrates_on_load at src/core/state/tests_stack_stamp.rs:119 fails if the migration call at src/core/state/mod.rs:108 is removed.
L3-C5 | CONFIRMED | The 

### Judge 2 (verdict PASS)

L1-C1: CONFIRMED
L1-C2: CONFIRMED
L1-C3: CONFIRMED
L1-C4: CONFIRMED
L1-C5: CONFIRMED
L1-C6: CONFIRMED-AS-NARROWED
L1-C7: REFUTED
L1-F1: CONFIRMED
L1-F2: CONFIRMED
L1-F3: CONFIRMED
L1-F4: CONFIRMED
L1-F5: CONFIRMED
L1-F6: CONFIRMED-AS-NARROWED
L1-F7: REFUTED
L2-C1: CONFIRMED
L2-C2: CONFIRMED
L2-C3: CONFIRMED
L2-C4: CONFIRMED
L2-C5: CONFIRMED
L2-C6: CONFIRMED-AS-NARROWED
L2-F1: CONFIRMED
L2-F2: CONFIRMED
L2-F3: CONFIRMED
L2-F4: CONFIRMED
L2-F5: CONFIRMED
L2-F6: CONFIRMED-AS-NARROWED
L3-C1: CONFIRMED
L3-C2: CONFIRMED
L3-C3: CONFIRMED
L3-C4: CONFIRMED
L3-C5: CONFIRMED
L3-C6: CONFIRMED-AS-NARROWED
L3-F1: CONFIRMED
L3-F2: CONFIRMED
L3-F3: CONFIRMED
L3-F4: CONFIRMED
L3-F5: CONFIRMED
L3-F6: CONFIRMED-AS-NARROWED
T1: CONFIRMED
T2: CONFIRMED
T3: REFUTED
T4: REFUTED
T5: CONFIRMED
T6: REFUTED
U1: REFUTED
U2: REFUTED
U3: CONFIRMED-AS-NARROWED
U4: REFUTED
X1: CONFIRMED
X2: CONFIRMED
X3: CONFIRMED
X4: CONFIRMED
X5: CONFIRMED
D1: REFUTED
D2: REFUTED
D3: REFUTED
D4: REFUTED
D5: REFUTED
D6: CONFIRMED-AS-NARROWED
D7: REFUTED
D8: REFUTED
D9: REFUTED
D10: REFUTED
F1: CONFIRMED
F2: CONFIRMED
F3: CONFIRMED
F4: CONFIRMED

Totals:
CONFIRMED: 42
CONFIRMED-AS-NARROWED: 8
REFUTED: 17

### Judge 3 (verdict PASS)

L1-F1: CONFIRMED
L1-F2: CONFIRMED
L1-F3: CONFIRMED
L1-F4: CONFIRMED
L1-F5: CONFIRMED
L1-F6: CONFIRMED-AS-NARROWED
L1-F7: REFUTED
L2-F1: CONFIRMED
L2-F2: CONFIRMED
L2-F3: CONFIRMED
L2-F4: CONFIRMED
L2-F5: CONFIRMED
L2-F6: CONFIRMED-AS-NARROWED
L3-F1: CONFIRMED
L3-F2: CONFIRMED
L3-F3: CONFIRMED
L3-F4: CONFIRMED
L3-F5: CONFIRMED
L3-F6: CONFIRMED-AS-NARROWED
T1: CONFIRMED
T2: CONFIRMED
T3: REFUTED
T4: REFUTED
T5: CONFIRMED
T6: REFUTED
U1: REFUTED
U2: REFUTED
U3: CONFIRMED
U4: REFUTED
X1: CONFIRMED
X2: CONFIRMED
X3: CONFIRMED
X4: CONFIRMED
X5: CONFIRMED
D1: REFUTED
D2: REFUTED
D3: REFUTED
D4: REFUTED
D5: REFUTED
D6: CONFIRMED
D7: REFUTED
D8: REFUTED
D9: REFUTED
D10: REFUTED
F1: CONFIRMED
F2: CONFIRMED
F3: CONFIRMED-AS-NARROWED
F4: CONFIRMED

Totals: 28 CONFIRMED, 4 CONFIRMED-AS-NARROWED, 16 REFUTED

