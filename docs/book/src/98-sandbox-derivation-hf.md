# Sandbox Planning, Derivation Lifecycle & HF Kernel Mapping

Falsification coverage for FJ-1316 (sandbox lifecycle), FJ-1342 (derivation DAG), and FJ-1350 (HF kernel mapping).

## Sandbox Planning (FJ-1316)

Plans the 10-step sandbox build lifecycle without executing:

```rust
use forjar::core::store::sandbox::{SandboxConfig, SandboxLevel};
use forjar::core::store::sandbox_exec::{plan_sandbox_build, validate_plan};

let config = SandboxConfig {
    level: SandboxLevel::Full,
    memory_mb: 4096, cpus: 8.0, timeout: 300,
    bind_mounts: vec![], env: vec![],
};
let plan = plan_sandbox_build(&config, "hash123", &inputs, "make", store_dir);
let errors = validate_plan(&plan);
```

### Sandbox Levels

| Level | Steps | Seccomp |
|-------|-------|---------|
| Full | 10 | 3 rules (connect, mount, ptrace) |
| NetworkOnly | 10 | 0 rules |
| Minimal | 9 | skips seccomp step |
| None | 9 | skips seccomp step |

### OCI Export

```rust
use forjar::core::store::sandbox_exec::{export_overlay_upper, oci_layout_plan};

let export_steps = export_overlay_upper(&overlay, output_path); // 3 steps
let oci_steps = oci_layout_plan(output_dir, "myapp:latest");    // 4 steps
```

## Derivation Lifecycle (FJ-1342)

Plans and simulates content-addressed derivation builds:

```rust
use forjar::core::store::derivation_exec::{plan_derivation, is_store_hit};

let plan = plan_derivation(&drv, &resolved, &store_entries, store_dir)?;
if is_store_hit(&plan) {
    println!("Cache hit — skipping build");
}
```

### DAG Execution

```rust
use forjar::core::store::derivation_exec::execute_derivation_dag;

let results = execute_derivation_dag(
    &derivations, &topo_order, &resolved, &store_entries, store_dir,
)?;
```

Each derivation computes a closure hash from sorted inputs + script + arch. Store hits skip the build entirely.

## HF Kernel Mapping (FJ-1350)

Maps HuggingFace model architectures to GPU kernel contracts:

```rust
use forjar::core::store::hf_config::{parse_hf_config_str, required_kernels};

let config = parse_hf_config_str(json)?;
let kernels = required_kernels(&config);
for k in &kernels {
    println!("{}: {}", k.op, k.contract);
}
```

### Model Architecture → Kernels

| Architecture | Key Kernels |
|-------------|-------------|
| LLaMA | rmsnorm, silu, rope, swiglu, gqa |
| GPT-2 | layernorm, gelu, absolute_position, tied_embeddings |
| Qwen2 | rmsnorm, silu, rope, bias_add, gqa |
| DeepSeek | rmsnorm, silu, rope, qk_norm |
| Gemma | rmsnorm, gelu, rope, tied_embeddings |

All architectures include base kernels: softmax, matmul, embedding_lookup.

## Rejection Criteria

34 falsification tests in `tests/falsification_sandbox_derivation_hf.rs`:

- **Sandbox**: step counts per level, seccomp rules, overlay config, validation, simulation determinism
- **Derivation**: store hit/miss detection, DAG execution ordering, invalid derivation rejection
- **HF**: config parsing, kernel contracts for 6 model families, unknown model fallback
