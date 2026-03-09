# Sandbox Lifecycle & Derivation Execution

Falsification coverage for FJ-1316 and FJ-1342.

## Sandbox Build Lifecycle (FJ-1316)

10-step lifecycle for isolated, reproducible builds inside pepita namespaces.

### Lifecycle Steps

| Step | Description |
|------|-------------|
| 1 | Create PID/mount/net namespace |
| 2 | Mount overlayfs (lower=inputs, upper=tmpfs) |
| 3 | Bind inputs read-only (one per input) |
| 4 | Apply cgroup limits (memory, CPU) |
| 5 | Apply seccomp BPF (Full level only) |
| 6 | Execute bashrs-purified build script |
| 7 | Extract outputs from $out |
| 8 | Compute BLAKE3 hash of output directory |
| 9 | Atomic move to content-addressed store |
| 10 | Destroy namespace and clean up |

### Seccomp Rules

| Level | Denied Syscalls |
|-------|----------------|
| Full | connect, mount, ptrace |
| NetworkOnly | (none) |
| Minimal | (none) |
| None | (none) |

```rust
use forjar::core::store::sandbox_exec::{plan_sandbox_build, validate_plan};

let plan = plan_sandbox_build(&config, build_hash, &inputs, script, store_dir);
assert!(validate_plan(&plan).is_empty());
```

### OCI Integration

- `export_overlay_upper()` — converts overlayfs whiteouts to OCI format, creates layer tarball
- `oci_layout_plan()` — generates OCI Image Layout directory structure
- `multi_arch_index()` — builds multi-platform OCI Image Index
- `sha256_digest()` — computes SHA-256 for OCI DiffID
- `gzip_compress()` — compresses layer data

## Derivation Execution (FJ-1342)

Derivations take store entries as inputs, apply transformations in a sandbox, and produce new store entries.

### Store Hit/Miss

```rust
use forjar::core::store::derivation_exec::{plan_derivation, is_store_hit};

let plan = plan_derivation(&derivation, &resolved, &store_entries, store_dir)?;
if is_store_hit(&plan) {
    // Skip build — substitute from store
} else {
    // Build required — sandbox plan attached
}
```

### DAG Execution

```rust
use forjar::core::store::derivation_exec::execute_derivation_dag;

let results = execute_derivation_dag(
    &derivations, &topo_order, &initial_resources, &store_entries, store_dir
)?;
// Each derivation's output feeds into downstream derivations
```

On store hit, steps 4-10 are skipped (7 steps). The closure hash is computed from sorted input hashes + script hash + architecture.

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_sandbox_derivation.rs` | 22 | 386 |
| `falsification_derivation_exec.rs` | 8 | 169 |
