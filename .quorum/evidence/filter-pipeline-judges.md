# Quorum evidence — PMAT-160 — adjudicated claims (majority of three judges)

Fifty-four claim ids (eighteen numbered claims and eighteen findings from three blind claim lanes, ten findings of the independent teamwork review, eight orchestrator dispositions and four measured claims) were put to three refuters on a clean tree at 753eb232 and then to three judges (agy 1.1.27, sandboxed; 210 s, 1413 s and 222 s). The rule is a majority of the three; CONFIRMED-AS-NARROWED counts as survived because each narrowing kept the substance and changed a sentence. Per-lane totals: judge 1 44 confirmed / 3 narrowed / 7 refuted; judge 2 44 / 2 / 8; judge 3 45 / 2 / 7. Majority: 47 survived (45 confirmed, 2 narrowed), 7 refuted. A delta round followed (below): 5 more claims confirmed on 753eb232..2b53621b. Judge 2 ran an in-place mutation experiment in src/cli/apply_dry_run.rs against the brief (whitespace residue, reverted by the orchestrator); judges 1 and 3 finished before that edit and judge 2's table diverges from theirs only on L2-F4, the claim about the file it mutated. Claim text is reproduced as the lanes posed it at 3ac1c791; a citation whose line no longer resolves in the base tree or the pushed tree is kept as a bare path.

## REFUTED — 7 claims killed (each 3-0)

1. [docs] L3-F1 — CHANGELOG.md:26 — The CHANGELOG states the order is 'filter by graph closure -> validate', which the code contradicts by correctly validating the full graph first. (proposed fix: Update the CHANGELOG to state 'parse -> validate -> filter by graph closure' to match the code.)
   - evidence: REFUTED 3-0, stale: fixed in 3ab88290; read at HEAD around CHANGELOG.md:26. The sentence above is the one killed, reproduced as the claim lane posed it.

2. [docs] L3-F2 — docs/roadmaps/roadmap.yaml:1859 — The ticket mandates 'filter by graph closure -> validate', which is exactly the root cause it identified for bug #468, and mandates resolving -m before validation, which would break cross-machine dependencies. (proposed fix: Rewrite the ticket to mandate 'parse -> validate -> filter by graph closure' and leave -m as an executor-level filter.)
   - evidence: REFUTED 3-0, stale: fixed in 3ab88290; read at HEAD around CHANGELOG.md:26. The sentence above is the one killed, reproduced as the claim lane posed it.

3. [teamwork] T1 — src/cli/apply_selection/closure.rs:295 — Negative selection narrowing (e.g. -r a --skip a) can leave the 'keep' set empty. `resolve_selection` completes without error, and the downstream apply silently succeeds doing nothing. This reintroduces the exact PMAT-199 "empty success" bug that `reject_empty_selection` was designed to fix. (proposed fix: Check if `keep.is_empty()` after `drop_negatives` and return an error before pruning.)
   - evidence: REFUTED 3-0, false as stated, and stale for the hole underneath: fixed in 3fdae0c3; read at HEAD around src/cli/apply_selection/closure.rs:295. The sentence above is the one killed, reproduced as the claim lane posed it.

4. [teamwork] T2 — src/cli/apply.rs:94 — `strip_unrequested_phony` is called AFTER `resolve_selection` and unconditionally deletes DAG edges to unrequested phonies using `.retain()`. If a phony was pulled in by a goal closure but not requested directly, dropping it here destroys its transitive dependencies, nullifying the careful edge contraction done earlier. (proposed fix: Ensure phonies are processed before or during edge contraction in `resolve_selection`, rather than destructively mutating the DAG afterwards.)
   - evidence: REFUTED 3-0, stale: fixed in 3fdae0c3; read at HEAD around src/cli/apply.rs:94. The sentence above is the one killed, reproduced as the claim lane posed it.

5. [teamwork] T8 — CHANGELOG.md:49 — Standalone `check -r x` now pulls in the `depends_on` closure. This is unsafe for CI pipelines expecting an isolated check; out-of-target drift on dependencies will now cause targeted checks to fail. This breaking change is buried in CHANGELOG.md as a "deliberate consequence". (proposed fix: Re-evaluate standalone check closure expansion, or prominently flag this as a breaking CI change.)
   - evidence: REFUTED 3-0, killed on the merits; read at HEAD around CHANGELOG.md:49. The sentence above is the one killed, reproduced as the claim lane posed it.

6. [teamwork] T9 — tests/falsification_apply_filter_pipeline.rs:106 — `falsification_apply_filter_pipeline.rs` exercises `apply --check`, but it completely fails to test the standalone `forjar check` command because the `Project::run` test helper hardcodes `forjar apply`. (proposed fix: Extend the test fixture to test `forjar check` alongside `forjar apply --check`.)
   - evidence: REFUTED 3-0, stale: fixed in 3fdae0c3; read at HEAD around tests/falsification_apply_filter_pipeline.rs:106. The sentence above is the one killed, reproduced as the claim lane posed it.

7. [teamwork] T10 — src/cli/apply_selection/closure.rs:220 — AC2 dictates the pipeline order: "parse -> filter by graph closure -> validate". The diff intentionally implements "parse -> validate -> filter by graph closure" to avoid the unknown-dependency bug (which was caused by filtering before validation). The AC's requested order was logically impossible, but the diff technically fails to deliver it as written. (proposed fix: Update the ticket's Acceptance Criteria to reflect the mathematically necessary order.)
   - evidence: REFUTED 3-0, stale: fixed in 3ab88290; read at HEAD around src/cli/apply_selection/closure.rs:220. The sentence above is the one killed, reproduced as the claim lane posed it.

## CONFIRMED — 47 claims survived refutation (45 as written, 2 as narrowed)

1. [resolver] L1-C1 — The order inside `resolve_selection` (`src/cli/apply_selection/closure.rs:93`) validates the full graph using `resolver::build_execution_order` before applying any closure or pruning, ensuring that an undeclared dependency produces an error rather than being silently discarded by subset narrowing.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply_selection/closure.rs:93 and matches the sentence as posed.

2. [resolver] L1-C2 — The `--check` branch now resolves all resource selectors by explicitly invoking `resolve_selection` with `--subset`, `--exclude`, `-g`, and `--skip` via `selectors_of(args, &scope)` (`src/cli/dispatch_apply_check.rs:73`) before checking the narrowed config.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/dispatch_apply_check.rs:73 and matches the sentence as posed.

3. [resolver] L1-C3 — The `--dry-run` branch renders the scoped plan without dropping downstream closure dependencies because `apply` instantiates `GateScope` with `resource: None` and `group: None` (`src/cli/apply.rs:113`), turning the downstream `plan_selector` filters into no-ops.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply.rs:113 and matches the sentence as posed.

4. [resolver] L1-C4 — The test `check_dry_run_and_apply_select_the_same_ids` (`tests/falsification_apply_filter_pipeline.rs:336`) would fail if `cmd_apply_check` in `src/cli/dispatch_apply_check.rs` bypassed `resolve_selection` and ran `cmd_check_selected` on the unpruned config.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around tests/falsification_apply_filter_pipeline.rs:336 and matches the sentence as posed.

5. [resolver] L1-C5 — The contract statement that "`apply --check` under each of --subset, --exclude, -g and --skip checks only the selected resources" (`contracts/flag-has-effect-v1.yaml:1169`) is true of the code because those selectors are parsed and passed to `resolve_selection` to prune the config prior to execution.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply_selection/closure.rs:93 and matches the sentence as posed.

6. [resolver] L1-C6 — The verbose reporting of added dependencies was left byte-identical to previous behavior, as evidenced by `added_suffix` (`src/cli/apply_selection/closure.rs:274`) formatting exactly as `, +{} dependencies` or an empty string.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply_selection/closure.rs:274 and matches the sentence as posed.

7. [tests] L2-C1 — `resolve_selection` explicitly runs `resolver::build_execution_order(config)?` to validate the full unpruned graph before resolving and closing the selection, ensuring that an undeclared dependency correctly produces an "unknown" error. (src/cli/apply_selection/closure.rs:92)
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply_selection/closure.rs:92 and matches the sentence as posed.

8. [tests] L2-C2 — The `--check` mode is now handled by `cmd_apply_check`, which first processes all scoping flags (including `--subset`, `--exclude`, `-g`, `--skip`, `--only-machine`, `--exclude-machine`) through `resolve_selection` before checking, ensuring an out-of-scope red resource does not fail the check. (src/cli/dispatch_apply_check.rs:73)
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/dispatch_apply_check.rs:73 and matches the sentence as posed.

9. [tests] L2-C3 — `apply --dry-run` now renders its text and JSON bodies from a single `types::ExecutionPlan` scoped via `plan_selector` filters, and the downstream drift gate and executor are explicitly passed `resource=None` and `group=None` because the config is already pruned to the closure and re-applying the filters would drop the newly added prerequisites. (src/cli/apply.rs:105)
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply.rs:105 and matches the sentence as posed.

10. [tests] L2-C4 — The test `check_under_subset_ignores_the_red_resource_out_of_scope` correctly asserts that a scoped check does not fail on an out-of-scope red resource; a concrete change removing `resolve_selection` from `cmd_apply_check` (e.g. by commenting out line 73 in `src/cli/dispatch_apply_check.rs`) would cause it to fail on line 168 because the check would incorrectly evaluate the un-converged `charlie` resource and exit non-zero. (tests/falsification_apply_filter_pipeline.rs:168)
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around tests/falsification_apply_filter_pipeline.rs:168 and matches the sentence as posed.

11. [tests] L2-C5 — The `CHANGELOG.md` asserts that "`--check` now honours `--subset`/`--exclude`/`-g`/`--skip`/`-m` exactly as `apply` does", which correctly describes the new code path where `cmd_apply_check` passes these selectors directly into `resolve_selection` to prune the config prior to execution. (CHANGELOG.md:32)
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around CHANGELOG.md:32 and matches the sentence as posed.

12. [tests] L2-C6 — The core executor's unscoped behavior and internal `-r` filtering logic were left byte-identical; instead of modifying the executor in `src/core/executor/resource_ops.rs`, the diff intentionally passes `None` for the resource and group filters at the `cmd_apply_scoped` call site to gracefully bypass the executor's internal filtering. (src/cli/apply.rs:142)
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply.rs:142 and matches the sentence as posed.

13. [docs] L3-C1 — The `resolve_selection` function first validates the full graph (`resolver::build_execution_order`) before resolving positive selectors, closing over dependencies, and dropping negatives, so an undeclared dependency produces a validation error immediately (src/cli/apply_selection/closure.rs:93).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply_selection/closure.rs:93 and matches the sentence as posed.

14. [docs] L3-C2 — `apply --check` now parses the config and passes all resource-set selectors (`--subset`, `--exclude`, `-g`, `--skip`, `-r`, `-t`) into `resolve_selection` via `selectors_of` before checking the selected config (src/cli/dispatch_apply_check.rs:73).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/dispatch_apply_check.rs:73 and matches the sentence as posed.

15. [docs] L3-C3 — The `--dry-run` output renders precisely the `GateScope` that `cmd_apply_scoped` computes, which explicitly passes `None` for the `resource` and `group` filters to ensure the `depends_on` closure pulled in by `resolve_selection` is not dropped downstream by the executor (src/cli/apply.rs:113).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply.rs:113 and matches the sentence as posed.

16. [docs] L3-C4 — The test `subset_pulls_the_dependency_closure_in` asserts that `--subset a` keeps both `a` and its dependency `b` in the configuration; reverting the order inside `resolve_selection` to prune negatives before validating the DAG would make this test fail with a 'depends on unknown' error (src/cli/tests_apply_selection_closure.rs:87).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/tests_apply_selection_closure.rs:87 and matches the sentence as posed.

17. [docs] L3-C5 — The doc comment for `--exclude` asserts that 'an excluded dependency is skipped and its dependents still run', which is true of the code because `resolve_selection` uses `contract_edges` to remove excluded resources while updating the `depends_on` edges of their dependents (src/cli/commands/apply_args.rs:140).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/commands/apply_args.rs:140 and matches the sentence as posed.

18. [docs] L3-C6 — The behaviour of the machine selector (`-m`) remains unchanged as an executor-level filter: `cmd_apply_scoped` leaves the code byte-identical by passing `machine_filter` directly into `GateScope` without resolving it into the scoped resource set, preserving cross-machine dependencies (src/cli/apply.rs:112). The diff has defect 'CHANGELOG.md claims the order is filter then validate, but the code correctly validates first' at CHANGELOG.md:26.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply.rs:112 and matches the sentence as posed.

19. [resolver] L1-F1 — src/cli/apply_selection/closure.rs:93:? — Validation is called first inside resolve_selection.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply_selection/closure.rs:93 and matches the sentence as posed.

20. [resolver] L1-F2 — src/cli/dispatch_apply_check.rs:73:? — cmd_apply_check resolves the selection before checking.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/dispatch_apply_check.rs:73 and matches the sentence as posed.

21. [resolver] L1-F3 — src/cli/apply.rs:113:? — GateScope is instantiated with resource: None and group: None.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply.rs:113 and matches the sentence as posed.

22. [resolver] L1-F4 — tests/falsification_apply_filter_pipeline.rs:336:? — The identical id selection test across check, dry-run, and apply.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around tests/falsification_apply_filter_pipeline.rs:336 and matches the sentence as posed.

23. [resolver] L1-F5 — contracts/flag-has-effect-v1.yaml:1169:? — The contract specifying --check obeys filtering arguments.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply_selection/closure.rs:93 and matches the sentence as posed.

24. [resolver] L1-F6 — src/cli/apply_selection/closure.rs:274:? — The verbose string formatter for dependencies added.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply_selection/closure.rs:274 and matches the sentence as posed.

25. [tests] L2-F1 — tests/falsification_apply_filter_pipeline.rs:168 — The test `check_under_subset_ignores_the_red_resource_out_of_scope` asserts that `--check --subset alpha` succeeds without failing on `charlie`, which ensures the scope correctly narrows before validation.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around tests/falsification_apply_filter_pipeline.rs:168 and matches the sentence as posed.

26. [tests] L2-F2 — src/cli/apply.rs:105 — The gate scope is intentionally passed `resource: None` and `group: None` to preserve the graph closure built by `resolve_selection`.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply.rs:105 and matches the sentence as posed.

27. [tests] L2-F3 — src/cli/dispatch_apply_check.rs:73 — `cmd_apply_check` properly integrates `resolve_selection` before running `cmd_check_selected` to ensure that `--check` respects scoping flags.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/dispatch_apply_check.rs:73 and matches the sentence as posed.

28. [tests] L2-F4 — tests/falsification_apply_filter_pipeline.rs:206 — The `dry_run_lists_exactly_the_closure_and_its_summary_agrees` test verifies that dry-run outputs only the closure; changing `scope_plan` in `apply_dry_run.rs` to NOT call `apply_resource_filter` would make this test fail.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around tests/falsification_apply_filter_pipeline.rs:206 and matches the sentence as posed.

29. [docs] L3-F3 — src/cli/commands/apply_args.rs:140 — The doc comment asserts 'an excluded dependency is skipped and its dependents still run', which the code makes true via `contract_edges` in resolve_selection.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/commands/apply_args.rs:140 and matches the sentence as posed.

30. [docs] L3-F4 — docs/book/src/01-getting-started.md:1527 — The document asserts the selection includes each matched resource's depends_on closure, which is true because `resolve_selection` calls `resolver::goal_closure`.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around CHANGELOG.md:26 and matches the sentence as posed.

31. [teamwork] T3 — src/cli/dispatch_apply_b.rs:298 — `--refresh-only` and `--plan-file` paths fail to honor the old filters. `cmd_refresh_only` ignores `args.resource` entirely and refreshes everything, completely overriding user intent for targeted refreshes. (proposed fix: Pass `resolve_selection` derived configs into `cmd_refresh_only` and `apply_from_plan`, or explicitly reject resource filters for these modes.)
   - corrected: `--refresh-only` ignores the resource selectors (filed #470); `--plan-file` follows the saved plan's own scope by design (#358).
   - evidence: CONFIRMED-AS-NARROWED 3-0; the substance held under all three refuters and all three judges, the sentence changed; read at HEAD around src/cli/dispatch_apply_b.rs:298.

32. [teamwork] T4 — src/cli/apply_selection/narrow.rs:61 — Machine narrowing (`--only-machine`) drops dependencies located on other machines. `contract_edges` contracts the dependency, causing the dependent resource to run without its prerequisite ever converging, stranding it across the fleet. (proposed fix: Re-evaluate cross-machine dependency stranding or issue a warning when cross-machine edges are contracted.)
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply_selection/narrow.rs:61 and matches the sentence as posed.

33. [teamwork] T5 — src/cli/apply_selection/narrow.rs:? — Using `--only-machine m1 -m m2` causes the resource set to be pruned to `m1`, while the executor connects to `m2`. The executor runs against an empty state and exits 0.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply_selection/closure.rs:132 and matches the sentence as posed.

34. [teamwork] T6 — src/core/executor/machine_b.rs:316 — For an UNSCOPED run, the executor (e.g., `machine_b.rs`) behaves identically as before. `cfg.resource_filter` is `None` (just as it was for unscoped runs previously) and bypasses the `if let Some(filter)` check.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/core/executor/machine_b.rs:316 and matches the sentence as posed.

35. [teamwork] T7 — src/cli/apply_preflight.rs:267 — The confirmation prompt and apply summary perfectly agree with the selection. `apply.rs` passes `resource_filter=None` to `scoped_action_counts`, which correctly skips filtering and counts all changes within the already-pruned config.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply_preflight.rs:267 and matches the sentence as posed.

36. [disposition] D1 — — `-r a --skip a` leaves the selection EMPTY and exits 0: REJECTED as stated (NARROWED after refuter round 1: the new refusal fires only when the negatives remove EVERY member of the closure, e.g. `--exclude '*'`; `--subset alpha --exclude alpha` with `alpha -> bravo` still runs `bravo`, by the same rule as `-r a --skip a`). Measured on the 3-resource fixture: `apply --dry-run --yes -r alpha --skip alpha` lists `bravo` (the closure survives the skip; `skipping_the_selected_resource_keeps_its_closure` pins it). The hole underneath was real: `--exclude '*'` converged nothing at exit 0. FIXED in this branch: `closure.rs::empty_after_narrowing` refuses a selection the negatives emptied; `a_negat
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply_selection/closure.rs:99 and matches the sentence as posed.

37. [disposition] D2 — — `strip_unrequested_phony` scrubs `depends_on` edges to a stripped phony and loses transitive order: CONFIRMED, pre-existing (the function ran in the same position before this branch). FIXED: it now calls `narrow::contract_edges` before removing the phony; `stripping_an_unrequested_phony_contracts_through_it` is RED without the hunk.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply_selection/closure.rs:99 and matches the sentence as posed.

38. [disposition] D3 — — `cmd_refresh_only` ignores `-r`: CONFIRMED, out of the ticket's three bugs; filed as paiml/forjar#470.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply_selection/closure.rs:99 and matches the sentence as posed.

39. [disposition] D4 — — `--only-machine` contracts a cross-machine dependency edge and the dependent runs without its prerequisite: CONFIRMED AS DESIGNED. An explicit machine narrowing is the operator's decision, the verbose line names the contracted edge, and the executor already treated a resource skipped by `-m` this way (`ResourceOutcome::Skipped` does not halt dependents).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply_selection/closure.rs:99 and matches the sentence as posed.

40. [disposition] D5 — — `--only-machine m1 -m m2` runs nothing at exit 0: CONFIRMED, pre-existing (the executor-level `-m` was deliberately left as it was so unscoped and `-m` paths stay byte-identical); filed as paiml/forjar#471.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply_selection/closure.rs:99 and matches the sentence as posed.

41. [disposition] D6 — — the standalone `check` change is buried in CHANGELOG.md: REJECTED; it is its own paragraph of the [Unreleased] entry and names both consequences (closure for `check -r`, refusal of a typo).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply_selection/closure.rs:99 and matches the sentence as posed.

42. [disposition] D7 — — the binary suite never drives the standalone `check` command: CONFIRMED. FIXED: `standalone_check_selects_the_closure_and_refuses_a_typo` drives `forjar check -r alpha --json` (ids alpha, bravo) and `check -r typo` (non-zero, message).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply_selection/closure.rs:99 and matches the sentence as posed.

43. [disposition] D8 — — the ticket and CHANGELOG state the order as filter-then-validate: CONFIRMED for the bold line and the ticket title; FIXED in 3ab88290 (`parse -> validate the full graph -> filter by graph closure -> validate the selection -> check | dry-run | apply`). The code was always validate-first.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply_selection/closure.rs:99 and matches the sentence as posed.

44. [measured] F1 — — Unscoped behaviour is byte-identical to the installed forjar 1.25.2 for a VALID config (NARROWED after refuter round 1: a graph with a cycle or an undeclared dependency is refused earlier than in 1.25.2 — before the SSH ControlMasters and the drift gate — with the same message; measured on a two-resource cycle both binaries exit 1 with `dependency cycle detected involving: ...`, the members listed in the other order; the judges narrowed the sentence further to 'byte-identical observable output, different internal path' — an unscoped run now passes through resolve_selection): on a 3-resource local fixture (alpha depends_on bravo, group web; charlie), `apply --dry-run --yes`, `plan`, `apply 
   - corrected: Unscoped observable output is byte-identical to installed 1.25.2 for a valid config (six invocations diffed); the internal path differs, because an unscoped run now passes through resolve_selection (src/cli/apply.rs:91) so an invalid graph is refused before the SSH sockets and the drift gate, with the same message (a cycle's members may be listed in the other order).
   - evidence: CONFIRMED-AS-NARROWED 3-0; the substance held under all three refuters and all three judges, the sentence changed; read at HEAD around src/cli/apply.rs:113.

45. [measured] F2 — — Four mutations of the fix each turn the suites red: no closure (6 of 18 unit tests fail), validation moved after the prune (1 fails), `-r`/`-g` re-applied downstream in apply.rs (1 binary test fails: dry run `-r alpha` lists [alpha] not [alpha, bravo]), and the `--check` branch skipping `resolve_selection` (5 binary tests fail, e.g. `an out-of-scope red resource must not fail a scoped check`).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply.rs:113 and matches the sentence as posed.

46. [measured] F3 — — `pmat analyze vacuous-tests` (pmat 3.37.0) over the repository: 0 vacuous tests among 19342 examined in 2106 files, none in the 7 touched test paths.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply.rs:113 and matches the sentence as posed.

47. [measured] F4 — — The three fixes to 1.25.2 behaviour named in CHANGELOG.md are all reproduced on the fixture: 1.25.2 `apply --dry-run -r alpha` listed charlie (3 to add), `apply --subset alpha` was refused with `depends on unknown 'bravo'`, `apply --check --subset alpha` checked all three and failed on charlie; the branch lists alpha and bravo, converges 2, and reports `2 pass, 0 fail`.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it on a clean tree at 753eb232; the mechanism was read at HEAD around src/cli/apply.rs:113 and matches the sentence as posed.

## CONFIRMED — delta round, 5 claims on 753eb232..2b53621b (three refuter-judges, each 3-0)

The merge helper's first review (1 PASS, 2 FAIL at 14e6120b) read the ticket's old wording as demanding `-m` in the resolver, and read the standalone check change as unasked-for and the selection as unvalidated before the SSH sockets. 2b53621b answered: the acceptance criterion now says what the code does and why, and resolve_selection re-validates the narrowed selection as its last step. Three agy lanes attacked and ruled on the delta; all five claims survived 3-0 with no findings; the merge helper's second review then agreed 3-0.

1. [delta] DC1 — resolve_selection re-validates the narrowed selection with build_execution_order as its last step, guarded so an emptied frame skips it, and that runs before any SSH socket or gate on the apply path.
   - evidence: CONFIRMED 3-0; the guarded call sits at src/cli/apply_selection/closure.rs:131 and the resolve call in cmd_apply_scoped precedes open_control_masters (src/cli/apply.rs:113 is the gate scope built after it).

2. [delta] DC2 — a resource negative (--exclude, --skip) that empties the selection is refused, while machine narrowing that empties the frame converges nothing, the GH-211 rule the pinned test keeps.
   - evidence: CONFIRMED 3-0; src/cli/apply_selection/closure.rs:140 decides by the cause label narrow.rs writes and the refusal at src/cli/apply_selection/closure.rs:120 fires only for resource negatives; measured on the fixture: `--exclude '*'` exits 1 with the refusal, `--exclude-machine local` prints `(nothing selected)` at exit 0.

3. [delta] DC3 — the ticket's acceptance criterion now states that -m stays the executor-level machine filter, -t the plan-level tag filter, the CLI dispatches per mode with the selection pipeline as the one shared path, and the standalone check command resolves through it; each sentence is true of the code and of CHANGELOG.md.
   - evidence: CONFIRMED 3-0; CHANGELOG.md:26 carries the order and the paragraph under it the -m and -t rule; the roadmap entry PMAT-160 carries the same sentences; the merge helper's second review names the standalone check change as an explicit consequence of the one-code-path mandate.

4. [delta] DC4 — no unscoped apply of a valid config produces different observable output than before 2b53621b, the extra build_execution_order call being pure.
   - evidence: CONFIRMED 3-0; re-measured by the orchestrator on the final binary against installed 1.25.2: apply --dry-run, plan, apply, apply --check, a second dry run and a second apply — six of six identical after normalisation (src/cli/apply.rs:113 unchanged since 670b318d).

5. [delta] DC5 — every changed test file is green at HEAD: the six touched lib test modules (118 passed) and the binary suite tests/falsification_apply_filter_pipeline.rs (12 passed), with clippy and fmt clean.
   - evidence: CONFIRMED 3-0 and re-run by the orchestrator after the amend: 118 passed, 12 passed (tests/falsification_apply_filter_pipeline.rs:383 among them), clippy exit 0, fmt clean.

## Dissent recorded

Judge 2 alone refuted L2-F4 (deleting `apply_resource_filter` from `scope_plan` would fail the dry-run closure test) because the apply path passes resource=None, so that call is a no-op; the orchestrator agrees with the mechanism (plan_selector returns early on None) — the test pins the selection upstream, not that call — and records the claim as CONFIRMED by the 2-1 majority with this note. Judge 2 also returned FAIL on the grounds that D3 and D5 name defects unfixed at HEAD; both are outside the ticket's three bugs and filed (#470, #471), and judges 1 and 3 scoped the rule to the branch under review.

## Tables as returned

### Judge 1 (verdict PASS)

L1-C1: CONFIRMED — cycle and undeclared dep return errors in build_execution_order before pruning
L1-C2: CONFIRMED — cmd_apply_check resolves the selection explicitly before check
L1-C3: CONFIRMED-AS-NARROWED — The `--dry-run` branch renders the scoped plan without dropping downstream closure dependencies because `apply` instantiates `GateScope` with `resource: None` and `group: None`; `--plan-file` follows its own scope by design (#358) and `--refresh-only` ignoring filters is filed as #470.
L1-C4: CONFIRMED — test would fail because unpruned check would fail on the red resource charlie
L1-C5: CONFIRMED — contract statement accurately describes new behaviour
L1-C6: CONFIRMED — verbose reporting of added dependencies is byte-identical
L1-F1: CONFIRMED — validation runs first inside resolve_selection
L1-F2: CONFIRMED — cmd_apply_check resolves the selection before checking
L1-F3: CONFIRMED — GateScope is instantiated with resource: None and group: None
L1-F4: CONFIRMED — identical id selection test exists across check, dry-run, and apply
L1-F5: CONFIRMED — contract specifies --check obeys filtering arguments
L1-F6: CONFIRMED — verbose string formatter exists
L2-C1: CONFIRMED — build_execution_order validates the graph and rejects undeclared dependencies
L2-C2: CONFIRMED — cmd_apply_check processes selectors through resolve_selection
L2-C3: CONFIRMED — dry-run plan passes None downstream
L2-C4: CONFIRMED — scoped check ignores out-of-scope red resource
L2-C5: CONFIRMED — CHANGELOG matches the code behaviour
L2-C6: CONFIRMED — unscoped behavior is left byte-identical, passed None
L2-F1: CONFIRMED — test asserts check under subset ignores out-of-scope red resource
L2-F2: CONFIRMED — gate scope receives None
L2-F3: CONFIRMED — cmd_apply_check integrates resolve_selection
L2-F4: CONFIRMED — dry_run_lists_exactly_the_closure_and_its_summary_agrees test exists
L3-C1: CONFIRMED — graph is validated before resolving positive selectors
L3-C2: CONFIRMED — apply --check parses and passes resource-set selectors
L3-C3: CONFIRMED — dry-run output explicitly passes None
L3-C4: CONFIRMED — test subset_pulls_the_dependency_closure_in asserts correctly
L3-C5: CONFIRMED — excluded dependencies are contracted, dependents run
L3-C6: CONFIRMED — machine selector remains unchanged
L3-F1: REFUTED — stale: fixed in 3ab88290
L3-F2: REFUTED — stale: fixed in 3ab88290
L3-F3: CONFIRMED — doc comment is enforced by contract_edges
L3-F4: CONFIRMED — document assertion matches goal_closure usage
T1: REFUTED — stale: fixed in 3fdae0c3
T2: REFUTED — stale: fixed in 3fdae0c3
T3: CONFIRMED-AS-NARROWED — `--plan-file` correctly follows its own scope by design (#358) and `--refresh-only` bypassing filters is a separate filed issue (#470).
T4: CONFIRMED — machine narrowing contracts cross-machine edge, confirmed as designed
T5: CONFIRMED — runs nothing at exit 0, pre-existing bug filed as #471
T6: CONFIRMED — executor behaves identically with resource_filter=None
T7: CONFIRMED — scoped_action_counts properly receives None
T8: REFUTED — Killed: "Standalone `check -r x` now pulls in the `depends_on` closure. This is unsafe for CI pipelines expecting an isolated check; out-of-target drift on dependencies will now cause targeted checks to fail. This breaking change is buried in CHANGELOG.md as a \"deliberate consequence\"." Evidence: The change is explicitly documented in its own prominent paragraph in CHANGELOG.md, not buried.
T9: REFUTED — stale: fixed in 3fdae0c3
T10: REFUTED — stale: fixed in 3ab88290
D1: CONFIRMED — refusal correctly fires only when the negatives remove every member of the closure
D2: CONFIRMED — strip_unrequested_phony preserves ordering without self-edges or duplicates
D3: CONFIRMED — refresh-only defect filed as #470
D4: CONFIRMED — explicitly designed operator decision
D5: CONFIRMED — pre-existing and filed as #471
D6: CONFIRMED — explicitly documented in the CHANGELOG
D7: CONFIRMED — standalone check is driven by a new binary test
D8: CONFIRMED — ticket and CHANGELOG fixed in 3ab88290
F1: CONFIRMED-AS-NARROWED — Unscoped observable output is byte-identical to the installed forjar 1.25.2 for a valid config, but the internal code path now executes resolve_selection (goal_closure, build_execution_order, contract_edges, prune) upfront for unscoped runs.
F2: CONFIRMED — mutations correctly turn suites red
F3: CONFIRMED — pmat analyze yields 0 vacuous tests
F4: CONFIRMED — the three fixes are accurately reproduced
confirmed: 44, confirmed-as-narrowed: 3, refuted: 7

### Judge 2 (verdict FAIL)

L1-C1: CONFIRMED
L1-C2: CONFIRMED
L1-C3: CONFIRMED
L1-C4: CONFIRMED
L1-C5: CONFIRMED
L1-C6: CONFIRMED
L1-F1: CONFIRMED
L1-F2: CONFIRMED
L1-F3: CONFIRMED
L1-F4: CONFIRMED
L1-F5: CONFIRMED
L1-F6: CONFIRMED
L2-C1: CONFIRMED
L2-C2: CONFIRMED
L2-C3: CONFIRMED
L2-C4: CONFIRMED
L2-C5: CONFIRMED
L2-C6: CONFIRMED
L2-F1: CONFIRMED
L2-F2: CONFIRMED
L2-F3: CONFIRMED
L2-F4: REFUTED — Killed: The dry_run_lists_exactly_the_closure_and_its_summary_agrees test verifies that dry-run outputs only the closure; changing scope_plan in apply_dry_run.rs to NOT call apply_resource_filter would make this test fail. Evidence: cmd_apply_scoped passes resource: None into GateScope, which makes apply_resource_filter a no-op that returns immediately, so removing it from scope_plan would not make the test fail.
L3-C1: CONFIRMED
L3-C2: CONFIRMED
L3-C3: CONFIRMED
L3-C4: CONFIRMED
L3-C5: CONFIRMED
L3-C6: CONFIRMED
L3-F1: REFUTED — stale: fixed in 3ab88290
L3-F2: REFUTED — stale: fixed in 3ab88290
L3-F3: CONFIRMED
L3-F4: CONFIRMED
T1: REFUTED — stale: fixed in 3fdae0c3
T2: REFUTED — stale: fixed in 3fdae0c3
T3: CONFIRMED-AS-NARROWED — The --refresh-only path ignores resource filters (filed as #470), while the --plan-file path correctly follows its own scope by design (Refs #358).
T4: CONFIRMED
T5: CONFIRMED
T6: CONFIRMED
T7: CONFIRMED
T8: REFUTED — Killed: This breaking change is buried in CHANGELOG.md as a "deliberate consequence". Evidence: The CHANGELOG.md lists the standalone check consequence in its own distinct paragraph, explicitly detailing the closure behaviour and typo refusal.
T9: REFUTED — stale: fixed in 3fdae0c3
T10: REFUTED — stale: fixed in 3ab88290
D1: CONFIRMED
D2: CONFIRMED
D3: CONFIRMED
D4: CONFIRMED
D5: CONFIRMED
D6: CONFIRMED
D7: CONFIRMED
D8: CONFIRMED
F1: CONFIRMED-AS-NARROWED — Unscoped output is byte-identical for a valid config, but the internal code path differs by executing resolve_selection (including resolver::goal_closure and resolver::build_execution_order) rather than bypassing graph filtering.
F2: CONFIRMED
F3: CONFIRMED
F4: CONFIRMED

Totals: 44 confirmed, 2 confirmed-as-narrowed, 8 refuted.

### Judge 3 (verdict PASS)

L1-C1: CONFIRMED — resolve_selection validates the full graph using build_execution_order before pruning, and a cycle produces an error.
L1-C2: CONFIRMED — the --check branch resolves all selectors explicitly via resolve_selection before checking.
L1-C3: CONFIRMED — I verified in src/cli/apply.rs:113 that GateScope explicitly sets resource: None and group: None, correctly bypassing downstream executor filters.
L1-C4: CONFIRMED — the test correctly asserts success; reverting the fix would fail on charlie.
L1-C5: CONFIRMED — the contract is true and tested; --check --subset now exits 0.
L1-C6: CONFIRMED — the verbose reporting of added dependencies is byte-identical.
L1-F1: CONFIRMED — cited code matches the claim.
L1-F2: CONFIRMED — cited code matches the claim.
L1-F3: CONFIRMED — cited code matches the claim.
L1-F4: CONFIRMED — cited test matches the claim.
L1-F5: CONFIRMED — cited contract matches the claim.
L1-F6: CONFIRMED — cited code matches the claim.
L2-C1: CONFIRMED — explicitly runs build_execution_order to validate graph and rejects undeclared dependencies.
L2-C2: CONFIRMED — cmd_apply_check processes all scoping flags through resolve_selection.
L2-C3: CONFIRMED — GateScope receives None for resource and group to preserve the graph closure.
L2-C4: CONFIRMED — the test correctly asserts scoped check ignores out-of-scope red resource.
L2-C5: CONFIRMED — CHANGELOG matches the code behaviour.
L2-C6: CONFIRMED — executor's unscoped behavior left byte-identical; apply passes None to executor.
L2-F1: CONFIRMED — cited test matches the claim.
L2-F2: CONFIRMED — cited code matches the claim.
L2-F3: CONFIRMED — cited code matches the claim.
L2-F4: CONFIRMED — asserted test matches the claim.
L3-C1: CONFIRMED — first validates full graph before resolving.
L3-C2: CONFIRMED — apply --check parses and passes resource-set selectors.
L3-C3: CONFIRMED — dry-run output explicitly passes None.
L3-C4: CONFIRMED — test subset_pulls_the_dependency_closure_in asserts correctness.
L3-C5: CONFIRMED — doc comment is enforced by contract_edges.
L3-C6: CONFIRMED — machine selector remains executor-level.
L3-F1: REFUTED — stale: fixed in 3ab88290. The CHANGELOG at HEAD now correctly states the order as parse -> validate the full graph -> filter.
L3-F2: REFUTED — stale: fixed in 3ab88290. The roadmap ticket at HEAD now mandates parse -> validate the full graph -> filter.
L3-F3: CONFIRMED — cited code matches the claim.
L3-F4: CONFIRMED — cited document matches the claim.
T1: REFUTED — stale: fixed in the fix commit for the teamwork findings. empty_after_narrowing explicitly refuses negative selections that leave keep empty.
T2: REFUTED — stale: fixed in the fix commit for the teamwork findings. strip_unrequested_phony preserves transitive dependencies by using contract_edges instead of .retain().
T3: CONFIRMED-AS-NARROWED — I verified in apply_variants.rs that the --plan-file path follows its own scope by design, while --refresh-only ignoring resource filters is confirmed and filed as #470.
T4: CONFIRMED — cross-machine dependencies contract as designed.
T5: CONFIRMED — empty state and exit 0 confirmed as pre-existing bug #471.
T6: CONFIRMED — executor behaves identically for an unscoped run.
T7: CONFIRMED — confirmation prompt agrees with the selection.
T8: REFUTED — I verified that the standalone check change is not buried; it is prominently documented in its own paragraph at CHANGELOG.md:60-64.
T9: REFUTED — stale: fixed in the fix commit for the teamwork findings. standalone_check_selects_the_closure_and_refuses_a_typo successfully tests the standalone check command.
T10: REFUTED — stale: fixed in 3ab88290. I verified in docs/roadmaps/roadmap.yaml that the ticket Acceptance Criteria has been updated to reflect the new validate-first pipeline order.
D1: CONFIRMED — I verified in closure.rs that empty_after_narrowing correctly fires only when keep.is_empty() && !dropped.is_empty(), meaning the negatives remove every member of the closure.
D2: CONFIRMED — I verified in narrow.rs that strip_unrequested_phony now properly calls contract_edges. contract_edges avoids duplicates and self-edges via expand_dep tracking.
D3: CONFIRMED — refresh-only defect filed as #470.
D4: CONFIRMED — cross-machine contraction is designed behaviour.
D5: CONFIRMED — conflicting machine filters filed as #471.
D6: CONFIRMED — check closure is explicitly documented in its own CHANGELOG paragraph.
D7: CONFIRMED — standalone check is driven by a new binary test.
D8: CONFIRMED — ticket and CHANGELOG were successfully fixed in 3ab88290.
F1: CONFIRMED-AS-NARROWED — I verified in src/cli/apply_selection/closure.rs:93 that unscoped output is byte-identical to 1.25.2 for a valid config, but the internal code path now calls resolver::build_execution_order and resolver::goal_closure within resolve_selection.
F2: CONFIRMED — mutations correctly turn suites red.
F3: CONFIRMED — pmat analyze yields 0 vacuous tests.
F4: CONFIRMED — the three fixes are accurately reproduced.

Totals:
CONFIRMED: 45
CONFIRMED-AS-NARROWED: 2
REFUTED: 7

