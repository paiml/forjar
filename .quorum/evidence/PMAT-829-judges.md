# Judges — PMAT-829

Round count and heads: see `PMAT-829-lanes.md`.

## CONFIRMED

1. [diff] C1 — the cost limit now counts the changes planned for the selected resources: check_cost_limit prunes the config with resolve_selection before building the execution order and the plan, instead of planning the whole file.
   - evidence: src/cli/dispatch_apply.rs:107 calls resolve_selection(&mut config, sel, false)? before build_execution_order and planner::plan.
2. [diff] C2 — apply_execute hoists scope_of above the cost-limit branch and passes selectors built from the same args the real run uses, so the counted set and the applied set come from one selection.
   - evidence: src/cli/dispatch_apply_b.rs:422 builds sel with selectors_of(args, &scope).with_scope(&scope) and passes it to check_cost_limit.
3. [test] C3 — with -r a and limit 1 the run succeeds and writes only a.txt, while with no selector the old behaviour holds and 3 changes planned, limit is 1 is reported, so the fix cannot be skip the check.
   - evidence: tests/falsification_cost_limit_honors_selection.rs:45 and :58 are the selected and unselected cases; the unit tests at src/cli/tests_cov_apply_dispatch.rs:104 and :120 pass an empty selector and keep their assertions.
## REFUTED

1. [test] R1 — the exclude and skip test asserts only success and no cost-limit error, so it would not catch the apply itself ignoring the flags; it is shallower than the -r test.
   - evidence: tests/falsification_cost_limit_honors_selection.rs:66 is that test; recorded as a known gap, the -r test at :45 checks b.txt is absent.
2. [diff] R2 — no test covers dependency closure, where a selected resource depends on an unselected one, so parity of the counted set with the applied set is shown only for independent resources.
   - evidence: src/cli/dispatch_apply.rs:107 passes false to resolve_selection; recorded as a known gap and a follow-up, not a defect in the shipped code.
