# Substitution Protocol & Conda Import

Falsification coverage for FJ-1322 and FJ-1348.

## Substitution Protocol (FJ-1322)

Full substitution protocol executor — local → SSH cache → build:

```rust
use forjar::core::store::substitution::*;

let ctx = SubstitutionContext {
    closure_hash: "blake3:abc",
    input_hashes: &inputs,
    local_entries: &local,
    cache_config: &config,
    cache_inventories: &inventories,
    sandbox: None,
    store_dir: Path::new("/store"),
};

let plan = plan_substitution(&ctx);
match &plan.outcome {
    SubstitutionOutcome::LocalHit { store_path } => { /* skip build */ }
    SubstitutionOutcome::CacheHit { source, .. } => { /* pull from SSH */ }
    SubstitutionOutcome::CacheMiss { .. } => { /* build from scratch */ }
}

assert!(!requires_build(&plan)); // only true for CacheMiss
assert!(requires_pull(&plan));   // only true for CacheHit
```

### Protocol Steps

1. `ComputeClosureHash` — record input hashes and closure
2. `CheckLocalStore` — check local store for hit
3. `CheckSshCache` — check each SSH source in order
4. `PullFromCache` — rsync from SSH cache (on hit)
5. `BuildFromScratch` — sandbox build (on miss)
6. `StoreResult` — store output in content-addressed store
7. `PushToCache` — auto-push to first SSH source (if configured)

### Auto-Push

When `CacheConfig.auto_push` is true and a build occurs, the result
is automatically pushed to the first SSH cache source:

```rust
// With auto_push: true, miss plan includes PushToCache step
let plan = plan_substitution(&ctx);
assert!(plan.steps.iter().any(|s| matches!(s, SubstitutionStep::PushToCache { .. })));
```

## Conda Package Import (FJ-1348)

Parse conda `index.json` metadata:

```rust
use forjar::core::store::conda::*;

let info = parse_conda_index(r#"{"name": "numpy", "version": "1.26.4",
    "build": "py312h", "arch": "x86_64", "subdir": "linux-64"}"#)?;
assert_eq!(info.name, "numpy");
assert_eq!(info.arch, "x86_64");

// Auto-detect and extract .conda (ZIP) or .tar.bz2 formats
let info = read_conda(path, output_dir)?;

// Convert conda package to FAR format
let manifest = conda_to_far(conda_path, far_output)?;
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_substitution_conda.rs` | 16 | ~370 |
