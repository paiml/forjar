# Quorum evidence — PMAT-160 — lane and refuter summaries

Three claim lanes read the diff blind at 3ac1c791 with different lenses, an independent agy /teamwork-preview lane reviewed it separately, one crux lane surveyed the field, three refuters attacked the consolidated dossier (round 3; rounds 1 and 2 were discarded — one for a lane's in-place mutation, one for a full disk), and three judges adjudicated. Verdicts are reproduced as returned; every one of them was a claim the orchestrator re-ran.

## Claim lane 1 (verdict PASS)

"C1: The order inside `resolve_selection` (`src/cli/apply_selection/closure.rs:93`) validates the full graph using `resolver::build_execution_order` before applying any closure or pruning, ensuring that an undeclared dependency produces an error rather than being silently discarded by subset narrowing.
C2: The `--check` branch now resolves all resource selectors by explicitly invoking `resolve_selection` with `--subset`, `--exclude`, `-g`, and `--skip` via `selectors_of(args, &scope)` (`src/cli/dispatch_apply_check.rs:73`) before checking the narrowed config.
C3: The `--dry-run` branch renders the scoped plan without dropping downstream closure dependencies because `apply` instantiates `GateScope` with `resource: None` and `group: None` (`src/cli/apply.rs:113`), turning the downstream `plan_selector` filters into no-ops.
C4: The test `check_dry_run_and_apply_select_the_same_ids` (`tests/f..."

## Claim lane 2 (verdict PASS)

"I have reviewed the diff for PMAT-160 (issues #466, #467, #468) focusing on the TESTS lens. The implementation cleanly unifies the resource selection pipeline into a single `resolve_selection` step that validates, closes over `depends_on`, and prunes before dispatching to the executor, check, or dry-run. The tests robustly validate this pipeline and would fail if their corresponding logic in `apply.rs`, `dispatch_apply_check.rs` or `apply_dry_run.rs` was mutated or omitted.

Here are the six claims:
C1: `resolve_selection` explicitly runs `resolver::build_execution_order(config)?` to validate the full unpruned graph before resolving and closing the selection, ensuring that an undeclared dependency correctly produces an "unknown" error. (src/cli/apply_selection/closure.rs:92)
C2: The `--check` mode is now handled by `cmd_apply_check`, which first processes all scoping flags (including `--..."

## Claim lane 3 (verdict do-not-implement-as-written)

"The design specified in the ticket is flawed, as it mandates 'filter -> validate' (which would reintroduce the 'depends on unknown' bug it seeks to fix) and demands that the machine selector (-m) be resolved before validation (which would break cross-machine dependencies). The code correctly ignores this flawed design, but the CHANGELOG repeats it.

C1: The `resolve_selection` function first validates the full graph (`resolver::build_execution_order`) before resolving positive selectors, closing over dependencies, and dropping negatives, so an undeclared dependency produces a validation error immediately (src/cli/apply_selection/closure.rs:93).
C2: `apply --check` now parses the config and passes all resource-set selectors (`--subset`, `--exclude`, `-g`, `--skip`, `-r`, `-t`) into `resolve_selection` via `selectors_of` before checking the selected config (src/cli/dispatch_apply_check.rs:..."

## Independent lane — agy /teamwork-preview (verdict do-not-implement-as-written)

Ten findings, six cited; every defect among them is fixed in 3fdae0c3 / 3ab88290 or filed (#470, #471); see `filter-pipeline-agy.md`.

## Crux lane (verdict PASS)

Four systems surveyed (Terraform, Ansible, SaltStack, Puppet); see `filter-pipeline-crux.md`.

## Refuter 1 (verdict FAIL)

"L1-C1: SURVIVES — tried cycle and undeclared dep, they return errors in build_execution_order before pruning.
L1-C2: SURVIVES — checked cmd_apply_check, it resolves the selection before checking.
L1-C3: SURVIVES — GateScope explicitly sets resource: None and group: None.
L1-C4: SURVIVES — test correctly asserts success, reverting the fix would fail on charlie.
L1-C5: SURVIVES — contracts/flag-has-effect-v1.yaml asserts this and tests it, describing commands that would fail with the fix reverted.
L1-C6: SURVIVES — verbose reporting is byte-identical.
L1-F1: SURVIVES — validation runs first.
L1-F2: SURVIVES — resolves before check.
L1-F3: SURVIVES — GateScope receives None.
L1-F4: SURVIVES — i..."

## Refuter 2 (verdict PASS)

"L1-C1: SURVIVES — tried to find cycle bypass, but validation happens first.
L1-C2: SURVIVES — cmd_apply_check resolves selection explicitly before check.
L1-C3: NARROWED — dry-run behaves as claimed, but finding that refresh-only bypasses filters narrows this claim.
L1-C4: SURVIVES — test would fail if fix reverted.
L1-C5: SURVIVES — contract statement accurately describes new behaviour.
L1-C6: SURVIVES — verbose reporting strings were left byte-identical.
L1-F1: SURVIVES — grounded correctly in closure.rs.
L1-F2: SURVIVES — grounded correctly in dispatch_apply_check.rs.
L1-F3: SURVIVES — grounded correctly in apply.rs.
L1-F4: SURVIVES — grounded correctly in falsification_apply_filter_pipel..."

## Refuter 3 (verdict PASS)

"L1-C1: SURVIVES — resolve_selection validates the full graph using build_execution_order before pruning, and a cycle produces an error.
L1-C2: SURVIVES — the --check branch resolves all selectors explicitly via resolve_selection.
L1-C3: SURVIVES — the --dry-run branch instantiates GateScope with resource: None and group: None.
L1-C4: SURVIVES — the test would fail because the unpruned check would fail on the red resource charlie.
L1-C5: SURVIVES — the contract is true and tested; --check --subset now exits 0.
L1-C6: SURVIVES — the verbose reporting of added dependencies is byte-identical.
L1-F1: SURVIVES — cited code matches claim.
L1-F2: SURVIVES — cited code matches claim.
L1-F3: SURVIVES ..."

Refuter 1's FAIL is the brief's REFUTED-implies-FAIL rule applied to refutations of stale complaints; the three ruling tables agree on substance (full tables in `filter-pipeline-refuters.md`).
