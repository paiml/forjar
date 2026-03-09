# Codegen, Migration, HF Config, Dispatch & SAT

Falsification coverage for FJ-005, FJ-044, FJ-1350, FJ-2700, and FJ-045.

## Script Codegen (FJ-005)

Three script types generated for each resource:

| Script | Purpose |
|--------|---------|
| `check_script` | Read current state |
| `apply_script` | Converge to desired state |
| `state_query_script` | Query observable state for hashing |

```rust
use forjar::core::codegen::{check_script, apply_script};
use forjar::core::types::{Resource, ResourceType};

let mut r = Resource { resource_type: ResourceType::File, ..Default::default() };
r.path = Some("/etc/app.conf".into());
r.content = Some("key=value".into());
let script = apply_script(&r).unwrap();
```

Supported for all 15 resource types except Recipe (must expand first). `sudo: true` wraps the apply script in a `sudo bash` heredoc.

## Docker → Pepita Migration (FJ-044)

```rust
use forjar::core::migrate::docker_to_pepita;

let result = docker_to_pepita("app", &docker_resource);
// result.resource.resource_type == ResourceType::Pepita
// result.warnings contains migration notes
```

State mapping: `running` → `present`, `stopped` → `absent`, `absent` → `absent`.

Port-exposed Docker containers get `netns: true` in pepita. Warnings generated for images, volumes, environment, and restart policies.

## HuggingFace Config (FJ-1350)

Maps model architectures to kernel contracts:

```rust
use forjar::core::store::hf_config::{parse_hf_config_str, required_kernels};

let config = parse_hf_config_str(r#"{"model_type":"llama","num_attention_heads":32,"num_key_value_heads":8}"#).unwrap();
let kernels = required_kernels(&config);
// → rmsnorm, silu, rope, swiglu, gqa, softmax, matmul, embedding_lookup
```

Supports: llama, qwen2, mistral, gemma, phi, starcoder2, gpt2, falcon, internlm2, deepseek_v2.

GQA detected when `num_key_value_heads < num_attention_heads`.

## Task Dispatch (FJ-2700)

```rust
use forjar::core::task::dispatch::{prepare_dispatch, success_rate};
use forjar::core::types::DispatchConfig;

let config = DispatchConfig {
    name: "deploy".into(),
    command: "deploy --env {{ env }}".into(),
    params: vec![("env".into(), "production".into())],
    timeout_secs: Some(300),
};
let prepared = prepare_dispatch(&config, &[]);
// prepared.command == "deploy --env production"
```

## SAT Dependency Resolution (FJ-045)

DPLL solver proves dependency satisfiability:

```rust
use forjar::core::planner::sat_deps::{build_sat_problem, solve, SatResult};

let resources = vec!["app".into(), "db".into()];
let deps = vec![("app".into(), "db".into())];
let problem = build_sat_problem(&resources, &deps);
match solve(&problem) {
    SatResult::Satisfiable { assignment } => { /* all resources included */ }
    SatResult::Unsatisfiable { conflict_clause } => { /* conflict detected */ }
}
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_codegen_migrate.rs` | 26 | 406 |
| `falsification_task_dispatch.rs` | 23 | 337 |
