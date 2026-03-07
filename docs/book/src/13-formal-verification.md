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

21 proof harnesses across two modules:

**Abstract model proofs** (`src/core/kani_proofs.rs`):

| Harness | Property |
|---------|----------|
| `proof_blake3_idempotency` | Hashing is deterministic |
| `proof_blake3_collision_resistance` | Different inputs produce different hashes |
| `proof_converged_state_is_noop` | Re-applying a converged state changes nothing |
| `proof_status_transition_monotonic` | Status transitions are monotonic (never regress) |
| `proof_plan_determinism` | Same input produces same plan |
| `proof_topo_sort_stability` | Topological sort is stable |

**Production function proofs** (`src/core/kani_production_proofs.rs`):

| Harness | Function Under Test |
|---------|---------------------|
| `proof_mutation_grade_monotonic` | `MutationScore::grade()` |
| `proof_applicable_operators_valid` | `applicable_operators()` |
| `proof_contract_tier_ordering` | `VerificationTier::Ord` |
| `proof_lock_roundtrip` | `LockEntry` serialization |
| `proof_purity_level_lattice` | `PurityLevel` ordering |

Production function proofs call **real production code** under bounded
symbolic inputs, not abstract models. This catches actual implementation
bugs rather than proving properties of a separate model.

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

## Runtime Contracts on Production Code

The most practical verification layer: `debug_assert!` postconditions
on 9 critical-path functions. These fire in every `cargo test` run
and debug build, catching invariant violations with zero release cost.

| Function | Module | Postcondition |
|----------|--------|---------------|
| `determine_present_action` | `planner/mod.rs` | Converged + hash match implies NoOp |
| `hash_desired_state` | `planner/mod.rs` | Determinism (double-hash equality) |
| `save_lock` | `core/state/mod.rs` | File exists, temp file removed |
| `build_execution_order` | `core/resolver/dag.rs` | Valid topological order |
| `build_layer` | `store/layer_builder.rs` | Same inputs produce same BLAKE3; store idempotency |
| `assemble_image` | `store/image_assembler.rs` | OCI layout files exist, layer count matches |
| `compute_dual_digest` | `store/layer_builder.rs` | Size matches, digests non-empty |
| `write_oci_layout` | `store/layer_builder.rs` | oci-layout and config blob exist |
| `OciManifest::new` | `types/oci_types.rs` | Media types match OCI spec strings |

These contracts connect theoretical proofs to actual code. The Verus
model proves "if handler invariant holds, then idempotency holds." The
`debug_assert!` on `determine_present_action` catches violations of
that invariant in every test run.

## Verification Tier Model (FJ-2203)

Forjar tracks verification maturity across six tiers per critical-path function:

| Tier | Label | What It Proves |
|------|-------|---------------|
| L0 | Unlabeled | No contract annotation |
| L1 | Labeled | `#[contract]` macro present, no verification |
| L2 | Runtime | `#[ensures]` / `debug_assert!` active |
| L3 | Bounded | Kani harness covers this function |
| L4 | Proved | Verus spec with proof block (machine-checked) |
| L5 | Structural | Trait + executor enforcement (unbreakable) |

```rust
use forjar::core::types::{ContractCoverageReport, VerificationTier};

// Query coverage at a tier
let report = ContractCoverageReport::default();
let bounded_or_above = report.at_or_above(VerificationTier::Bounded);
```

Handler invariants are tracked per resource type. Exempt handlers (like `task` — imperative by nature) are documented with justification.

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

## Dual-Hash PlannerState Model

The real planner uses two hashes: a plan-time `hash_desired_state` and an executor-time stored hash. The **handler invariant** bridges them:

> ∀ r: handler(r).stored_hash == hash_desired_state(r)

The `PlannerState` model in `verus_spec.rs` captures this dual-hash domain:

```rust
pub struct PlannerState {
    pub desired_hash: String,     // plan-time hash
    pub stored_hash: Option<String>,  // executor lock hash
    pub converged: bool,
}
```

Under the handler invariant, three Kani proofs verify idempotency:
- `proof_idempotency_conditional`: converged + handler invariant → NoOp
- `proof_apply_then_noop`: apply stores matching hash → next plan is NoOp
- `proof_fleet_convergence`: N converged resources → all-NoOp fleet plan

Additional proofs cover the OCI build pipeline:
- `proof_layer_determinism`: same files produce same layer digest
- `proof_store_idempotency`: content-addressable put is idempotent
