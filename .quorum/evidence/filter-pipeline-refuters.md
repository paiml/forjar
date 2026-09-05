# Quorum evidence — PMAT-160 — refuter rulings (round 3, clean tree, HEAD 753eb232)

Three refuter lanes (agy 1.1.27, sandboxed, conversations conv-7720465d, conv-021c56e4, conv-64df9db2; 420-525 s each) attacked every claim in `filter-pipeline-claims.md`. Round 1 was discarded: a lane had mutated `src/cli/dispatch_apply_check.rs` in the working tree and two lanes ruled on that dirty tree. Round 2 was discarded: lanes copied the repository with its target/ directory into /tmp, filled the root filesystem, and two lanes died. Round 3 ran with `git status --porcelain` clean before and after and 212 G free. Lane 1's verdict FAIL is the brief's REFUTED-implies-FAIL rule applied to refutations of stale complaints; its ruling table agrees with lanes 2 and 3 on substance.

## Refuter 1 (verdict FAIL)

L1-C1: SURVIVES — tried cycle and undeclared dep, they return errors in build_execution_order before pruning.
L1-C2: SURVIVES — checked cmd_apply_check, it resolves the selection before checking.
L1-C3: SURVIVES — GateScope explicitly sets resource: None and group: None.
L1-C4: SURVIVES — test correctly asserts success, reverting the fix would fail on charlie.
L1-C5: SURVIVES — contracts/flag-has-effect-v1.yaml asserts this and tests it, describing commands that would fail with the fix reverted.
L1-C6: SURVIVES — verbose reporting is byte-identical.
L1-F1: SURVIVES — validation runs first.
L1-F2: SURVIVES — resolves before check.
L1-F3: SURVIVES — GateScope receives None.
L1-F4: SURVIVES — identical id selection test exists.
L1-F5: SURVIVES — contract exists.
L1-F6: SURVIVES — formatter exists.
L2-C1: SURVIVES — build_execution_order validates graph and rejects undeclared dependencies.
L2-C2: SURVIVES — cmd_apply_check processes selectors through resolve_selection.
L2-C3: SURVIVES — GateScope receives None.
L2-C4: SURVIVES — check_under_subset_ignores_the_red_resource_out_of_scope would fail with the fix reverted.
L2-C5: SURVIVES — CHANGELOG matches the code.
L2-C6: SURVIVES — apply passes resource_filter: None to executor. No call site passes -r/-g to the executor, drift gate, confirmation counts, or dry-run plan.
L2-F1: SURVIVES — test asserts correctly.
L2-F2: SURVIVES — gate scope receives None.
L2-F3: SURVIVES — cmd_apply_check integrates resolve_selection.
L2-F4: SURVIVES — dry_run_lists_exactly_the_closure_and_its_summary_agrees test would fail with fix reverted.
L3-C1: SURVIVES — graph is validated first.
L3-C2: SURVIVES — apply --check resolves selection.
L3-C3: SURVIVES — dry-run output renders GateScope with None.
L3-C4: SURVIVES — test passes.
L3-C5: SURVIVES — contract_edges enforces this.
L3-C6: SURVIVES — machine selector remains executor-level.
L3-F1: REFUTED — CHANGELOG at HEAD correctly states parse -> validate the full graph -> filter by graph closure.
L3-F2: REFUTED — roadmap at HEAD was updated to mandate parse -> validate the full graph -> filter by graph closure.
L3-F3: SURVIVES — contract_edges enforces it.
L3-F4: SURVIVES — document assertion is correct.
T1: REFUTED — resolve_selection returns empty_after_narrowing error if keep is empty and dropped is not.
T2: REFUTED — strip_unrequested_phony now uses narrow::contract_edges to preserve dependencies.
T3: SURVIVES — cmd_refresh_only ignores -r (filed as #470).
T4: SURVIVES — --only-machine strands cross-machine dependencies, but this is explicitly designed (D4).
T5: SURVIVES — runs nothing at exit 0 (filed as #471).
T6: SURVIVES — unscoped run bypasses executor resource_filter check since it's None.
T7: SURVIVES — passes resource_filter=None to scoped_action_counts.
T8: REFUTED — the standalone check change is its own paragraph in the CHANGELOG and not buried.
T9: REFUTED — falsification_apply_filter_pipeline.rs contains standalone_check_selects_the_closure_and_refuses_a_typo.
T10: SURVIVES — diff implemented parse -> validate -> filter, and AC was updated.
D1: SURVIVES — empty_after_narrowing is reached only when keep.is_empty() && !dropped.is_empty(), meaning the negatives removed every member of the closure.
D2: SURVIVES — contract_edges produces no self-dependencies, duplicates, or edges to removed nodes (acyclic, deduplicated, and only non-removed nodes are pushed).
D3: SURVIVES — confirmed as filed issue #470.
D4: SURVIVES — confirmed as designed.
D5: SURVIVES — confirmed as filed issue #471.
D6: SURVIVES — CHANGELOG lists standalone check in its own paragraph.
D7: SURVIVES — test drives the check command.
D8: SURVIVES — ticket and CHANGELOG were fixed to state validate-first.
F1: NARROWED — Output is byte-identical, but the code path differs: HEAD evaluates resolve_selection (goal_closure, contract_edges, prune) for unscoped runs, whereas origin/main bypassed graph filtering.
F2: SURVIVES — mutations would turn the tests red.
F3: SURVIVES — 0 vacuous tests.
F4: SURVIVES — fixes are reproduced.

Findings:
- [cited] src/cli/apply_selection/closure.rs:114 — T1
- [cited] src/cli/apply_selection/mod.rs:114 — T2
- [asserted] CHANGELOG.md:62 — T8
- [cited] tests/falsification_apply_filter_pipeline.rs:383 — T9
- [cited] CHANGELOG.md:26 — L3-F1
- [cited] docs/roadmaps/roadmap.yaml:1859 — L3-F2
- [measured] src/cli/apply.rs:91 — F1

## Refuter 2 (verdict PASS)

L1-C1: SURVIVES — tried to find cycle bypass, but validation happens first.
L1-C2: SURVIVES — cmd_apply_check resolves selection explicitly before check.
L1-C3: NARROWED — dry-run behaves as claimed, but finding that refresh-only bypasses filters narrows this claim.
L1-C4: SURVIVES — test would fail if fix reverted.
L1-C5: SURVIVES — contract statement accurately describes new behaviour.
L1-C6: SURVIVES — verbose reporting strings were left byte-identical.
L1-F1: SURVIVES — grounded correctly in closure.rs.
L1-F2: SURVIVES — grounded correctly in dispatch_apply_check.rs.
L1-F3: SURVIVES — grounded correctly in apply.rs.
L1-F4: SURVIVES — grounded correctly in falsification_apply_filter_pipeline.rs.
L1-F5: SURVIVES — grounded correctly in flag-has-effect-v1.yaml.
L1-F6: SURVIVES — grounded correctly in closure.rs.
L2-C1: SURVIVES — validation prevents undeclared dependency from being skipped.
L2-C2: SURVIVES — dispatch_apply_check resolves scoping flags before checking.
L2-C3: SURVIVES — dry-run plan passes None downstream.
L2-C4: SURVIVES — test asserts correctly and fails without fix.
L2-C5: SURVIVES — CHANGELOG text matches code behaviour.
L2-C6: SURVIVES — executor `-r` is bypassed by passing None.
L2-F1: SURVIVES — test validates scope narrowing.
L2-F2: SURVIVES — gate scope passes None.
L2-F3: SURVIVES — cmd_apply_check calls resolve_selection.
L2-F4: SURVIVES — test asserts dry-run lists the closure.
L3-C1: SURVIVES — resolve_selection validates first.
L3-C2: SURVIVES — check parses selectors via selectors_of.
L3-C3: SURVIVES — GateScope uses None for resource/group.
L3-C4: SURVIVES — subset pulling dependency asserts correctly.
L3-C5: SURVIVES — excluded dependencies are contracted, dependents run.
L3-C6: SURVIVES — machine selector stays executor-level.
L3-F1: REFUTED — CHANGELOG updated to "parse -> validate the full graph -> filter by graph closure -> validate the selection" in 3ab88290.
L3-F2: REFUTED — ticket and roadmap updated to the correct order.
L3-F3: SURVIVES — doc comment is enforced by contract_edges.
L3-F4: SURVIVES — book statement matches goal_closure usage.
T1: REFUTED — empty_after_narrowing in closure.rs refuses negative selectors that empty the closure.
T2: REFUTED — strip_unrequested_phony now uses narrow::contract_edges to preserve dependencies.
T3: NARROWED — --plan-file follows its own scope by design, and --refresh-only is a separate filed issue (#470).
T4: SURVIVES — machine narrowing contracts cross-machine edge, confirmed as designed.
T5: SURVIVES — empty state and exit 0 confirmed as pre-existing bug #471.
T6: SURVIVES — executor behaves identically with resource_filter=None.
T7: SURVIVES — scoped_action_counts properly receives None.
T8: SURVIVES — standalone check pulling closure is confirmed and deliberate.
T9: REFUTED — test standalone_check_selects_the_closure_and_refuses_a_typo drives forjar check.
T10: REFUTED — roadmap.yaml AC updated to the correct validate-first order.
D1: SURVIVES — negative that empties the selection is correctly refused.
D2: SURVIVES — unrequested phony stripping preserves ordering.
D3: SURVIVES — refresh-only defect filed as #470.
D4: SURVIVES — cross-machine contraction is designed behaviour.
D5: SURVIVES — conflicting machine filters filed as #471.
D6: SURVIVES — check closure is explicitly documented in its own CHANGELOG paragraph.
D7: SURVIVES — standalone check is driven by test.
D8: SURVIVES — ticket and CHANGELOG fixed in 3ab88290.
F1: NARROWED — output is identical, but unscoped runs now execute resolve_selection and build_execution_order upfront.
F2: SURVIVES — mutations would cause tests to fail.
F3: SURVIVES — vacuous-tests analysis passed.
F4: SURVIVES — fixes to 1.25.2 behaviour reproduce correctly.

Findings:
- [cited] src/cli/apply.rs:113 — L1-C3
- [cited] CHANGELOG.md:21 — L3-F1
- [cited] docs/roadmaps/roadmap.yaml:1859 — L3-F2
- [cited] src/cli/apply_selection/closure.rs:118 — T1
- [cited] src/cli/apply_selection/mod.rs:127 — T2
- [cited] src/cli/dispatch_apply_b.rs:298 — T3
- [cited] tests/falsification_apply_filter_pipeline.rs:383 — T9
- [cited] docs/roadmaps/roadmap.yaml:1859 — T10
- [measured] src/cli/apply.rs:91 — F1

## Refuter 3 (verdict PASS)

L1-C1: SURVIVES — resolve_selection validates the full graph using build_execution_order before pruning, and a cycle produces an error.
L1-C2: SURVIVES — the --check branch resolves all selectors explicitly via resolve_selection.
L1-C3: SURVIVES — the --dry-run branch instantiates GateScope with resource: None and group: None.
L1-C4: SURVIVES — the test would fail because the unpruned check would fail on the red resource charlie.
L1-C5: SURVIVES — the contract is true and tested; --check --subset now exits 0.
L1-C6: SURVIVES — the verbose reporting of added dependencies is byte-identical.
L1-F1: SURVIVES — cited code matches claim.
L1-F2: SURVIVES — cited code matches claim.
L1-F3: SURVIVES — cited code matches claim.
L1-F4: SURVIVES — cited test matches claim.
L1-F5: SURVIVES — cited contract matches claim.
L1-F6: SURVIVES — cited code matches claim.
L2-C1: SURVIVES — explicitly runs build_execution_order.
L2-C2: SURVIVES — cmd_apply_check processes all scoping flags.
L2-C3: SURVIVES — plan_selector filters prune it, and downstream explicitly receives None.
L2-C4: SURVIVES — test correctly asserts scoped check ignores out-of-scope red resource.
L2-C5: SURVIVES — CHANGELOG.md accurately asserts --check honours subset exactly as apply does.
L2-C6: SURVIVES — executor's unscoped behavior left byte-identical, passed None.
L2-F1: SURVIVES — cited test matches claim.
L2-F2: SURVIVES — cited code matches claim.
L2-F3: SURVIVES — cited code matches claim.
L2-F4: SURVIVES — asserted test matches claim.
L3-C1: SURVIVES — first validates full graph before resolving.
L3-C2: SURVIVES — apply --check parses and passes resource-set selectors.
L3-C3: SURVIVES — dry-run output explicitly passes None.
L3-C4: SURVIVES — test subset_pulls_the_dependency_closure_in asserts correctness.
L3-C5: SURVIVES — doc comment is true via contract_edges.
L3-C6: SURVIVES — machine selector remains unchanged.
L3-F1: REFUTED — the CHANGELOG at HEAD now correctly states the order as 'parse -> validate the full graph -> filter'.
L3-F2: REFUTED — the roadmap ticket at HEAD now mandates 'parse -> validate the full graph -> filter'.
L3-F3: SURVIVES — cited code matches claim.
L3-F4: SURVIVES — cited document matches claim.
T1: REFUTED — empty_after_narrowing explicitly refuses negative selections that leave keep empty.
T2: REFUTED — strip_unrequested_phony preserves transitive dependencies by using contract_edges instead of .retain().
T3: SURVIVES — cmd_refresh_only still ignores args.resource (as filed in #470).
T4: SURVIVES — cross-machine dependencies contract as designed.
T5: SURVIVES — --only-machine m1 -m m2 runs nothing at exit 0.
T6: SURVIVES — executor behaves identically for an unscoped run.
T7: SURVIVES — confirmation prompt agrees with the selection.
T8: SURVIVES — standalone check -r x pulls in the closure.
T9: REFUTED — standalone_check_selects_the_closure_and_refuses_a_typo successfully tests the standalone check command.
T10: REFUTED — the ticket Acceptance Criteria has been updated to reflect the new pipeline order.
D1: SURVIVES — the refusal correctly fires only when the negatives remove every member of the closure.
D2: SURVIVES — strip_unrequested_phony calls contract_edges.
D3: SURVIVES — out of scope and filed as #470.
D4: SURVIVES — explicitly designed operator decision.
D5: SURVIVES — pre-existing and filed as #471.
D6: SURVIVES — consequence is explicitly documented in the CHANGELOG.
D7: SURVIVES — standalone check is driven by a new binary test.
D8: SURVIVES — ticket and CHANGELOG were successfully fixed in 3ab88290.
F1: NARROWED — unscoped output is byte-identical, but the internal code path now calls resolver::goal_closure and resolver::build_execution_order within resolve_selection.
F2: SURVIVES — mutations correctly turn suites red.
F3: SURVIVES — pmat analyze yields 0 vacuous tests.
F4: SURVIVES — the three fixes are accurately reproduced.

Findings:
- [cited] CHANGELOG.md:26 — L3-F1
- [cited] docs/roadmaps/roadmap.yaml:1859 — L3-F2
- [cited] src/cli/apply_selection/closure.rs:118 — T1
- [cited] src/cli/apply_selection/mod.rs:127 — T2
- [cited] tests/falsification_apply_filter_pipeline.rs:383 — T9
- [cited] docs/roadmaps/roadmap.yaml:1859 — T10
- [measured] src/cli/apply_selection/closure.rs:93 — F1
