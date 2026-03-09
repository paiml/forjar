# Sandbox Lifecycle & Derivation DAG

Falsification coverage for FJ-1315, FJ-1316, and FJ-1341–FJ-1344.

## Sandbox Configuration (FJ-1315)

Four isolation levels with preset profiles:

```rust
use forjar::core::store::sandbox::*;

// Preset profiles: full, network-only, minimal, gpu
let config = preset_profile("full").unwrap();
assert!(validate_config(&config).is_empty());

// Network and filesystem isolation checks
assert!(blocks_network(SandboxLevel::Full));
assert!(enforces_fs_isolation(SandboxLevel::Minimal));
assert!(!enforces_fs_isolation(SandboxLevel::None));
```

| Level | Network | FS Isolation | Seccomp |
|-------|---------|-------------|---------|
| Full | blocked | yes | connect, mount, ptrace |
| NetworkOnly | allowed | yes | none |
| Minimal | allowed | yes | none |
| None | allowed | no | none |

## Sandbox Build Plan (FJ-1316)

10-step lifecycle plan (dry-run first):

```rust
use forjar::core::store::sandbox_exec::*;

let plan = plan_sandbox_build(&config, "blake3:hash", &inputs, "make build", store_dir);
// Steps: namespace → overlay → bind → cgroup → seccomp → build → extract → hash → store → destroy
assert!(validate_plan(&plan).is_empty());

// Simulate without real I/O
let result = simulate_sandbox_build(&config, "blake3:hash", &inputs, "make", store_dir);
assert!(result.output_hash.starts_with("blake3:"));
```

## Derivation Model (FJ-1341)

Derivations transform store entries via sandboxed builds:

```rust
use forjar::core::store::derivation::*;

// Parse from YAML
let drv = parse_derivation("inputs:\n  src:\n    store: blake3:abc\nscript: make")?;

// Closure hash = composite(sorted inputs + script + arch)
let closure = derivation_closure_hash(&drv, &input_hashes);

// Purity from sandbox level
assert_eq!(derivation_purity(&drv_with_full_sandbox), PurityLevel::Pure);
```

## Derivation Execution (FJ-1342–1343)

```rust
use forjar::core::store::derivation_exec::*;

// Plan: checks store hit, generates sandbox plan if miss
let plan = plan_derivation(&drv, &resolved, &store_entries, store_dir)?;
if is_store_hit(&plan) {
    // 7 steps skipped (steps 4-10)
} else {
    // Full build via sandbox
}

// Simulate (dry-run)
let result = simulate_derivation(&drv, &resolved, &[], store_dir)?;
```

## DAG Execution (FJ-1344)

```rust
use forjar::core::store::derivation::{validate_dag, Derivation};
use forjar::core::store::derivation_exec::execute_derivation_dag;

// Validate DAG (detect cycles)
let order = validate_dag(&graph)?; // topological sort

// Execute in order, propagating store hashes downstream
let results = execute_derivation_dag(&derivations, &order, &initial, &[], store_dir)?;
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_sandbox_derivation.rs` | 24 | ~387 |
| `falsification_derivation_dag.rs` | 40 | ~480 |
