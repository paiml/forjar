# Convergence Testing & Kernel FAR Packaging

Falsification coverage for FJ-2600 (convergence property testing) and FJ-1353 (kernel contract FAR packaging).

## Convergence Testing (FJ-2600)

Verifies three convergence properties for each resource:

```rust
use forjar::core::store::convergence_runner::*;

let target = ConvergenceTarget {
    resource_id: "app-config".into(),
    resource_type: "file".into(),
    apply_script: "echo 'port=8080' > ${FORJAR_SANDBOX}/etc/app.conf".into(),
    state_query_script: "cat ${FORJAR_SANDBOX}/etc/app.conf".into(),
    expected_hash: String::new(),
};

let result = run_convergence_test(&target);
assert!(result.converged);   // apply reaches desired state
assert!(result.idempotent);  // second apply is no-op
assert!(result.preserved);   // state unchanged after second apply
```

### Safety

Scripts containing system-modifying commands (`systemctl`, `apt-get`, `pkill`, etc.) are rejected in local mode — they require a container backend.

### Parallel Execution

```rust
let results = run_convergence_parallel(targets, 4); // 4 parallel sandboxes
let summary = ConvergenceSummary::from_results(&results);
println!("{}", summary); // "Convergence: 10/10 passed (100%)"
```

## Kernel FAR Packaging (FJ-1353)

Packages verified kernel contracts into FAR archives:

```rust
use forjar::core::store::kernel_far::contracts_to_far;

let manifest = contracts_to_far(&contracts_dir, &hf_config, &coverage, &far_path)?;
assert!(manifest.kernel_contracts.is_some());
assert_eq!(manifest.arch, "noarch");
```

### Onboarding Pipeline

1. Parse HuggingFace `config.json` for model architecture
2. Derive required kernel contracts from model type
3. Read binding registry and scan contracts directory
4. Compute coverage report
5. Scaffold missing contract YAML stubs
6. Package everything into a FAR archive

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_convergence_kernel.rs` | 22 | ~270 |
