# Lock Files, Profile Generations, References & Cache

Falsification coverage for FJ-1310 (lock files), FJ-1302 (profile generations), FJ-1304 (reference scanning), and FJ-1320 (cache configuration).

## Lock Files (FJ-1310)

Pins all resolved inputs to specific versions and BLAKE3 hashes:

```rust
use forjar::core::store::lockfile::{parse_lockfile, check_staleness, check_completeness};

let lf = parse_lockfile(yaml)?;
let stale = check_staleness(&lf, &current_hashes); // hash mismatches
let missing = check_completeness(&lf, &all_inputs); // unpinned inputs
```

## Profile Generations (FJ-1302)

Atomic symlink-based generation management for instant rollback:

```rust
use forjar::core::store::profile::*;

let gen0 = create_generation(&profiles_dir, "/store/hash-v1")?; // gen 0
let gen1 = create_generation(&profiles_dir, "/store/hash-v2")?; // gen 1
assert_eq!(current_generation(&profiles_dir), Some(1));

rollback(&profiles_dir)?; // back to gen 0
let gens = list_generations(&profiles_dir)?; // [(0, target), (1, target)]
```

## Reference Scanning (FJ-1304)

Conservative scanning for BLAKE3 store hash references in file content:

```rust
use forjar::core::store::reference::{scan_file_refs, scan_directory_refs, is_valid_blake3_hash};

assert!(is_valid_blake3_hash("blake3:abcdef...64hex"));
let refs = scan_file_refs(content, &known_hashes); // only known hashes
let all_refs = scan_directory_refs(dir, &known_hashes)?; // recursive walk
```

## Cache Configuration (FJ-1320)

SSH/local binary cache with substitution protocol:

```rust
use forjar::core::store::cache::*;

let cfg = parse_cache_config(yaml)?;
let result = resolve_substitution(hash, &local_entries, &cache_inventories);
match result {
    SubstitutionResult::LocalHit { store_path } => /* use local */,
    SubstitutionResult::CacheHit { source_index, .. } => /* pull from cache */,
    SubstitutionResult::CacheMiss => /* build from scratch */,
}
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_lockfile_profile_cache.rs` | 37 | ~380 |
