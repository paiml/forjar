# Convert Execution & Cache Commands

Falsification coverage for FJ-1363 (convert --apply execution) and FJ-1360 (cache SSH command generation).

## Convert Execution (FJ-1363)

Applies automated conversion changes to YAML configs with backup and atomic write:

```rust
use forjar::core::store::convert_exec::apply_conversion;

let result = apply_conversion(&config_path, &report)?;
assert!(result.backup_path.exists());   // backup created before changes
assert!(result.changes_applied > 0);     // version pins, store flags applied
assert!(result.lock_pins_generated > 0); // lock file created
```

### Change Types

| Type | Effect |
|------|--------|
| `AddVersionPin` | Sets `version: latest` on unversioned resources |
| `EnableStore` | Adds `store: true` to enable content-addressed storage |
| `GenerateLockPin` | Generates BLAKE3 lock pin in `forjar.inputs.lock.yaml` |

### Safety

- Backup created before any modification (`forjar.yaml.bak`)
- Atomic write via temp file + rename
- Existing version/store fields are never overwritten
- Nonexistent resources in YAML are safely skipped

## Cache Command Generation (FJ-1360)

Generates rsync/cp commands for SSH and local cache transport:

```rust
use forjar::core::store::cache_exec::{pull_command, push_command};
use forjar::core::store::cache::CacheSource;

let ssh = CacheSource::Ssh { host: "cache.prod".into(), user: "forjar".into(),
    path: "/var/lib/forjar/cache".into(), port: Some(2222) };

let pull = pull_command(&ssh, "blake3:abc123", staging_path);
// rsync -az -e 'ssh -p 2222' 'forjar@cache.prod:.../abc123/' '/tmp/staging/'

let push = push_command(&ssh, "blake3:abc123", store_dir);
// rsync -az -e 'ssh -p 2222' '/store/abc123/' 'forjar@cache.prod:.../abc123/'
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_convert_cache_exec.rs` | 21 | ~280 |
