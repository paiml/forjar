# Convergence Testing & Sandbox Isolation

Forjar implements property-based convergence testing with sandbox isolation for safe verification.

## Convergence Model (FJ-2600/2601)

Three properties tested for every resource:

| Property | Definition |
|----------|-----------|
| **Convergence** | First apply reaches desired state |
| **Idempotency** | Second apply is a no-op |
| **Preservation** | State unchanged after other resources apply |

```rust
use forjar::core::store::convergence_runner::{
    ConvergenceResult, ConvergenceSummary, ConvergenceTarget, run_convergence_test,
};

let target = ConvergenceTarget {
    resource_id: "config".into(),
    resource_type: "file".into(),
    apply_script: "echo 'hello' > $FORJAR_SANDBOX/test.txt".into(),
    state_query_script: "cat $FORJAR_SANDBOX/test.txt".into(),
    expected_hash: String::new(),
};
let result = run_convergence_test(&target);
assert!(result.passed()); // converged + idempotent + preserved
```

### Safety Guard

Scripts containing system-modifying commands (`systemctl`, `apt-get`, `pkill`, `mount`, etc.) are automatically rejected in local sandbox mode — they require a container backend.

### Summary Aggregation

```rust
let summary = ConvergenceSummary::from_results(&results);
println!("{}", summary); // "Convergence: 8/10 passed (80%)"
```

## Sandbox Isolation (FJ-2603)

Four isolation levels:

| Level | Network | Filesystem | Seccomp | cgroups |
|-------|---------|------------|---------|---------|
| Full | Blocked | Isolated | Yes (deny connect/mount/ptrace) | Yes |
| NetworkOnly | Allowed | Isolated | No | Yes |
| Minimal | Allowed | Isolated (PID/mount) | No | Yes |
| None | Allowed | No isolation | No | No |

### Preset Profiles

```rust
use forjar::core::store::sandbox::preset_profile;

let gpu = preset_profile("gpu").unwrap();
// 16 GB memory, 8 CPUs, GPU device bind, NVIDIA_VISIBLE_DEVICES=all
```

| Profile | Level | Memory | CPUs | Timeout |
|---------|-------|--------|------|---------|
| `full` | Full | 2 GB | 4 | 600s |
| `network-only` | NetworkOnly | 4 GB | 8 | 1200s |
| `minimal` | Minimal | 1 GB | 2 | 300s |
| `gpu` | NetworkOnly | 16 GB | 8 | 3600s |

### Config Validation

```rust
use forjar::core::store::sandbox::{validate_config, SandboxConfig, SandboxLevel};

let config = SandboxConfig {
    level: SandboxLevel::Full,
    memory_mb: 2048,
    cpus: 4.0,
    timeout: 600,
    bind_mounts: vec![],
    env: vec![],
};
let errors = validate_config(&config);
assert!(errors.is_empty());
```

Validates: memory > 0 and ≤ 1 TiB, cpus > 0 and ≤ 1024, timeout > 0, bind mount paths non-empty.

### 10-Step Sandbox Lifecycle

1. Create PID/mount/net namespace
2. Mount overlayfs (lower=inputs, upper=tmpfs)
3. Bind inputs read-only
4. Apply cgroup limits (memory, CPU)
5. Seccomp BPF (Full level: deny connect/mount/ptrace)
6. Execute bashrs-purified build script
7. Extract outputs from `$out`
8. Compute BLAKE3 hash of output directory
9. Atomic move to content-addressed store
10. Destroy namespace and clean up

## WASM Deployment (FJ-2402)

Size budget enforcement and drift detection for WASM bundles:

```rust
use forjar::core::types::{WasmSizeBudget, BundleSizeDrift};

let budget = WasmSizeBudget::default(); // 100 KB core, 500 KB full
let drift = BundleSizeDrift::check(&budget, 90 * 1024, Some(85 * 1024));
assert!(drift.is_ok()); // within budget, < 20% growth
```

## Reproducible Builds (FJ-2403)

```rust
use forjar::core::types::ReproBuildConfig;

let config = ReproBuildConfig::default();
assert!(config.is_reproducible()); // locked + no_incremental + lto + codegen_units=1
```

## Falsification

```bash
cargo run --example validation_convergence_falsification
```

Key invariants verified:
- Convergence result requires all three properties (converge + idem + preserve)
- Summary correctly counts pass/fail by property
- Full sandbox blocks network, enforces FS isolation + seccomp
- Minimal sandbox enforces FS but no seccomp
- None sandbox has no isolation
- All 4 preset profiles exist with correct settings
- Sandbox config rejects zero memory/cpus/timeout and excessive limits
