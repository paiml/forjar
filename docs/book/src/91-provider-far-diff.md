# Provider Import, FAR Archives & Store Diff

Falsification coverage for FJ-1333–1336 (provider import), FJ-1346 (FAR archive format), FJ-1345 (store diff and sync).

## Provider Import (FJ-1333–1336)

Universal import interface — any external tool can seed the forjar store:

```rust
use forjar::core::store::provider::{import_command, ImportConfig, ImportProvider};

let config = ImportConfig {
    provider: ImportProvider::Apt,
    reference: "nginx".into(),
    version: Some("1.24".into()),
    arch: "x86_64".into(),
    options: BTreeMap::new(),
};

let cmd = import_command(&config);
// "apt-get install -y --download-only nginx=1.24"
```

### Supported Providers

| Provider | Command | Capture Method |
|----------|---------|----------------|
| Apt | `apt-get install` | Package files via dpkg manifest |
| Cargo | `cargo install` | Binary output in $CARGO_HOME/bin/ |
| Uv | `uv pip install` | Virtualenv contents |
| Nix | `nix build` | Output tree in /nix/store/ |
| Docker | `docker create && export` | Filesystem snapshot |
| Tofu | `tofu output -json` | State outputs as YAML |
| Terraform | `terraform output -json` | State outputs as YAML |
| Apr | `apr pull` | Model artifacts (gguf, safetensors) |

## FAR Archive Format (FJ-1346)

Binary archive format with streaming decode:

```rust
use forjar::core::store::far::{encode_far, decode_far_manifest, FarManifest, FAR_MAGIC};

// Encode: magic → manifest_len → zstd(manifest) → chunks → sig
let mut buf = Vec::new();
encode_far(&manifest, &chunks, &mut buf)?;

// Decode (streaming — reads only header + manifest):
let (manifest, chunk_table) = decode_far_manifest(reader)?;
```

### Layout

```text
FAR_MAGIC (12 bytes: "FORJAR-FAR\x00\x01")
manifest_len (u64 LE)
zstd(manifest_yaml)
chunk_count (u64 LE)
chunk_table: [hash(32) + offset(u64) + length(u64)] × N
zstd(chunk_data) × N
signature_len (u64 LE, 0 = unsigned)
```

## Store Diff & Sync (FJ-1345)

Diff store entries against upstream origins and build sync plans:

```rust
use forjar::core::store::store_diff::{compute_diff, build_sync_plan};

let diff = compute_diff(&meta, Some("blake3:new_upstream_hash"));
if diff.upstream_changed {
    let plan = build_sync_plan(&[(meta, Some(upstream_hash))]);
    // plan.re_imports: leaf nodes to re-import
    // plan.derivation_replays: derived entries to rebuild (sorted by depth)
}
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_provider_far_diff.rs` | 38 | ~310 |
