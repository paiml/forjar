# Store Substitution, Provider Import & Kernel Mapping

Forjar's substitution protocol orchestrates the local → cache → build
fallback chain. The universal provider import interface seeds the
content-addressed store from any package manager.

## Substitution Protocol (FJ-1322)

Three-tier resolution: local store hit → SSH cache hit → build from scratch.

```rust
use forjar::core::store::substitution::{plan_substitution, requires_build};

let plan = plan_substitution(&ctx);
if requires_build(&plan) {
    // Build from scratch (sandbox optional)
} else {
    // Hit: either local or cache pull
}
```

Protocol steps:
1. `ComputeClosureHash` — record input closure identity
2. `CheckLocalStore` — content-addressed lookup
3. `CheckSshCache` — ordered SSH source check
4. `PullFromCache` / `BuildFromScratch` — resolve
5. `StoreResult` — persist to local store
6. `PushToCache` — auto-push to first SSH source (if configured)

## Universal Provider Import (FJ-1333)

8 providers supported: apt, cargo, uv, nix, docker, tofu, terraform, apr.

```rust
use forjar::core::store::provider::{import_command, validate_import, ImportConfig};

let config = ImportConfig {
    provider: ImportProvider::Apt,
    reference: "nginx".into(),
    version: Some("1.24.0".into()),
    ..
};
let cmd = import_command(&config);
// "apt-get install -y --download-only nginx=1.24.0"
```

## HuggingFace Kernel Mapping (FJ-1350)

Maps `model_type` to kernel contracts via architecture constraints:

```rust
use forjar::core::store::hf_config::{parse_hf_config_str, required_kernels};

let config = parse_hf_config_str(json)?;
let kernels = required_kernels(&config);
// llama: rmsnorm, silu, rope, swiglu, gqa, softmax, matmul, embedding_lookup
```

Supported model families: llama, qwen2, mistral, gemma, phi, starcoder2,
gpt2, falcon, internlm2, deepseek_v2.

## Chunker & Merkle Tree (FJ-1347)

64KB fixed-size chunking with binary Merkle tree verification:

```rust
use forjar::core::store::chunker::{chunk_bytes, tree_hash, reassemble};

let chunks = chunk_bytes(&data);
let root = tree_hash(&chunks);  // verified streaming
let original = reassemble(&chunks);
assert_eq!(original, data);
```

## Falsification Example

```bash
cargo run --example store_profile_sandbox_far
cargo test --test falsification_store_substitution_provider
cargo test --test falsification_store_db_chunker_layer
```
