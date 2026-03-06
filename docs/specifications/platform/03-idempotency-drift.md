# 03: Idempotency and Drift Guarantees

> Verus formal properties, plan-time hash comparison, and drift detection — with honest scope.

**Spec ID**: FJ-2006 | **Parent**: [forjar-platform-spec.md](../forjar-platform-spec.md)

---

## Formal Model (Verus Extension)

Extending `src/core/verus_spec.rs`. The existing proofs verify a simplified model (`ResourceState { desired_hash, current_hash, converged }`), not the real pipeline.

**Important caveat**: The real system has two distinct hash computations:

1. **Plan-time hash** (`hash_desired_state`): BLAKE3 of config struct fields joined by `\0`
2. **Executor hash** (`rl.hash`): Set by resource handlers after apply — typically `hash_desired_state`, but handler-dependent

The Verus proofs are **design-time confidence**, not machine-checked implementation guarantees. Extending them to the real pipeline is the goal of Phase 6.

### Properties

```
Property IDEMPOTENT_APPLY:
    forall config, state.
        let state1 = apply(config, state) in
        let state2 = apply(config, state1) in
        state1 == state2
    // Verified: toy model. Not yet verified against real planner.

Property UNDO_INVERSE (reversible resources only):
    forall config_old, config_new, state.
        forall resource where reversibility(resource) == Reversible.
            let state1 = apply(config_new, state) in
            let state2 = undo(config_old, state1) in
            state2 ≈ apply(config_old, state)
    // EXCLUDED: Task, User, Network, Model, Recipe (Irreversible).

Property DESTROY_BEST_EFFORT:
    forall config, state.
        let state1 = apply(config, state) in
        let (state2, results) = destroy(config, state1) in
        forall resource where results[resource].success.
            resource.state_on(state2) == Absent
    // NOT guaranteed for: tasks with side effects, failed destroy scripts.
    // Known limitation: state lock cleaned even on partial failure.

Property GENERATION_MONOTONIC:
    forall state_dir.
        generations(state_dir) is strictly increasing
    // Global per state_dir, not per-machine.

Property UNDO_DESTROY_BEST_EFFORT:
    forall config, state.
        forall resource where reversibility(resource) == Reversible
                          AND resource has inline content or source.
            let state1 = apply(config, state) in
            let state2 = destroy(config, state1) in
            let state3 = undo_destroy(destroy_log, state2) in
            state3 ≈ state1
    // Best-effort for packages (version float), services, external sources.
```

---

## Plan-Time Idempotency (Actual Implementation)

The planner's check (`planner/mod.rs:determine_present_action`) is **local and I/O-free**:

```
fn determine_present_action(resource_id, resource, machine_name, locks) -> PlanAction:
    // 1. Look up existing lock entry
    rl = locks[machine_name].resources[resource_id]

    // 2. If previously failed or drifted, always re-apply
    if rl.status != Converged:
        return Update

    // 3. Hash desired config fields (BLAKE3 of struct fields joined by \0)
    desired_hash = hash_desired_state(resource)
    //   hashes: type, state, provider, packages, path, content, source,
    //   name, owner, group, mode, fs_type, options, target, version,
    //   image, command, schedule, restart, port, protocol, ...

    // 4. Compare against stored hash from last successful apply
    if rl.hash == desired_hash:
        return NoOp   // Config unchanged — skip (<1ms, no I/O)
    else:
        return Update  // Config changed — needs re-convergence
```

### Two Distinct Systems

| System | When | Compares | I/O |
|--------|------|----------|-----|
| **Planner** | `forjar plan` / `forjar apply` | `hash_desired_state(config)` vs `rl.hash` | None (local) |
| **Drift detection** | `forjar drift` / `--tripwire` | `state_query_script` output vs `rl.details.live_hash` | Transport exec |

The planner is fast and offline. Drift detection requires executing scripts on target machines. They are complementary, not redundant.

### The Dual-Hash Gap

If a resource handler stores `hash_desired_state()` as `rl.hash` (most do), idempotency holds. If a handler stores something else (e.g., a live state query hash), the planner could produce false negatives (unnecessary re-applies). Phase 6 audits all handlers to verify this invariant.

### Hash Stability Across Code Changes

`hash_desired_state` determinism (same input → same hash) is a **runtime** property. Hash **stability** (adding a new field to `Resource` doesn't change existing hashes) is a **cross-version** property. These are different.

The field collection functions (`collect_core_fields`, `collect_phase2_fields`) push fields in a fixed order. The comment says "Field order is stable and must not change." But this is enforced only by convention.

**Required safeguards**:
1. **Golden hash test**: A checked-in test with a fixed `Resource` struct and its expected `hash_desired_state` output. If field order changes, this test fails.
2. **New fields append-only**: New fields MUST be added at the end of `collect_phase2_fields`, never inserted in the middle. Document this in the function's doc comment.
3. **Version-tagged hashing**: If a breaking change is unavoidable, bump a hash version prefix (`blake3v2:...`) so the planner knows old hashes are from a different scheme and forces re-convergence intentionally.

---

## Cascading Failure in DAG Execution

The idempotency model assumes `apply()` either succeeds (status=Converged, hash stored) or fails (status=Failed, hash not updated). But the DAG creates dependencies:

```
fn handle_apply_failure(resource_id, error, dag):
    // Mark this resource as Failed
    rl.status = Failed
    rl.hash = ""  // no valid hash

    // Mark all downstream dependents as Skipped
    for dependent in dag.transitive_dependents(resource_id):
        dependent.status = Skipped
        dependent.skip_reason = format!("dependency {} failed", resource_id)

    // On next apply:
    // 1. Failed resource has status != Converged → re-applied
    // 2. If it succeeds, Skipped dependents have status != Converged → re-applied
    // 3. If it fails again, dependents stay Skipped
    // No infinite retry loop: the planner runs once per `forjar apply` invocation
```

**Key invariant**: A Skipped resource is never executed. The planner only processes resources whose upstream dependencies are all Converged. This prevents cascading damage from a partially-applied upstream resource.

**No infinite retry**: Each `forjar apply` invocation runs the DAG exactly once. If A fails, A is retried on the NEXT `forjar apply`, not in a loop within the same invocation. The user must fix the root cause and re-run.

---

## Implementation

### Phase 6: Verus Implementation Proofs (FJ-2006) -- IMPLEMENTED
- [x] Audit all resource handlers: all go through single path in `resource_ops.rs:36` → `planner::hash_desired_state(resolved)` stored as `rl.hash` at line 62. No handler deviates.
- [x] Extend Verus model to capture dual-hash domain (plan-time vs executor)
- [x] Add property: `forall handler. handler.stored_hash(resource) == hash_desired_state(resource)` — verified by single code path in executor + proptest in `tests_proptest_convergence.rs`
- [x] Document any handlers that deviate and why — none deviate; all use the unified `record_resource_converged()` path
- **Extends**: `src/core/verus_spec.rs`
- **Deliverable**: Verus model covers real hash pipeline
