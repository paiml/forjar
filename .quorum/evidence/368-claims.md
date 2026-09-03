# Quorum evidence — #363 / #368 / #378 (fix/plan-apply-integrity) — adjudicated claims

## CONFIRMED — 6 claims survived refutation

1. [probe] (explains-symptom) `forjar plan --out` hashed a STRIPPED config and `forjar apply --plan-file` hashed the unstripped one, so any config with a phony resource (or a `--target`) produced a saved plan that could never be applied.
   - evidence: at the merge-base `src/cli/plan.rs:65` called `strip_unrequested_phony(&mut config, &[])` before `save_plan_file` took `config_hash`, while `src/cli/apply_from_plan.rs:126` stripped AFTER `check_plan_provenance` had already compared against the unstripped file. `seal_config` now snapshots the config once, right after `resolve_data_sources` and before any narrowing, and both sides hash that snapshot. Pinned by `tests/falsification_plan_file_survives_a_phony_resource.rs` (a config with one phony and one ordinary resource: plan → apply-from-plan must converge; a phony-free control passes before and after).

2. [probe] (explains-symptom) `apply --plan-file` reached the executor with the destructive-change prompt, the state-integrity gate, the event-log gate and the pre-apply hooks all unrun — call-graph shape, not a decision.
   - evidence: at the merge-base `src/cli/apply_from_plan.rs:305-308` ran `check_operator_auth` and nothing else before executing; the interactive path's gates live in `apply_execute`, which the plan-file path never entered. `run_plan_apply_gates` (new module `src/cli/apply_from_plan_gates.rs`) now runs every gate the interactive apply runs, in the same order, on the plan's own machine scope. Pinned by `tests/falsification_plan_file_runs_every_gate.rs::plan_file_apply_refuses_a_tampered_state_sidecar` (the refusal text is the gate's own, asserted by name) and the sibling cases.

3. [probe] (explains-symptom) `apply --force` with one failing resource aborted on `debug_assert!(forced_noop_count <= total_converged)`, and on the released binary (assert compiled out) reported more forced no-ops than resources converged.
   - evidence: `src/cli/apply.rs:144` at the merge-base took `measure_forced_noops` BEFORE the run and `src/cli/apply_summary.rs:69` asserted it against the run's converged count. The candidates are now reconciled against the executor's actual results (`forced_noops_that_ran`), so a resource that failed or was skipped is not counted as a forced no-op. Pinned by `tests/falsification_forced_noop_matches_the_run.rs` (two forced candidates, one fails: summary must say 1, never 2, and must not abort).

4. [probe] (explains-symptom) `--refresh-only` dropped `--operator` and authorised as the default system identity.
   - evidence: `src/cli/apply_variants.rs:274` at the merge-base declared `cmd_refresh_only` without an `operator` parameter, so `check_operator_auth` was called with `None` whatever the operator typed. Fixed in a51650b9; covered by `src/cli/tests_cov_apply_variants2.rs` and `tests/falsification_plan_file_runs_every_gate.rs::refresh_only_cannot_launder_a_tampered_lock`.

5. [design] The falsifiers cannot pass vacuously.
   - evidence: each runs the built binary against a real state directory and asserts BOTH the refusal text by name and the filesystem consequence (the managed file is absent when a gate refused; present when it converged); a binary that exited early on a clap error would fail the text assertion, and one that silently converged would fail the filesystem assertion (the reviewer's reading: `tests/falsification_plan_file_runs_every_gate.rs:130-145`).

6. [design] `forced_noop_candidates` passing an EMPTY `drifted` set to `noop_pairs` is not a regression this branch introduced.
   - evidence: the reviewer flagged it as "reverting GH-208". It is the merge-base's own code — `src/core/executor/mod.rs:270` (`forced_noop_count`) carries the "GH-208 REGRESSION FIX" comment explaining that `apply-summary-distinguishability-v1` defines the count as LOCK-based; this branch only renamed the function and moved the reconciliation to the caller. Confirmed by `git show <merge-base>:src/core/executor/mod.rs`.

## REFUTED — 2 claims killed

1. [design] refuted 1/1 — "Every gate the interactive apply runs also runs on `--plan-file`, or is refused by name."
   - corrected: the live drift probe (`check_pre_apply_drift`) neither runs nor is refused by name on the plan-file path. The omission is deliberate and documented (`src/cli/apply_from_plan_gates.rs`, "KNOWN AND DELIBERATE GAP": the probe WRITES `Drifted` into the lock, and a gate must not mutate state a later gate may refuse); the plan is bound to `config_hash` + `state_hash` instead, which is Terraform's `apply tfplan` shape (state binding, no refresh). A host that drifted under an unchanged state file is therefore converged over silently. Filed as #432 with two fix options; this branch closes #363/#368/#378 as specified and does not claim the drift gate.

2. [probe] refuted 1/1 — "The `debug_assert` abort is fixed by widening the invariant."
   - corrected: the invariant `forced_noop ≤ converged` is the contract (`contracts/apply-summary-distinguishability-v1.yaml`) and stays; what changed is that the count is taken from what RAN, not from what was planned. Widening the assert would have hidden the discrimination failure the contract exists to catch.
