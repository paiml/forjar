# Lock Files & Binary Cache

Falsification coverage for FJ-1310 and FJ-1320.

## Lock Files (FJ-1310)

`forjar.inputs.lock.yaml` pins resolved inputs to versions and BLAKE3 hashes:

```rust
use forjar::core::store::lockfile::*;

// Parse lock file
let lf = parse_lockfile("schema: '1'\npins:\n  nginx:\n    provider: apt\n    hash: blake3:abc\n")?;

// Detect stale pins (hash changed since lock)
let stale = check_staleness(&lockfile, &current_hashes);

// Check completeness (all inputs pinned)
let missing = check_completeness(&lockfile, &["nginx".into(), "curl".into()]);
```

## Binary Cache (FJ-1320)

SSH-only binary cache with substitution protocol:

```rust
use forjar::core::store::cache::*;

// Configure cache sources (SSH or local)
let config = CacheConfig {
    sources: vec![
        CacheSource::Local { path: "/var/lib/forjar/store".into() },
        CacheSource::Ssh { host: "cache.example.com".into(), user: "forjar".into(),
                           path: "/cache".into(), port: Some(2222) },
    ],
    auto_push: true,
    max_size_mb: 1024,
};
assert!(validate_cache_config(&config).is_empty());

// Substitution protocol: local → remote caches → build
let result = resolve_substitution("blake3:abc", &local_entries, &inventories);
match result {
    SubstitutionResult::LocalHit { store_path } => { /* use local */ }
    SubstitutionResult::CacheHit { source_index, .. } => { /* fetch from cache */ }
    SubstitutionResult::CacheMiss => { /* build from scratch */ }
}

// SSH command generation
assert_eq!(ssh_command(&ssh_source).unwrap(), "ssh -p 2222 forjar@cache.example.com");
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_staleness_lockfile_cache.rs` | 25 | ~328 |
