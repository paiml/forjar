# HF Config, Mutation Testing & Registry Push

Falsification coverage for FJ-1350, FJ-2604, FJ-2105, and FJ-1352.

## HuggingFace Config Parsing (FJ-1350)

Parses `config.json` and maps model architectures to kernel contracts.

### Architecture → Kernel Mapping

| Model Type | Norm | Activation | Position | MLP |
|-----------|------|-----------|----------|-----|
| llama, codellama | RmsNorm | SiLU | RoPE | SwiGLU |
| qwen2 | RmsNorm | SiLU | RoPE | SwiGLU (bias) |
| gpt2, gpt_neo | LayerNorm | GELU | Absolute | GELU MLP |
| gemma | RmsNorm | GELU | RoPE | GELU MLP |
| deepseek_v2 | RmsNorm | SiLU | RoPE | SwiGLU (QK-norm) |

```rust
use forjar::core::store::hf_config::{parse_hf_config_str, required_kernels};

let config = parse_hf_config_str(json)?;
let kernels = required_kernels(&config);
// GQA detected when num_key_value_heads < num_attention_heads
```

## Mutation Testing Operators (FJ-2604)

### Operator Applicability

| Operator | Applicable Types |
|----------|-----------------|
| DeleteFile, ModifyContent, ChangePermissions, CorruptConfig | file |
| StopService, KillProcess | service |
| RemovePackage | package |
| UnmountFilesystem | mount |

```rust
use forjar::core::store::mutation_runner::{applicable_operators, mutation_script};
use forjar::core::types::MutationOperator;

let ops = applicable_operators("file");
let script = mutation_script(MutationOperator::DeleteFile, "nginx.conf");
```

Scripts use `$FORJAR_SANDBOX` prefix for sandbox-aware paths.

## OCI Registry Push (FJ-2105)

### Config Validation

```rust
use forjar::core::store::registry_push::{validate_push_config, RegistryPushConfig};

let config = RegistryPushConfig {
    registry: "ghcr.io".into(),
    name: "myorg/myapp".into(),
    tag: "v1.0".into(),
    check_existing: true,
};
assert!(validate_push_config(&config).is_empty());
```

### Push Protocol

1. HEAD check for existing blobs (skip if exists)
2. POST to initiate upload
3. PUT for monolithic upload (< 64 MB) or chunked PATCH + PUT (>= 64 MB)
4. PUT manifest

## Contract Scaffolding (FJ-1352)

Generates YAML contract stubs for missing kernel operations.

```rust
use forjar::core::store::contract_scaffold::scaffold_contracts;

let stubs = scaffold_contracts(&missing_kernels, "team");
// Each stub has filename and yaml_content with EQ, PO, and FALSIFY sections
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_hf_mutation_registry.rs` | 33 | 478 |
