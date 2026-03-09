# Store Profiles, Sandbox & FAR Archives

Forjar's content-addressed store uses generational profiles for
atomic rollback, sandbox isolation for build reproducibility, and
FAR archives for portable distribution.

## Profile Generations (FJ-1302)

Profiles are numbered symlink trees. Each `create_generation`
atomically switches a `current` symlink to the new generation
via temp symlink + rename(2).

```rust
use forjar::core::store::profile::{create_generation, rollback, list_generations};

let g0 = create_generation(&profiles_dir, "/store/hash0/content")?;
let g1 = create_generation(&profiles_dir, "/store/hash1/content")?;
// current -> generation 1

let rolled = rollback(&profiles_dir)?;
// current -> generation 0 (instant rollback)

let gens = list_generations(&profiles_dir)?;
// [(0, "/store/hash0/content"), (1, "/store/hash1/content")]
```

## Build Sandbox (FJ-1315)

4-level isolation model with resource limits:

| Level | Network | Filesystem | Use Case |
|-------|---------|------------|----------|
| Full | Blocked | Isolated | Reproducible builds |
| NetworkOnly | Allowed | Isolated | Package fetches |
| Minimal | Allowed | PID/mount NS | Legacy compat |
| None | Allowed | None | Debug/development |

```rust
use forjar::core::store::sandbox::{preset_profile, validate_config, blocks_network};

let config = preset_profile("full").unwrap();
assert!(validate_config(&config).is_empty()); // valid
assert!(blocks_network(config.level));         // no network

// GPU preset: network allowed, /dev/nvidia0 bind-mounted
let gpu = preset_profile("gpu").unwrap();
assert_eq!(gpu.memory_mb, 16384);
```

Presets: `full`, `network-only`, `minimal`, `gpu`.

## FAR Archive Format (FJ-1346)

Binary format for portable store entry distribution:

```
FORJAR-FAR\x00\x01  (12-byte magic)
manifest_len: u64    (LE)
zstd(manifest_yaml)  (YAML manifest, zstd-compressed)
chunk_count: u64     (LE)
chunk_table[]        (32-byte hash + u64 offset + u64 length per chunk)
zstd(chunks)         (zstd-compressed chunk data)
sig_len: u64         (0 = unsigned)
```

```rust
use forjar::core::store::far::{encode_far, decode_far_manifest, FarManifest};

// Encode
let mut buf = Vec::new();
encode_far(&manifest, &chunks, &mut buf)?;

// Decode (streaming — no full load)
let (manifest, chunk_table) = decode_far_manifest(reader)?;
```

## Falsification Example

```bash
cargo run --example store_profile_sandbox_far
```
