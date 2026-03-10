# Provider Exec, Sandbox Validation & Sync Parsing

Falsification coverage for FJ-1359 (provider execution helpers), FJ-1361 (sandbox dry-run), and FJ-1362 (sync execution parsing).

## Provider Execution Helpers (FJ-1359)

Pure functions for staging scripts, directory hashing, and atomic store placement:

```rust
use forjar::core::store::provider_exec::*;

let script = build_staging_script("cargo install ripgrep", staging_dir);
// export STAGING='/tmp/staging'\nmkdir -p "$STAGING"\ncargo install ripgrep

let hash = hash_staging_dir(staging_dir)?;   // blake3:... deterministic
let (files, bytes) = dir_stats(output_dir);  // recursive count
atomic_move_to_store(staging, target)?;       // rename(2) with parent creation
```

### Hash Properties

- Deterministic: same content always produces same hash
- Sensitive: different content produces different hashes
- Recursive: nested directory structures are fully traversed
- Empty rejection: empty staging directories return an error

## Sandbox Dry-Run (FJ-1361)

Pre-flight validation of sandbox execution plans without running commands:

```rust
use forjar::core::store::sandbox_run::*;

let commands = dry_run_sandbox_plan(&plan)?;  // collect executable commands
let errors = validate_sandbox_commands(&plan); // I8 validation check
assert!(errors.is_empty());
```

Informational steps (no command) are automatically skipped.

## Sync Execution Parsing (FJ-1362)

Provider string parsing and reimport staging:

```rust
use forjar::core::store::sync_exec::*;

let provider = parse_provider("apt")?;  // → ImportProvider::Apt
let path = tempdir_for_reimport("blake3:abc123...");
// /tmp/forjar-reimport-abc1234567890123
```

All 8 providers supported: apt, cargo, uv, nix, docker, tofu, terraform, apr.

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_provider_sandbox_sync.rs` | 27 | ~230 |
