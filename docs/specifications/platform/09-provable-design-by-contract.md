# 09: Provable Design by Contract

> Four-tier verification architecture: from runtime assertions to machine-checked proofs.

**Spec IDs**: FJ-2200 (runtime contracts), FJ-2201 (Kani real-code harnesses), FJ-2202 (Verus narrowed proofs), FJ-2203 (structural enforcement) | **Parent**: [forjar-platform-spec.md](../forjar-platform-spec.md)

---

## Current State: Four Layers, All Incomplete

Forjar has verification infrastructure at four levels. Each has specific gaps that weaken the overall provability story.

### Layer 1: Semantic Labels (`provable-contracts` macros)

10 functions annotated with `#[contract("name-v1", equation = "eq")]`:

| Contract ID | Functions | Module |
|------------|-----------|--------|
| `dag-ordering-v1` | `build_execution_order` | `resolver/dag.rs` |
| `execution-safety-v1` | `save_lock` | `core/state/mod.rs` |
| `blake3-state-v1` | `hash_file`, `hash_string`, `composite_hash` | `tripwire/hasher.rs` |
| `codegen-dispatch-v1` | `check_script`, `apply_script`, `state_query_script` | `core/codegen/dispatch.rs` |
| `recipe-determinism-v1` | `validate_inputs`, `expand_recipe` | `core/recipe/` |

**Gap**: These are metadata annotations. The macro names the contract and equation, but injects no assertions, pre/post checks, or verification conditions. They're documentation with a proc macro wrapper — useful for coverage tracking, but they prove nothing.

### Layer 2: Kani Bounded Model Checking

6 proof harnesses in `core/kani_proofs.rs`:

| Harness | What It Models | Bound |
|---------|---------------|-------|
| `proof_blake3_idempotency` | Hash determinism | 4 bytes |
| `proof_blake3_collision_resistance` | Hash uniqueness | 4 bytes |
| `proof_converged_state_is_noop` | Idempotency gate | 4 bytes |
| `proof_status_transition_monotonic` | Status enum transition | u8 |
| `proof_plan_determinism` | Plan consistency | 3 resources |
| `proof_topo_sort_stability` | DAG ordering | 3 nodes |

**Gap**: These operate on simplified models, not real code. `proof_plan_determinism` models the planner as two loops comparing `u32` values — it never calls `determine_present_action` or `hash_desired_state`. It proves that "comparing integers is deterministic," which is trivially true.

### Layer 3: Verus Formal Proofs

8 spec functions + 3 proof blocks in `core/verus_spec.rs` for a toy `ResourceState { desired_hash, current_hash, converged }`:

| Proof | Property | Scope |
|-------|----------|-------|
| `proof_termination` | Reconcile terminates in N+1 iterations | Toy model |
| `proof_convergence` | After reconcile, all resources converged | Toy model |
| `proof_idempotency` | Converged state has zero changes | Toy model |

**Gap**: The real planner (`planner/mod.rs:191`) has `determine_present_action` with `StateLock` containing `ResourceStatus` enums, nested HashMap lookups, and the dual-hash architecture. The Verus model captures none of this.

### Layer 4: Runtime Safety

- `Result<T, String>` on all I/O paths
- BLAKE3 sidecars for state integrity (`state/integrity.rs`)
- Tamper-evident event chain (`tripwire/chain.rs`)
- bashrs purification before shell execution (`core/purifier.rs`)
- Flight-grade bounds: `MAX_RESOURCES=256`, `MAX_DEPTH=32`

**Gap**: No runtime contract checking. No `debug_assert!` on invariants like "handler must store `hash_desired_state`." No pre/post conditions on critical-path functions.

---

## Five Gaps

### G1: The Critical Path Is Uncontracted

The most important function in the codebase — `determine_present_action` — has zero contracts:

```
fn determine_present_action(
    resource_id: &str,
    resource: &Resource,
    machine_name: &str,
    locks: &HashMap<String, StateLock>,
) -> PlanAction
    // NO precondition
    // NO postcondition
    // NO idempotency assertion
```

This function is the idempotency gate. Its correctness determines whether a second apply is a no-op. It needs:
- **Precondition**: `resource_id` is non-empty, `machine_name` is a valid machine
- **Postcondition**: if `rl.status == Converged && rl.hash == hash_desired_state(resource)` then result is `NoOp`
- **Determinism**: identical inputs always produce identical output

Similarly, `hash_desired_state` has no contract guaranteeing determinism (field ordering stability).

### G2: The Handler Invariant Is Unenforceable

The spec says: "all handlers must store `hash_desired_state()` as `rl.hash`." But:
- No trait constraint requires it
- No runtime assertion checks it
- No Kani/Verus proof covers it
- A new resource type can silently break idempotency

This is the most dangerous gap. The entire idempotency proof chain depends on this invariant, and nothing enforces it.

### G3: Kani Proofs Don't Touch Real Code

Compare what exists vs what's needed:

```
// EXISTS: proves integer comparison is deterministic (trivially true)
fn proof_plan_determinism():
    let current: u32 = kani::any()
    let desired: u32 = kani::any()
    if current != desired: changes_1 += 1
    if current != desired: changes_2 += 1
    assert_eq!(changes_1, changes_2)

// NEEDED: proves the REAL planner is deterministic
fn proof_real_plan_determinism():
    let resource: Resource = make_bounded_resource()
    let locks: HashMap<String, StateLock> = make_bounded_locks()
    let a1 = determine_present_action("r", &resource, "m", &locks)
    let a2 = determine_present_action("r", &resource, "m", &locks)
    assert_eq!(a1, a2)
```

### G4: New Platform Capabilities Have Zero Contracts

The entire container build pipeline — OCI assembly, layer construction, pepita-to-OCI export, distribution — has no formal properties. Critical contracts missing:
- Layer hash determinism (same inputs, same BLAKE3 + SHA-256)
- OCI manifest validity (valid JSON, all blobs referenced, correct media types)
- Store idempotency (storing the same content twice is a no-op)
- Dual-digest consistency (BLAKE3 store hash and SHA-256 OCI digest both computed correctly)

### G5: Labels Are Disconnected from Verification

The `#[contract("blake3-state-v1", equation = "hash_file")]` label on `hash_file()` has no connection to any Kani harness or Verus spec that proves the "hash_file" equation. The `contract_coverage.rs` module tracks implementation status, but "Implemented" means "YAML file exists," not "property verified."

---

## Target Architecture: Four Tiers

Each tier catches what the tier above cannot.

```
                Tier 4: Structural Enforcement
                ┌──────────────────────────────┐
                │ Trait + debug_assert on every │ ← handler invariant
                │ apply() — can't silently      │   structurally enforced
                │ break the hash contract       │
                └──────────────┬───────────────┘
                               │
                Tier 3: Verus Conditional Proofs
                ┌──────────────┴───────────────┐
                │ "IF handler invariant holds   │ ← machine-checked proof
                │  THEN idempotency holds"      │   for all inputs
                │ Narrowed to real types        │
                └──────────────┬───────────────┘
                               │
                Tier 2: Kani Bounded Model Checking
                ┌──────────────┴───────────────┐
                │ Real code harnesses verify    │ ← each handler checked
                │ each handler satisfies the    │   up to bounded inputs
                │ invariant                     │
                └──────────────┬───────────────┘
                               │
                Tier 1: Runtime Contracts
                ┌──────────────┴───────────────┐
                │ #[ensures] / debug_assert!    │ ← catches violations
                │ on critical-path functions    │   during integration tests
                └──────────────────────────────┘
```

### How the Tiers Compose

The proof chain works bottom-up:

1. **Runtime contracts** (Tier 1) catch violations during every test run. Zero false negatives for tested paths. Misses untested paths.
2. **Kani harnesses** (Tier 2) exhaustively explore all paths up to a bound. Proves each resource handler stores `hash_desired_state` for all inputs within the bound. Misses inputs beyond the bound.
3. **Verus proofs** (Tier 3) prove the conditional property for **all** inputs: "if the handler invariant holds, then idempotency holds." Unbounded. But the proof is conditional — it doesn't verify the handler invariant itself.
4. **Structural enforcement** (Tier 4) makes the handler invariant impossible to silently violate. The executor checks it at the point of use. A handler returning the wrong hash triggers a `debug_assert` failure.

Together: Verus proves the theorem, Kani verifies the hypothesis for each handler, runtime contracts catch regressions, and structural enforcement prevents silent introduction of violations.

---

## Tier 1: Runtime Contracts (FJ-2200)

Add the `contracts` crate (v0.6.7) for runtime pre/post checking on critical-path functions.

### Idempotency Gate

```rust
use contracts::*;

#[requires(!resource_id.is_empty(), "resource_id must be non-empty")]
#[requires(!machine_name.is_empty(), "machine_name must be non-empty")]
#[ensures(
    // Core idempotency postcondition:
    // If resource is converged and hash matches, result MUST be NoOp
    locks.get(machine_name)
        .and_then(|l| l.resources.get(resource_id))
        .map_or(true, |rl|
            !(rl.status == ResourceStatus::Converged
              && rl.hash == hash_desired_state(resource))
            || ret == PlanAction::NoOp
        )
)]
fn determine_present_action(
    resource_id: &str,
    resource: &Resource,
    machine_name: &str,
    locks: &HashMap<String, StateLock>,
) -> PlanAction
```

### Hash Determinism

```rust
#[ensures(ret == hash_desired_state(resource), "hash must be deterministic")]
pub fn hash_desired_state(resource: &Resource) -> String
```

The `contracts` crate's `old()` pseudo-function enables temporal contracts:

```rust
#[ensures(old(hash_desired_state(resource)) == ret)]
```

### Target Functions

| Function | Contract Type | Property |
|----------|--------------|----------|
| `determine_present_action` | `#[ensures]` | Converged + hash match → NoOp |
| `hash_desired_state` | `#[ensures]` | Deterministic (same input → same output) |
| `hash_desired_state` | golden test | Stable (fixed Resource → fixed hash across versions) |
| `save_lock` | `#[ensures]` | File exists after call, content matches |
| `build_execution_order` | `#[ensures]` | Output is valid topological order |
| `composite_hash` | `#[ensures]` | Deterministic |
| `hash_file` | `#[ensures]` | Deterministic for same file content |
| `apply_script` | `#[requires]` | Resource type is not Recipe |
| `export_overlay_upper` | `#[ensures]` | OCI whiteouts correctly converted |
| `build_layer` | `#[ensures]` | Layer digest matches content |

### Runtime Modes

| Mode | Mechanism | Cost |
|------|-----------|------|
| `#[requires]` / `#[ensures]` | `assert!` in all builds | Panic on violation |
| `#[debug_requires]` / `#[debug_ensures]` | `debug_assert!` | Zero cost in release |
| `#[test_requires]` / `#[test_ensures]` | `#[cfg(test)]` only | Zero cost outside tests |

**Recommendation**: Use `#[debug_ensures]` for hot-path functions (planner, hasher). Use `#[ensures]` for safety-critical functions (state writes, store operations).

---

## Tier 2: Kani Real-Code Harnesses (FJ-2201)

Replace abstract-model harnesses with harnesses that call real Forjar functions.

### Harness 1: Planner Idempotency (Real Code)

```rust
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(4)]
fn proof_planner_idempotency_real() {
    let resource = make_bounded_resource();  // bounded Resource struct
    let machine = "test-machine";
    let mut locks = HashMap::new();

    // First plan: should be Create (no lock entry)
    let a1 = determine_present_action("r1", &resource, machine, &locks);
    assert_eq!(a1, PlanAction::Create);

    // Simulate apply: create lock entry with hash_desired_state
    let mut lock = StateLock::new(machine);
    lock.resources.insert("r1".into(), ResourceLock {
        status: ResourceStatus::Converged,
        hash: hash_desired_state(&resource),
        ..Default::default()
    });
    locks.insert(machine.into(), lock);

    // Second plan: MUST be NoOp
    let a2 = determine_present_action("r1", &resource, machine, &locks);
    assert_eq!(a2, PlanAction::NoOp, "idempotency: second plan must be NoOp");
}
```

### Harness 2: Handler Invariant Per Resource Type

```rust
#[cfg(kani)]
#[kani::proof]
fn proof_handler_invariant_file() {
    let mut resource = Resource::default();
    resource.resource_type = ResourceType::File;
    resource.path = Some("/test/file".into());
    resource.content = Some(make_bounded_string());

    // The handler must return hash_desired_state as the stored hash
    let expected = hash_desired_state(&resource);
    let handler_hash = simulate_file_handler(&resource);
    assert_eq!(handler_hash, expected,
        "file handler must store hash_desired_state");
}

// One harness per resource type:
// proof_handler_invariant_package
// proof_handler_invariant_service
// proof_handler_invariant_mount
// proof_handler_invariant_cron
// proof_handler_invariant_docker
// proof_handler_invariant_gpu
```

### Harness 3: Hash Determinism

```rust
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(8)]
fn proof_hash_desired_state_determinism() {
    let resource = make_bounded_resource();
    let h1 = hash_desired_state(&resource);
    let h2 = hash_desired_state(&resource);
    assert_eq!(h1, h2, "hash_desired_state must be deterministic");
}
```

### Harness 4: DAG Ordering on Real Config

```rust
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(5)]
fn proof_dag_ordering_real() {
    let config = make_bounded_config();  // 3-4 resources with deps
    let o1 = build_execution_order(&config);
    let o2 = build_execution_order(&config);
    assert_eq!(o1, o2, "DAG ordering must be deterministic");

    // Verify topological property: for each edge A→B, A appears before B
    if let Ok(ref order) = o1 {
        verify_topological_property(&config, order);
    }
}
```

### Harness 5: OCI Layer Determinism

```rust
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(4)]
fn proof_layer_determinism() {
    let files: [(String, Vec<u8>); 2] = kani::any();
    let layer1 = build_file_layer(&files);
    let layer2 = build_file_layer(&files);
    assert_eq!(layer1.blake3_hash, layer2.blake3_hash);
    assert_eq!(layer1.sha256_digest, layer2.sha256_digest);
}
```

### Bounded Input Generators

```rust
#[cfg(kani)]
fn make_bounded_resource() -> Resource {
    let mut r = Resource::default();
    r.resource_type = match kani::any::<u8>() % 4 {
        0 => ResourceType::File,
        1 => ResourceType::Package,
        2 => ResourceType::Service,
        _ => ResourceType::Cron,
    };
    r.path = Some("/bounded/path".into());
    r.content = if kani::any() { Some("content".into()) } else { None };
    r.state = Some("present".into());
    r
}

#[cfg(kani)]
fn make_bounded_config() -> ForjarConfig {
    // 3 resources: A → B → C (chain), verifiable within unwind bound
    // ...
}
```

### Kani vs Existing Harnesses

| Property | Old (Abstract) | New (Real Code) |
|----------|---------------|-----------------|
| Hash determinism | Hashes 4-byte arrays | Calls `hash_desired_state` on `Resource` |
| Plan determinism | Compares u32 pairs | Calls `determine_present_action` |
| Status monotonicity | Compares u8 enum | Uses real `ResourceStatus` transitions |
| DAG ordering | 3-node bool adjacency | Calls `build_execution_order` on `ForjarConfig` |
| Idempotency | Not tested on real code | Full plan → apply → re-plan cycle |
| Handler invariant | Not tested | Per-resource-type harness |

---

## Tier 3: Verus Narrowed Proofs (FJ-2202)

Instead of verifying the full `Resource` type (30+ fields), narrow the Verus model to the real dual-hash comparison logic.

### The Conditional Idempotency Proof

```
// Model the REAL dual-hash architecture
struct PlannerState {
    desired_hash: Seq<u8>,    // output of hash_desired_state
    stored_hash: Seq<u8>,     // rl.hash from lock file
    status: ResourceStatus,   // real enum
}

spec fn plan_action(s: PlannerState) -> PlanAction {
    if s.status != Converged { Update }
    else if s.stored_hash == s.desired_hash { NoOp }
    else { Update }
}

// The handler invariant as a spec function
spec fn handler_stores_desired_hash(
    handler_output: Seq<u8>,
    desired: Seq<u8>
) -> bool {
    handler_output == desired
}

// MAIN THEOREM: idempotency holds given the handler invariant
proof fn proof_idempotency_conditional(s: PlannerState)
    requires
        s.status == Converged,
        handler_stores_desired_hash(s.stored_hash, s.desired_hash),
    ensures
        plan_action(s) == NoOp,
{
    // QED: stored == desired by invariant, status == Converged by precondition
    // → NoOp branch taken
}
```

### The Apply-Then-Plan Proof

```
// Model the apply phase
spec fn apply_result(
    resource: Resource,
    handler_hash: Seq<u8>,
) -> PlannerState {
    PlannerState {
        desired_hash: hash_desired_state_spec(resource),
        stored_hash: handler_hash,
        status: Converged,
    }
}

// After apply, if handler invariant holds, next plan is NoOp
proof fn proof_apply_then_noop(resource: Resource, handler_hash: Seq<u8>)
    requires
        handler_stores_desired_hash(handler_hash, hash_desired_state_spec(resource)),
    ensures
        plan_action(apply_result(resource, handler_hash)) == NoOp,
{
    // Follows from proof_idempotency_conditional
}
```

### The Convergence Proof (Extended)

```
// N resources, each with handler invariant
proof fn proof_fleet_convergence(
    resources: Seq<(Resource, PlannerState)>,
    handler_invariant_holds: bool,
)
    requires
        handler_invariant_holds,
        forall |i: int| 0 <= i < resources.len() ==>
            resources[i].1.status == Converged &&
            handler_stores_desired_hash(
                resources[i].1.stored_hash,
                hash_desired_state_spec(resources[i].0)
            ),
    ensures
        forall |i: int| 0 <= i < resources.len() ==>
            plan_action(resources[i].1) == NoOp,
{
    // Each resource satisfies proof_idempotency_conditional independently
}
```

### What Verus Proves vs What It Assumes

| Statement | Proved or Assumed |
|-----------|------------------|
| Converged + hash match → NoOp | **Proved** (for all inputs) |
| Apply sets status to Converged | **Proved** (model-level) |
| Handler stores `hash_desired_state` | **Assumed** (verified by Kani Tier 2) |
| `hash_desired_state` is deterministic | **Assumed** (verified by Kani Tier 2) |
| `hash_desired_state` is collision-free | **Assumed** (BLAKE3 property, not Forjar's to prove) |
| Real `determine_present_action` matches model | **Assumed** (bridged by runtime contracts Tier 1) |
| Hash field ordering is stable across versions | **Assumed** (enforced by golden hash test) |

**Model-implementation gap**: The Verus model and the runtime `#[debug_ensures]` contract encode the same postcondition. If the implementation diverges from the model, the runtime contract catches it during testing. This is defense-in-depth, not a formal model-implementation proof. See [08-known-limitations.md L14](08-known-limitations.md).

The proof is conditional: "IF the handler invariant holds, THEN idempotency holds." Kani verifies the antecedent. Verus proves the implication. Together they prove the conclusion — up to Kani's bound.

---

## Tier 4: Structural Enforcement (FJ-2203)

Make the handler invariant **structurally unbreakable**.

### The Handler Trait

```rust
/// Every resource handler must implement this trait.
/// The executor enforces that apply() returns hash_desired_state.
pub trait ResourceHandler {
    /// Apply the resource to the target machine.
    /// Returns ApplyResult containing the hash to store in the lock.
    ///
    /// CONTRACT: result.hash MUST equal hash_desired_state(resource).
    /// Violation triggers debug_assert failure in the executor.
    fn apply(
        &self,
        resource: &Resource,
        machine: &Machine,
        transport: &dyn Transport,
    ) -> Result<ApplyResult, String>;
}
```

### Executor Enforcement

```rust
fn execute_resource(
    handler: &dyn ResourceHandler,
    resource_id: &str,
    resource: &Resource,
    machine: &Machine,
    transport: &dyn Transport,
) -> Result<ResourceLock, String> {
    let result = handler.apply(resource, machine, transport)?;

    // STRUCTURAL INVARIANT: handler hash must match desired state hash
    let expected = hash_desired_state(resource);
    debug_assert_eq!(
        result.hash, expected,
        "HANDLER INVARIANT VIOLATED: {} handler stored hash {} != expected {}. \
         This breaks idempotency. The handler must return hash_desired_state().",
        resource.resource_type, result.hash, expected
    );

    Ok(ResourceLock {
        status: ResourceStatus::Converged,
        hash: result.hash,
        // ...
    })
}
```

### Why `debug_assert` Not `assert`

The handler invariant is a **logical** invariant, not a safety invariant. A violation doesn't corrupt memory or cause UB — it causes unnecessary re-applies. Using `debug_assert`:
- Catches violations in every test run and debug build
- Zero cost in release builds
- Prevents a single handler bug from crashing production
- Kani + Verus provide the stronger guarantee for release

If a handler legitimately needs to store a different hash (e.g., a live-state-query hash for drift detection), it must be documented and exempted:

```rust
/// EXEMPTION: This handler stores a live-state hash instead of
/// hash_desired_state. See docs/specifications/platform/09-provable-design-by-contract.md §L1.
/// Consequence: planner may produce false-negative re-applies for this resource type.
const HANDLER_INVARIANT_EXEMPT: bool = true;
```

---

## Contract Coverage Registry

Extend the existing `contract_coverage.rs` to track verification tier per contract.

### Coverage Levels

```
Level 0: Unlabeled     — no contract annotation
Level 1: Labeled       — #[contract] macro present, no verification
Level 2: Runtime       — #[ensures] / debug_assert active
Level 3: Bounded       — Kani harness covers this function
Level 4: Proved        — Verus spec with proof block
Level 5: Structural    — trait + executor enforcement
```

### Coverage Report

```
$ forjar contracts --coverage

Contract Coverage Report
========================
Total functions on critical path:  24
Level 5 (structural):              1   (execute_resource)
Level 4 (proved):                  3   (reconcile, apply, is_converged)
Level 3 (bounded):                 8   (real-code Kani harnesses)
Level 2 (runtime):                14   (#[ensures] contracts)
Level 1 (labeled):                10   (#[contract] macros)
Level 0 (unlabeled):              6   (needs annotation)

Handler Invariant Coverage:
  File handler:      Level 3 (Kani verified)
  Package handler:   Level 3 (Kani verified)
  Service handler:   Level 3 (Kani verified)
  Cron handler:      Level 3 (Kani verified)
  Mount handler:     Level 2 (runtime only)
  Docker handler:    Level 2 (runtime only)
  GPU handler:       Level 2 (runtime only)
  Image handler:     Level 1 (labeled only)    ← needs Kani harness
```

### Connecting Labels to Verification

The `#[contract]` macro gains an optional `verified_by` field:

```rust
#[contract("blake3-state-v1",
    equation = "hash_file",
    verified_by = "kani::proof_hash_file_determinism, runtime::debug_ensures"
)]
pub fn hash_file(path: &Path) -> Result<String, String>
```

The coverage checker validates that referenced harnesses and contracts actually exist.

---

## OCI Build Contracts (FJ-2200 Extension)

Contracts for the container build pipeline specified in [05-container-builds.md](05-container-builds.md).

### Layer Construction

```rust
#[contract("oci-layer-v1", equation = "deterministic_layer")]
#[debug_ensures(ret.is_ok() -> {
    let layer = ret.as_ref().unwrap();
    // Determinism: same call produces same hashes
    // (verified by Kani harness proof_layer_determinism)
    layer.blake3_hash.starts_with("blake3:")
        && layer.sha256_digest.starts_with("sha256:")
        && layer.diff_id.starts_with("sha256:")
})]
pub fn build_layer(group: &ResourceGroup) -> Result<Layer, String>
```

### OCI Manifest Validity

```rust
#[contract("oci-manifest-v1", equation = "valid_manifest")]
#[debug_ensures(ret.is_ok() -> {
    let manifest = ret.as_ref().unwrap();
    // All layer digests reference existing blobs
    manifest.layers.iter().all(|l| blobs.contains_key(&l.digest))
        // Config digest references existing blob
        && blobs.contains_key(&manifest.config.digest)
        // Schema version is 2
        && manifest.schema_version == 2
})]
pub fn assemble_manifest(layers: &[Layer], config: &ImageConfig) -> Result<Manifest, String>
```

### Store Idempotency

```rust
#[contract("store-v1", equation = "put_idempotent")]
#[debug_ensures({
    // After put, get returns the same content
    let stored = store::get(&hash);
    stored.is_some() && stored.unwrap() == content
})]
pub fn store_put(hash: &str, content: &[u8]) -> Result<(), String>
```

### Dual-Digest Consistency

```rust
#[contract("oci-layer-v1", equation = "dual_digest")]
#[debug_ensures(ret.is_ok() -> {
    let (blake3, sha256) = ret.as_ref().unwrap();
    // Both computed from same uncompressed content
    // blake3 for store addressing, sha256 for OCI
    blake3.starts_with("blake3:") && sha256.starts_with("sha256:")
})]
pub fn compute_dual_digest(uncompressed: &[u8]) -> Result<(String, String), String>
```

---

## Implementation

### Phase 13: Runtime Contracts (FJ-2200) — INCOMPLETE
- [x] Runtime contract postconditions (using `debug_assert!` — lighter than `contracts` crate)
- [ ] `#[debug_ensures]` on `determine_present_action` — **NOT on production function**; postcondition exists only on `spec_determine_present_action()` wrapper in `verus_spec.rs` (see E6 in FALSIFICATION-REPORT.md)
- [ ] `debug_assert!` on `hash_desired_state` — **NOT on production function**; exists on spec wrapper only
- [ ] `debug_assert!` on `save_lock` — **NOT on production function**
- [ ] `debug_assert!` on `build_execution_order` — **NOT on production function**
- [x] `debug_assert!` on `composite_hash`, `hash_file`, `hash_string` — these DO have `#[contract]` macros in `tripwire/hasher.rs` (metadata only, no runtime assertion)
- [ ] `#[debug_ensures]` on OCI `build_layer`, `assemble_manifest` — **NOT on production functions**
- [x] Wire `forjar contracts --coverage` command — command exists, reports Level 1 (labeled) for most functions
- **Deliverable**: ~~All critical-path functions have runtime contracts~~ Spec wrapper functions have contracts; production functions remain uncontracted. See Gap G1 above.
- **Five-Whys Remediation**: See end of file.

### Phase 14: Kani Real-Code Harnesses (FJ-2201) — INCOMPLETE
- [x] `proof_planner_idempotency_real` — **EXISTS but bounded toy model** (4-byte inputs, does NOT call `determine_present_action`; see E5 in FALSIFICATION-REPORT.md)
- [x] `proof_handler_invariant_{file,package,service}` — **EXISTS but abstract model** (verifies simplified hash comparison, not real handler code paths)
- [x] `proof_hash_determinism_real` — **EXISTS but 4-byte bound** (useful but limited)
- [x] `proof_dag_ordering_real` — **EXISTS but 3-node model** (not real `ForjarConfig`)
- [x] `proof_layer_determinism` — type-level only
- [x] `proof_store_idempotency` — type-level only
- [x] Deprecate abstract-model harnesses (documented in `kani_proofs.rs` with deprecation notice + proof assumptions table)
- **Deliverable**: ~~`cargo kani` passes on real-code harnesses~~ Harnesses exist but operate on abstract models with tiny bounds. 4 of 17 harnesses touch real types. None call production functions directly.
- **Five-Whys Remediation**: See end of file.

### Phase 15: Verus Narrowed Proofs (FJ-2202) — PARTIAL
- [x] `PlannerState` type exists modeling dual-hash — but proof operates on the model, not real `StateLock`/`ResourceStatus` types
- [x] `proof_idempotency_conditional` — **correct for toy model**; does NOT reference production `determine_present_action`
- [x] `proof_apply_then_noop` — **correct for toy model**; see note above
- [x] `proof_fleet_convergence` — **correct for toy model** (Seq-based, not HashMap-based)
- [x] Document proof assumptions — assumptions table exists in `kani_proofs.rs`
- **Extends**: `src/core/verus_spec.rs`
- **Deliverable**: ~~Verus proofs cover real hash pipeline~~ Verus proofs cover a simplified model of the hash pipeline. The model captures the core logic correctly but does not reference production types or functions. See Gap G3 and the "What Verus Proves vs What It Assumes" table above.

### Phase 16: Structural Enforcement (FJ-2203) -- PARTIAL
- [x] `HashInvariantCheck` type: pass/fail assertions with deviation_reason
- [x] `HandlerAuditReport` with checks, exemptions, `all_passed()`, `format_report()`
- [x] `HandlerExemption` with handler, reason, approved_by
- [x] `ContractAssertion` + `ContractKind` (requires/ensures/invariant) for runtime tracking
- [x] 6-level `VerificationTier` enum (Unlabeled→Structural) with ordering, serde, Display
- [x] `ContractEntry` per-function tracking with tier, contract_id, verified_by
- [x] `HandlerInvariantStatus` per-resource-type with exemption support
- [x] `ContractCoverageReport` with histogram, `at_or_above()`, `format_summary()`
- [x] `verified_by` field on `ContractEntry` (already present — `Vec<String>` linking functions to proofs)
- **Deliverable**: Handler invariant structurally enforced, coverage report shows per-tier status

---

## Verification State of the Art: Honest Assessment

What each tool can and cannot do, as of March 2026.

| Tool | Proves | Limitation | Annotation Cost |
|------|--------|-----------|----------------|
| **Verus** | Functional correctness for all inputs | Subset of Rust, no async, 3-4x annotation overhead | High |
| **Kani** | Safety + correctness up to a bound | Bounded only, no concurrency, monomorphic | Low |
| **contracts crate** | Nothing — runtime assertion framework | Catches violations, doesn't prove absence | Minimal |
| **Prusti** | Overflow/panic absence (zero annotation) | Safe Rust only, limited generics | Zero to low |
| **Creusot** | Functional correctness (Why3 backend) | Pearlite spec language, academic | High |

### What's Realistic for Forjar

Full formal verification of the entire codebase is not realistic. The Asterinas OS project — the most ambitious Rust verification effort to date — verified 11 of 14 priority targets in one module at 3-4x annotation overhead.

For Forjar, the practical target is:

| Module | Verification Level | Justification |
|--------|-------------------|---------------|
| Planner (`determine_present_action`, `hash_desired_state`) | Tier 3 (Verus) + Tier 2 (Kani) | Idempotency is the core guarantee |
| Handler hash storage | Tier 4 (structural) + Tier 2 (Kani) | The critical invariant |
| DAG resolver | Tier 2 (Kani) + Tier 1 (runtime) | Determinism matters, not worth Verus cost |
| OCI layer builder | Tier 2 (Kani) + Tier 1 (runtime) | Determinism + validity |
| State I/O (save_lock, integrity) | Tier 1 (runtime) | I/O-heavy, hard to model check |
| Transport (SSH, pepita) | Tier 1 (runtime) | External systems, can't verify |
| CLI parsing | None | Low-value target |

This gives the highest confidence where it matters most (idempotency, determinism) without the unsustainable annotation burden of trying to verify everything.

---

## Five-Whys Root Cause Analysis

### Why are runtime contracts on spec wrappers instead of production functions?

1. **Why aren't production functions contracted?** Because `#[ensures]` was added to `spec_*()` wrapper functions in `verus_spec.rs`, not to `determine_present_action()` in `planner/mod.rs`.
2. **Why were wrappers used?** Because the spec wrappers were written first as Verus proof targets, and contracts were added there for convenience during the formal verification work.
3. **Why weren't they moved to production code?** Because `determine_present_action` takes `HashMap<String, StateLock>` which the `contracts` crate's `#[ensures]` macro can't easily express postconditions over.
4. **Why can't the macro handle complex types?** The `contracts` crate only supports simple boolean expressions; the idempotency postcondition requires nested HashMap lookups and equality checks.
5. **Root cause**: The contract infrastructure was designed for the simple Verus model types, not the real production types with complex ownership and borrowing.

**Remediation**: Use `debug_assert!` directly inside `determine_present_action` after the match arms, rather than proc macro attributes. This is simpler and works with arbitrary Rust types. Estimated: 20 lines of code in `planner/mod.rs`.

### Why are Kani harnesses operating on toy models?

1. **Why don't Kani harnesses call `determine_present_action`?** Because the function takes `HashMap<String, StateLock>` and `Resource` with 30+ fields, exceeding Kani's bounded model checking capacity.
2. **Why does it exceed capacity?** Kani unrolls all possible values; `Resource` has `Option<String>` fields that explode the state space exponentially.
3. **Why not constrain the input space?** Bounded generators (`make_bounded_resource`) were created but still too large for Kani to verify in reasonable time.
4. **Why not use a different tool?** Proptest already covers this space empirically (see `tests_proptest_convergence.rs`). Kani's value-add is exhaustive verification, which is only feasible on small models.
5. **Root cause**: Fundamental mismatch between Kani's exhaustive bounded verification model and Forjar's complex aggregate types.

**Remediation**: Accept the limitation honestly. Kani proves properties of the abstract model. Proptest provides empirical verification of the concrete implementation. Runtime `debug_assert!` catches violations during integration testing. Together these provide strong (but not exhaustive) assurance. Rename harnesses from `proof_*_real` to `proof_*_model` for honesty.

### Why does the implementation section contradict the gaps section?

1. **Why do Phases 13-15 show `[x]` while Gaps G1-G5 say "uncontracted"?** Because phases were checked off when types and wrapper functions were created, without verifying that production functions were actually contracted.
2. **Why weren't production functions verified?** Because the implementer treated type creation and spec wrapper annotation as "done."
3. **Why was this accepted?** Because the spec review compared checkbox counts, not actual code paths.
4. **Why wasn't actual code audited?** Because the falsification report (E5, E6) identified this gap but remediation was deferred.
5. **Root cause**: Aspirational spec writing — checkboxes were marked based on intent and type existence rather than runtime verification.

**Remediation**: This document has been updated (2026-03-06) to change `[x]` to `[ ]` for items that exist only as spec wrappers, and to add "(types only)" / "(model only)" qualifiers. Future spec updates must distinguish between "type exists" and "production function contracted."
