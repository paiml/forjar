# Formal Verification & Provability

Forjar provides formal verification capabilities for safety-critical and high-assurance environments. This chapter covers the verification tools, proof frameworks, and certification paths available.

## Kani Bounded Model Checking

Forjar includes [Kani](https://model-checking.github.io/kani/) proof harnesses for critical algorithms. These verify properties like idempotency, collision resistance, and determinism under bounded symbolic execution.

```rust
// From src/core/kani_proofs.rs
#[kani::proof]
fn proof_blake3_idempotency() {
    let data: [u8; 16] = kani::any();
    let h1 = blake3::hash(&data);
    let h2 = blake3::hash(&data);
    assert_eq!(h1, h2);  // Same input always produces same hash
}
```

Six proof harnesses are included:

| Harness | Property |
|---------|----------|
| `proof_blake3_idempotency` | Hashing is deterministic |
| `proof_blake3_collision_resistance` | Different inputs produce different hashes |
| `proof_converged_state_is_noop` | Re-applying a converged state changes nothing |
| `proof_status_transition_monotonic` | Status transitions are monotonic (never regress) |
| `proof_plan_determinism` | Same input produces same plan |
| `proof_topo_sort_stability` | Topological sort is stable |

Run with: `cargo kani --harness proof_blake3_idempotency`

## TLA+ Execution Specification

The file `docs/specifications/ForjarExecution.tla` provides a complete TLA+ model of the plan-apply protocol.

**Safety properties verified:**
- `SafetyDependencyOrder` — Resources applied only after dependencies converge
- `SafetyNoRegression` — Converged resources never revert to pending

**Liveness properties verified:**
- `LivenessAllConverge` — All resources eventually converge
- `LivenessTermination` — The protocol terminates

**Idempotency property:**
- Re-running apply on a fully converged state produces no changes

## SAT-Based Dependency Resolution

The `planner/sat_deps.rs` module provides a DPLL SAT solver for dependency conflict detection.

```yaml
# If resource A depends on B, and B depends on C,
# but C conflicts with A, the SAT solver detects this:
resources:
  - name: A
    depends_on: [B]
  - name: B
    depends_on: [C]
  - name: C
    conflicts_with: [A]  # Unsatisfiable!
```

The solver converts dependency graphs to CNF (conjunctive normal form) and uses unit propagation to detect conflicts before apply begins.

## Alloy Structural Verification

The `docs/specifications/ForjarDependencyGraph.als` provides an Alloy model verifying structural properties of dependency graphs:

- **No self-loops** — Resources cannot depend on themselves
- **No cycles** — Dependency graphs are always DAGs
- **Unique names** — Resource names are unique within a config
- **Topological ordering** — A valid total order always exists

## MC/DC Coverage Analysis

For DO-178C DAL-A compliance, forjar provides Modified Condition/Decision Coverage analysis via `core/mcdc.rs`.

```bash
# Generate MC/DC test pairs for a 3-condition AND decision
forjar qualify --mcdc --conditions 3 --operator and
```

MC/DC ensures every condition independently affects the decision outcome, required for the highest airborne software assurance level.

## Verus-Verified Reconciliation Loop

The `core/verus_spec.rs` module contains machine-checked proofs (when compiled with the Verus toolchain) that the observe-diff-apply reconciliation loop:

1. **Terminates** — The loop always reaches a fixed point
2. **Converges** — After sufficient iterations, all resources match desired state
3. **Is idempotent** — Applying to an already-converged state is a no-op
4. **Is monotonic** — Each iteration reduces the diff, never increases it

## Proof Obligation Taxonomy

Forjar classifies proof obligations by type (`core/compliance.rs`):

| Type | Description | Example |
|------|-------------|---------|
| Idempotency | Re-apply produces same state | `apply(apply(s)) == apply(s)` |
| Convergence | System reaches desired state | All resources eventually `Converged` |
| Safety | No harmful transitions | Dependencies satisfied before apply |
| Determinism | Same inputs produce same outputs | Plan is deterministic |
| Bounded | Resource consumption is bounded | Memory usage is predictable |

## DO-330 Tool Qualification

For avionics supply chains, `core/do330.rs` generates qualification packages:

```bash
forjar qualify --do330 --tql 5
```

Output includes:
- Requirements traceability matrix (requirement → test case mapping)
- Structural coverage evidence (line, branch, MC/DC)
- Tool qualification level justification (TQL-1 through TQL-5)

## Flight-Grade Execution

The `core/flight_grade.rs` module provides a `no_std`-compatible execution model:

- **No dynamic allocation** — All data in fixed-size arrays on the stack
- **No unbounded loops** — All iterations bounded by `MAX_RESOURCES` (256)
- **No panic paths** — All operations return `Result`
- **Deterministic memory** — Stack usage is compile-time predictable

```bash
# Check if a configuration is flight-grade compliant
forjar qualify --flight-grade -f config.yaml
```

## Ferrocene Certification

Forjar supports the [Ferrocene](https://ferrocene.dev/) safety-certified Rust toolchain for ISO 26262 and DO-178C environments.

```bash
# Generate certification evidence
forjar certify --standard iso26262

# Source compliance check (no unsafe, no forbidden attributes)
forjar certify --check-source src/
```

See `core/ferrocene.rs` for the full certification evidence model including ASIL levels (QM through D) and DAL levels (E through A).

## Reproducible Builds

Binary reproducibility is verified via `core/repro_build.rs`:

```bash
# Check build environment for reproducibility
forjar qualify --repro-check

# Generate reproducibility evidence
forjar qualify --repro-evidence --binary target/release/forjar
```

Required settings for reproducible builds:
- `SOURCE_DATE_EPOCH` set
- `CARGO_INCREMENTAL=0`
- `codegen-units = 1` in `[profile.release]`
- `lto = true`
- `panic = "abort"`
