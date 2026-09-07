# Quorum evidence — PMAT-161 — the claims as put to the refuters

Three claim lanes read `git diff origin/main...HEAD` blind at 41c0eac2 (lenses: state stamp and resolver; CLI paths; tests, contracts and documents), one independent agy /teamwork-preview lane (a 25-minute attempt timed out; the 35-minute retry is what is quoted) and one plan-mode fallback on the same hunting prompt reviewed it separately, and one crux lane surveyed the field. Everything below is reproduced as returned. Line numbers are at 41c0eac2; HEAD is now fa9c0061 after the four fixes the review forced (PMAT-174..177), the roadmap commits and the leftovers commit, so a refuter must re-locate a cited line before calling it wrong. Claim ids: L1-C1..L3-C6 and L*-F* (claim lanes), T1.. (teamwork retry findings), U1.. (plan-mode fallback findings), X1.. (crux rows), D1..D8 (the orchestrator's dispositions), F1..F4 (the orchestrator's measured claims).

## Claim lane 1 (verdict FAIL)

C1: The multi-stack guard sits at src/cli/undo.rs:305, and on the undo path `stage_target_config` runs after it while `compute_undo_diff` runs before it.
C2: `stack_conflict` at src/core/state/stamp/mod.rs:323 refuses the same name from a different config file or a machine another stack owns, but lets through a different name with its own machines.
C3: A 1.0 lock is migrated in memory on load by `state::stamp::migrate`, and an older forjar parsing a 1.1 file drops the `stacks` map when it re-saves while keeping `name`, `machines`, and `outputs` at src/core/types/state_types.rs:21.
C4: The test `a_machine_another_stack_owns_is_a_conflict` in src/core/state/tests_stack_stamp.rs:213 fails if the conflict check at src/core/state/stamp/mod.rs:332 is bypassed.
C5: The help string at src/cli/commands/apply_args.rs:60 stating that rollback refuses outright for multi-stack dirs is true of the code.
C6: Single-stack behavior is unchanged because `multi_stack_restore_refusal` at src/core/state/stamp/mod.rs:161 returns `None` when `names.len() <= 1`.
C7: the diff has defect INV-REFUSAL-IS-BEFORE-THE-FIRST-BYTE at src/cli/apply.rs:317.

Findings:
- L1-F1 [asserted] src/cli/undo.rs:305 — The multi-stack guard sits at src/cli/undo.rs:305, and on the undo path `stage_target_config` runs after it while `compute_undo_diff` runs before it.
- L1-F2 [asserted] src/core/state/stamp/mod.rs:323 — `stack_conflict` at src/core/state/stamp/mod.rs:323 refuses the same name from a different config file or a machine another stack owns, but lets through a different name with its own machines.
- L1-F3 [asserted] src/core/types/state_types.rs:21 — A 1.0 lock is migrated in memory on load by `state::stamp::migrate`, and an older forjar parsing a 1.1 file drops the `stacks` map when it re-saves while keeping `name`, `machines`, and `outputs` at src/core/types/state_types.rs:21.
- L1-F4 [asserted] src/core/state/tests_stack_stamp.rs:213 — The test `a_machine_another_stack_owns_is_a_conflict` fails if the conflict check at src/core/state/stamp/mod.rs:332 is bypassed.
- L1-F5 [asserted] src/cli/commands/apply_args.rs:60 — The help string at src/cli/commands/apply_args.rs:60 stating that rollback refuses outright for multi-stack dirs is true of the code.
- L1-F6 [asserted] src/core/state/stamp/mod.rs:161 — Single-stack behavior is unchanged because `multi_stack_restore_refusal` at src/core/state/stamp/mod.rs:161 returns `None` when `names.len() <= 1`.
- L1-F7 [asserted] src/cli/apply.rs:317 — the diff has defect INV-REFUSAL-IS-BEFORE-THE-FIRST-BYTE (proposed fix: Check refuse_multi_stack_restore at the beginning of cmd_apply_scoped when rollback_on_failure is true, before any resources are destroyed.)

## Claim lane 2 (verdict PASS)

C1: The multi-stack guard `refuse_multi_stack_restore(state_dir, Some(target))` sits at `src/cli/undo.rs:305`, running after checking the `--yes` confirmation and printing the changes diff, but before `stage_target_config` and `destroy_absent_from_target`.
C2: `stack_conflict` at `src/core/state/stamp/mod.rs:323` refuses the same config name from a different `-f` or a machine another stack owns, but lets through a different config name with its own machines.
C3: A 1.0 lock on load migrates its stamp to the `stacks` map with no recorded `-f` (`file: None`), and after a 1.1 save, an older forjar ignores and drops the `stacks` map on re-save as documented in `src/core/types/state_types.rs:26`.
C4: Removing the `restore::refuse_multi_stack_restore(state_dir, Some(generation))?;` call at `src/cli/generation/mod.rs:83` makes the test `a_state_dir_holding_several_stacks_refuses_every_restore` fail.
C5: The book sentence 'undo and rollback are not part of that support: both refuse outright while the shared dir holds more than one stack' at `docs/book/src/08-state-management.md:509` is true of the code's refusal implementation.
C6: For a single-stack dir, `undo` and `rollback` behave exactly as before without being refused, evidenced by `multi_stack_restore_refusal` at `src/core/state/stamp/mod.rs:160` explicitly returning `None` when `names.len() <= 1`.

Findings:
- L2-F1 [cited] src/cli/undo.rs:305 — The multi-stack guard refuse_multi_stack_restore sits in cmd_undo and runs after printing the diff and checking the --yes flag but before stage_target_config and destroy_absent_from_target.
- L2-F2 [cited] src/core/state/stamp/mod.rs:323 — stack_conflict refuses the same config name from a different -f or a machine another stack owns, but lets through a different config name with its own machines.
- L2-F3 [cited] src/core/types/state_types.rs:26 — A 1.0 lock on load migrates its single stamp into the stacks map with no recorded -f, and after a 1.1 save, an older forjar parses it but drops the stacks map when re-saving.
- L2-F4 [asserted] src/cli/generation/mod.rs:83 — Removing the restore::refuse_multi_stack_restore call makes the a_state_dir_holding_several_stacks_refuses_every_restore test fail.
- L2-F5 [cited] docs/book/src/08-state-management.md:509 — The document sentence 'undo and rollback are not part of that support: both refuse outright while the shared dir holds more than one stack' is true of the code.
- L2-F6 [cited] src/core/state/stamp/mod.rs:160 — For a single-stack dir, undo and rollback behave exactly as before without being refused, because multi_stack_restore_refusal returns None.

## Claim lane 3 (verdict PASS)

C1: The multi-stack guard sits at src/cli/undo.rs:305 on the undo path, running right after the read-only dry-run/yes checks and before any destructive operations like destroy_absent_from_target.
C2: The stack_conflict function at src/core/state/stamp/mod.rs:323 refuses the same name applied from a different config file or a machine another stack owns, but lets through different names with their own machines.
C3: A 1.0 lock is migrated in memory on load into a 1.1 stacks map at src/core/state/stamp/mod.rs:202, and an older forjar ignores this new field but keeps the single name stamp when re-saving as documented at src/core/types/state_types.rs:26.
C4: The test legacy_single_stamp_lock_migrates_on_load at src/core/state/tests_stack_stamp.rs:119 fails if the migration call at src/core/state/mod.rs:108 is removed.
C5: The document sentence 'Until stack-scoped restore lands, all three refuse outright whenever the state dir holds more than one stack, regardless of which name's generation was asked for' at CHANGELOG.md:102 is true of the code.
C6: A single-stack dir remains completely unchanged—producing no apply warnings, allowing undo, and hiding per-stack status—as evidenced by the test at tests/falsification_state_stamp_per_name.rs:351.

Findings:
- L3-F1 [asserted] src/cli/undo.rs:305 — C1: The multi-stack guard sits at src/cli/undo.rs:305 on the undo path, running right after the read-only dry-run/yes checks and before any destructive operations like destroy_absent_from_target.
- L3-F2 [cited] src/core/state/stamp/mod.rs:323 — C2: The stack_conflict function at src/core/state/stamp/mod.rs:323 refuses the same name applied from a different config file or a machine another stack owns, but lets through different names with their own machines.
- L3-F3 [asserted] src/core/state/stamp/mod.rs:202 — C3: A 1.0 lock is migrated in memory on load into a 1.1 stacks map at src/core/state/stamp/mod.rs:202, and an older forjar ignores this new field but keeps the single name stamp when re-saving as documented at src/core/types/state_types.rs:26.
- L3-F4 [asserted] src/core/state/tests_stack_stamp.rs:119 — C4: The test legacy_single_stamp_lock_migrates_on_load at src/core/state/tests_stack_stamp.rs:119 fails if the migration call at src/core/state/mod.rs:108 is removed.
- L3-F5 [cited] CHANGELOG.md:102 — C5: The document sentence 'Until stack-scoped restore lands, all three refuse outright whenever the state dir holds more than one stack, regardless of which name's generation was asked for' at CHANGELOG.md:102 is true of the code.
- L3-F6 [cited] tests/falsification_state_stamp_per_name.rs:351 — C6: A single-stack dir remains completely unchanged—producing no apply warnings, allowing undo, and hiding per-stack status—as evidenced by the test at tests/falsification_state_stamp_per_name.rs:351.

## Independent lane — agy /teamwork-preview, 35-minute retry (verdict do-not-implement-as-written)

The branch successfully implements multi-stack state locks, safe 1.0 schema migrations, upfront multi-stack restore refusal, and preserves byte-identical single-stack status output. However, it contains two critical logic flaws that will corrupt state: a path-collapse bug in rename retirement (misidentifying config files due to flawed `ParentDir` stripping), and a machine-dropping defect during scoped applies (failing to union unselected machines and bypassing ownership guards).

A standard automated claim lane would have run existing tests, seen green passes, and issued a false-positive approval. The agent team actively uncovered the path-collapse bug via component stripping analysis, exposed the scoped apply machine dropping via written POC tests, and validated architectural invariants to ensure state safety.

The full deliverable report is available at: `/tmp/lanes-161q1-344044/DOCUMENT_REVIEW_REPORT.md`

Findings:
- T1 [cited] src/cli/undo.rs:305 — `refuse_multi_stack_restore` strictly precedes all staging, pruning, and generation snapshots, preventing cross-stack corruption during restores.
- T2 [cited] src/core/state/stamp/mod.rs:202 — `stamp::migrate` correctly initializes legacy 1.0 schema entries with `file: None` and provides a one-apply grace period without spurious warnings.
- T3 [cited] src/core/state/stamp/mod.rs:262 — CRITICAL: `same_config_file` incorrectly strips `ParentDir` components, causing relative paths like `../forjar.yaml` to collapse into a wildcard matching any relative `*/forjar.yaml`, triggering wrong-file rename retirements.
- T4 [cited] src/core/state/stamp/rename.rs:58 — HIGH-SEVERITY: Scoped apply (`apply --machine`) drops untargeted machines from the state stamp because `next_stamp` does not union `previous.machines`, allowing sibling stacks to bypass the ownership guard.
- T5 [cited] src/cli/status_core.rs:131 — Multi-stack summary blocks are strictly gated by `stacks.len() > 1`, ensuring single-stack directories remain byte-identical to pre-PMAT-161 output.
- T6 [cited] docs/roadmaps/roadmap.yaml:1882 — The code correctly implements the widened criteria (upfront restore refusal), but the roadmap backlog retains stale, contradictory pre-pivot text about per-stack undo.

## Plan-mode fallback on the teamwork prompt (verdict FAIL)

I have reviewed the branch `PMAT-161-state-stamp-per-name`. While the multi-stack refusal logic correctly protects `undo_prune` and `undo --resume`, I found that `--rollback-on-failure` executes the entire apply process (writing to the state dir and destroying resources) before it fails and reaches the refusal in `rollback_to_generation`. Additionally, generation snapshot GC silently deletes snapshots belonging to other stacks in a shared state dir. Older forjars will also silently erase the `stacks` map when saving a `1.1` lock, and `retire_renamed` leaves duplicate renames behind by using `.find()`. The machine ownership guard, migration logic, and status outputs are implemented correctly.

Findings:
- U1 [cited] src/cli/apply.rs:174 — --rollback-on-failure writes to the state dir before the refusal (proposed fix: Check refuse_multi_stack_restore at the beginning of cmd_apply if rollback_on_failure is enabled.)
- U2 [cited] src/cli/apply_snapshot.rs:118 — generation snapshots (gc_old_snapshots) deletes snapshots from other stacks in a shared dir (proposed fix: Scope generation snapshots to the stack name or segregate them by stack in the shared dir.)
- U3 [asserted] src/core/types/state_types.rs:1 — Older forjars will silently erase the stacks map because they lack check_schema and deserialize 1.1 as 1.0 (proposed fix: N/A (Older forjars cannot be updated, but this should be explicitly handled or warned about).)
- U4 [cited] src/core/state/stamp/rename.rs:33 — retire_renamed uses .find() which only retires the first matching entry, leaving duplicates behind (proposed fix: Use .retain() or a loop to remove all matching entries instead of just the first.)

## Crux lane — competitive survey (verdict PASS)

### Competitive Survey: State Management and Ownership

| System | Stack Identity | Wrong Stack Detection | Cross-Stack Machine/Resource Ownership | Rename Operation |
|--------|----------------|-----------------------|----------------------------------------|------------------|
| **forjar** | Name-keyed `stacks:` map in shared `forjar.lock.yaml` | Warns if SAME name applies from DIFFERENT `-f` config file | Prevents cross-stack fighting: warns if a machine is recorded under ANOTHER stack's entry | Natively supported; applying same config under new `name:` retires old name and moves lineage |
| **Terraform** | One backend, many workspaces; `terraform state` lineage ID [X] | State locking prevents concurrency; workspaces isolate state [X] | Two workspaces can import and fight over the same resource (no cross-workspace locks) [X] | `terraform workspace select/new/delete` or `state mv`; no native CLI workspace rename [X] |
| **Pulumi** | Project + Stack name in backend; URNs contain stack name [X] | Backend enforces stack identity; URN mismatch on wrong config [X] | Like TF, multiple stacks can import the same resource and conflict [X] | Natively supported via `pulumi stack rename` (preserves lineage/URNs) [X] |
| **Nix** | Profile symlink to generation (e.g. `nix-env --switch-profile`) [X] | Profile path explicitly selected [X] | Multiple profiles can exist on one machine; conflicts happen at path level [X] | No native rename; simply point a new symlink to a generation [X] |
| **Kubernetes** | Namespaces + ownerReferences [X] | Namespaces isolate resources [X] | `ownerReferences` prevent multiple controllers from adopting the same child resource [X] | Resources/namespaces cannot be natively renamed; must recreate [X] |
| **Ansible** | Stateless; playbooks and inventory define targets [X] | N/A (no state) [X] | Playbooks blindly overwrite each other (last apply wins) [X] | N/A (just rename playbook file) [X] |

**Verdict Paragraph:**
PASS. forjar's rules are at least as sound as the field's. By explicitly checking if a machine is already owned by another stack in the shared state directory, forjar prevents cross-stack resource fighting, providing stronger cross-stack safety than Terraform [X] and Pulumi [X] which allow users to import and fight over the same physical resources. It matches the robustness of Kubernetes [X] `ownerReferences` and improves upon the lack of ownership tracking in Nix [X] and Ansible [X]. Furthermore, forjar natively supports rename-

Findings:
- X1 [asserted] Terraform:? — Terraform identifies stacks via workspaces with isolated state files and lineage IDs. It relies on operators not to import the same resource into multiple workspaces, as it lacks cross-workspace locks on physical resources. CLI workspace renames require manual state moves [X].
- X2 [asserted] Pulumi:? — Pulumi identifies deployments by project and stack name. It natively supports stack renaming (preserving lineage) but, like Terraform, does not inherently prevent two stacks from importing and fighting over the same physical resource [X].
- X3 [asserted] Nix:? — Nix identifies stacks using profile symlinks pointing to specific generations. Multiple profiles can exist on a machine, but renaming is not natively modeled beyond updating symlinks [X].
- X4 [asserted] Kubernetes:? — Kubernetes relies on namespaces for isolation and ownerReferences to explicitly prevent controllers from adopting and fighting over the same child resources. Resources cannot be natively renamed [X].
- X5 [asserted] Ansible:? — Ansible is stateless, operating without a central state file or stack identity. It does not detect wrong stacks or track resource ownership, allowing playbooks to overwrite each other [X].

## Orchestrator dispositions (each is itself a claim to refute)

- D1 — `apply --rollback-on-failure` ran the whole apply and reached the multi-stack refusal only inside `maybe_rollback_generation`, which swallowed it into a warning (L1-C7, U1, the delegate's note): CONFIRMED. FIXED in 00af3c08 (PMAT-174): when `rollback_on_failure` is set the pre-flight calls `refuse_multi_stack_restore` before the drift gate, the SSH sockets and any write, and `maybe_rollback_generation` propagates the refusal as an Err. Test: `rename_cases::rollback_on_failure_is_refused_before_the_apply_writes_anything`.
- D2 — `stamp::same_config_file` skipped ParentDir components and used `ends_with`, so `../forjar.yaml` matched any `*/forjar.yaml` and `retire_renamed` could retire another stack (teamwork, with an executed reproducer): CONFIRMED. FIXED in be1309f5 (PMAT-175): the two stamped paths are compared exactly as `stamped_config_file` produces them; a file-less entry never matches.
- D3 — a scoped `apply -m X` rewrote the stamp's machines from the filtered results, releasing the stack's other machines (teamwork): CONFIRMED. FIXED in be1309f5 (PMAT-176): machines = (previous ∪ written) ∩ declared, the declared set passed from the config at the apply_output call site (`machines.retain(|m| declared.contains(m))`).
- D4 — `gc_old_snapshots` trimmed other stacks' generation snapshots in a shared dir (U2): CONFIRMED. FIXED in 1c6d0d64 (PMAT-177): retention is skipped with a `note:` while the global lock holds more than one stack; single-stack retention unchanged.
- D5 — `retire_renamed` used `.find()` and retired only the first matching entry (U4): CONFIRMED. FIXED in be1309f5: every matching entry is retired (`stamp/rename.rs`, doc comment at line 23).
- D6 — an older forjar re-saving a 1.1 lock drops the `stacks` map (U3 calls it a defect; L1-C3, L2-C3, L3-C3 call it benign): REJECTED as a defect. It is the documented limit (CHANGELOG, the book page): the new reader fails closed on an unknown schema, and downgrading a shared state dir is unsupported. No code change.
- D7 — the roadmap kept stale pre-pivot text about per-stack undo (teamwork): CONFIRMED. FIXED in 22a359f7: the acceptance criterion states the shipped decision.
- D8 — L1-C7 (`INV-REFUSAL-IS-BEFORE-THE-FIRST-BYTE at apply.rs:317`) is D1 stated from the apply side; same fix, same test.
- D9 — the refuters found a second retention path, `gc_generations` (src/cli/generation/mod.rs), that PMAT-177 did not guard: CONFIRMED. FIXED in 6c5d9416 (PMAT-182): one helper, `stamp::retention::skip_note`, serves both sweeps; `gc_generations` removes nothing and prints one note while the lock holds more than one stack.
- D10 — the refuters found `stack_conflict` calling `same_config_file` with no state dir, falling into the suffix match PMAT-175 removed from the rename path: CONFIRMED. FIXED in 507b6103 (PMAT-183): the state dir is a required argument of `stack_conflict`, `machine_owner`, `same_config_file` and `records_config_file`; `unattributed_match` is deleted; both callers pass it.

## Orchestrator's own measured claims

- F1 (updated at 507b6103: falsification_state_stamp_per_name 18 passed; `cargo test --lib` 13482 passed, 0 failed, 4 ignored; fmt and clippy clean) — Acceptance at fa9c0061: tests/falsification_state_stamp_per_name 14 passed, falsification_undo_replay_fidelity 6, falsification_undo_state_dir_interlock 6, falsification_undo_actually_undoes 8; `cargo test --lib -- core::state undo generation state_identity apply_snapshot cov_apply_b` 440 passed; `pv validate contracts/undo-refuses-multi-stack-state-dir-v1.yaml` valid; `cargo fmt --all -- --check` clean; `cargo clippy --all-targets -- -D warnings` clean.
- F2 — Three mutations of the guard at 5044ab1f, each restored afterwards: the multi-stack threshold disabled (`names.len() <= 1` → `<= 99` in src/core/state/stamp/mod.rs) turns 4 of 14 tests red (`rollback_on_failure_is_refused_before_the_apply_writes_anything`, `a_refused_undo_destroys_nothing_on_the_way_to_refusing`, `a_state_dir_holding_several_stacks_refuses_every_restore`, `a_renamed_stack_is_one_lineage_not_two_stacks`); the threshold moved to three stacks (`<= 2`) turns the two-stack arm of `a_renamed_stack_is_one_lineage_not_two_stacks` red; the guard call removed from `cmd_undo` (src/cli/undo.rs:305 commented out) turns `a_refused_undo_destroys_nothing_on_the_way_to_refusing` red. No `stack_conflict` test moved under any mutation (discrimination).
- F3 — A single-stack dir's OBSERVABLE behaviour is unchanged (the refuters narrowed the claim: an unscoped single-stack run now evaluates `rollback_on_failure_gate` and `shared_stack_count`, which return early; its output, exit code and state dir are identical): `a_single_stack_dir_behaves_exactly_as_before` (apply, apply, undo, no warning), the six interlock tests and the eight `undo_actually_undoes` tests pass with intent unchanged; `status` prints its multi-stack block only when `stacks.len() > 1` (src/cli/status_core.rs); `multi_stack_restore_refusal` returns None for one stack (src/core/state/stamp/mod.rs:168).
- F4 — Coverage on origin/main measured with `cargo llvm-cov --locked --ignore-run-fail --summary-only` on a clean clone: lines 96.54% for the forjar crate, 96.41% for the workspace — above the 95 floor before this branch; the branch adds 40+ tests and removes none.
