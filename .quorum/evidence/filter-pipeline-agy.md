# Independent review — agy /teamwork-preview — PMAT-160

An independent stack (agy 1.1.27, `/teamwork-preview`, sandboxed, conversation conv-69ee1fb6, 498 s) reviewed the final diff at 3ac1c791 without seeing the claim lanes' output. A plan-stage teamwork run (conversation conv-f65839de) had grilled the design before phase 1 and returned PASS.

## Verdict (do-not-implement-as-written)

The branch fundamentally reorganizes filtering into a single resolution step, but introduces several subtle logical flaws. I am returning 'do-not-implement-as-written' due to major safety concerns:
1. Negative narrowing reintroduces the empty success bug (e.g. `-r a --skip a` exits 0).
2. Phony pruning (`strip_unrequested_phony`) runs after closure and destructively removes edges via `.retain()`, losing transitive dependencies.
3. `--refresh-only` silently drops `-r` and refreshes the whole lock.
4. Standalone `check -r` checks the dependency closure, an unadvertised breaking change that will fail CI jobs on out-of-target drift.
5. The test suite fails to exercise standalone `check` because the helper hardcodes `apply`.

What I contributed that a claim lane would not:
A claim lane would verify test passes and coverage thresholds, which this PR meets. I conducted a semantic audit of the DAG manipulation, identifying that `strip_unrequested_phony` destroys the closure that `contract_edges` built. I caught the CI-breaking semantic shift in `check -r`, identified that `--refresh-only` drops selectors entirely, and caught that the author actively (and correctly) disobeyed the flawed topological ordering requested in AC2.

## Findings, as returned

- T1 [cited] src/cli/apply_selection/closure.rs:295 — Negative selection narrowing (e.g. -r a --skip a) can leave the 'keep' set empty. `resolve_selection` completes without error, and the downstream apply silently succeeds doing nothing. This reintroduces the exact PMAT-199 "empty success" bug that `reject_empty_selection` was designed to fix. (proposed fix: Check if `keep.is_empty()` after `drop_negatives` and return an error before pruning.)
- T2 [cited] src/cli/apply.rs:94 — `strip_unrequested_phony` is called AFTER `resolve_selection` and unconditionally deletes DAG edges to unrequested phonies using `.retain()`. If a phony was pulled in by a goal closure but not requested directly, dropping it here destroys its transitive dependencies, nullifying the careful edge contraction done earlier. (proposed fix: Ensure phonies are processed before or during edge contraction in `resolve_selection`, rather than destructively mutating the DAG afterwards.)
- T3 [cited] src/cli/dispatch_apply_b.rs:298 — `--refresh-only` and `--plan-file` paths fail to honor the old filters. `cmd_refresh_only` ignores `args.resource` entirely and refreshes everything, completely overriding user intent for targeted refreshes. (proposed fix: Pass `resolve_selection` derived configs into `cmd_refresh_only` and `apply_from_plan`, or explicitly reject resource filters for these modes.)
- T4 [asserted] src/cli/apply_selection/narrow.rs:61 — Machine narrowing (`--only-machine`) drops dependencies located on other machines. `contract_edges` contracts the dependency, causing the dependent resource to run without its prerequisite ever converging, stranding it across the fleet. (proposed fix: Re-evaluate cross-machine dependency stranding or issue a warning when cross-machine edges are contracted.)
- T5 [asserted] src/cli/apply_selection/narrow.rs:? — Using `--only-machine m1 -m m2` causes the resource set to be pruned to `m1`, while the executor connects to `m2`. The executor runs against an empty state and exits 0.
- T6 [cited] src/core/executor/machine_b.rs:316 — For an UNSCOPED run, the executor (e.g., `machine_b.rs`) behaves identically as before. `cfg.resource_filter` is `None` (just as it was for unscoped runs previously) and bypasses the `if let Some(filter)` check.
- T7 [cited] src/cli/apply_preflight.rs:267 — The confirmation prompt and apply summary perfectly agree with the selection. `apply.rs` passes `resource_filter=None` to `scoped_action_counts`, which correctly skips filtering and counts all changes within the already-pruned config.
- T8 [asserted] CHANGELOG.md:49 — Standalone `check -r x` now pulls in the `depends_on` closure. This is unsafe for CI pipelines expecting an isolated check; out-of-target drift on dependencies will now cause targeted checks to fail. This breaking change is buried in CHANGELOG.md as a "deliberate consequence". (proposed fix: Re-evaluate standalone check closure expansion, or prominently flag this as a breaking CI change.)
- T9 [cited] tests/falsification_apply_filter_pipeline.rs:106 — `falsification_apply_filter_pipeline.rs` exercises `apply --check`, but it completely fails to test the standalone `forjar check` command because the `Project::run` test helper hardcodes `forjar apply`. (proposed fix: Extend the test fixture to test `forjar check` alongside `forjar apply --check`.)
- T10 [cited] src/cli/apply_selection/closure.rs:220 — AC2 dictates the pipeline order: "parse -> filter by graph closure -> validate". The diff intentionally implements "parse -> validate -> filter by graph closure" to avoid the unknown-dependency bug (which was caused by filtering before validation). The AC's requested order was logically impossible, but the diff technically fails to deliver it as written. (proposed fix: Update the ticket's Acceptance Criteria to reflect the mathematically necessary order.)

## What it contributed that no claim lane raised, and what became of it

- The emptied-selection hole (`--exclude '*'` converged nothing at exit 0): confirmed by measurement and FIXED in 3fdae0c3 (`closure.rs::empty_after_narrowing`). The lane's own example (`-r a --skip a`) was wrong — the closure survives and `bravo` runs — but the hole underneath was real.
- `strip_unrequested_phony` scrubbing edges instead of contracting them: confirmed, pre-existing, FIXED in 3fdae0c3.
- The binary suite never drove the standalone `check` command the branch changed: confirmed, FIXED in 3fdae0c3 (`standalone_check_selects_the_closure_and_refuses_a_typo`).
- The ticket and CHANGELOG order line said filter-then-validate: confirmed, FIXED in 3ab88290.
- `--refresh-only` ignoring `-r`, and `--only-machine X -m Y` running nothing: confirmed, outside the ticket's three bugs, filed as #470 and #471.
- `--only-machine` contracting a cross-machine edge: confirmed as designed (explicit narrowing; the verbose line names the contracted edge).

The verdict `do-not-implement-as-written` was driven by those findings; every one that was a defect is fixed or filed, and the lane affirmed the two properties that matter most for the unscoped path: `machine_b.rs:316` behaves identically with `resource_filter=None`, and the confirmation counts agree with the selection (`apply_preflight.rs`). The judges later killed its T1, T2, T8, T9 and T10 as stale or wrong and narrowed T3 (`filter-pipeline-judges.md`).
