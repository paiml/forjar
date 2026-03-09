# Store Purity, Reproducibility Score, References & HF Config

Falsification coverage for FJ-1304, FJ-1305, FJ-1329, and FJ-1350.

## Purity Classification (FJ-1305)

Four-level purity model with monotonicity invariant:

```rust
use forjar::core::store::purity::*;

// Classify a resource based on its signals
let signals = PuritySignals {
    has_version: true,
    has_store: true,
    has_sandbox: true,
    ..Default::default()
};
let result = classify("nginx", &signals);
assert_eq!(result.level, PurityLevel::Pure);

// Monotonicity: resource purity = max(own, dependencies)
let signals = PuritySignals {
    has_version: true, has_store: true, has_sandbox: true,
    dep_levels: vec![PurityLevel::Impure],
    ..Default::default()
};
assert_eq!(classify("pkg", &signals).level, PurityLevel::Impure);

// Recipe-level aggregate (worst of all resources)
let recipe = recipe_purity(&[PurityLevel::Pure, PurityLevel::Pinned]);
assert_eq!(recipe, PurityLevel::Pinned);
```

### Purity Levels

| Level | Ord | Criteria |
|-------|-----|----------|
| Pure | 0 | version + store + sandbox |
| Pinned | 1 | version + (store OR sandbox) |
| Constrained | 2 | no version pin |
| Impure | 3 | curl-pipe detected, or impure dependency |

## Reproducibility Score (FJ-1329)

Weighted composite score with per-resource breakdown:

```rust
use forjar::core::store::repro_score::*;
use forjar::core::store::purity::PurityLevel;

let score = compute_score(&[
    ReproInput { name: "nginx".into(), purity: PurityLevel::Pure,
                 has_store: true, has_lock_pin: true },
    ReproInput { name: "script".into(), purity: PurityLevel::Impure,
                 has_store: false, has_lock_pin: false },
]);

// Weighted: purity 50%, store 30%, lock 20%
println!("{:.1} ({})", score.composite, grade(score.composite));
```

### Grade Thresholds

| Grade | Threshold |
|-------|-----------|
| A | >= 90 |
| B | >= 75 |
| C | >= 50 |
| D | >= 25 |
| F | < 25 |

## BLAKE3 Reference Validation (FJ-1304)

Content-addressed store references use BLAKE3 hashes:

```rust
use forjar::core::store::reference::*;
use std::collections::BTreeSet;

// Validate hash format: "blake3:" + 64 hex chars
assert!(is_valid_blake3_hash("blake3:aabbccdd..."));
assert!(!is_valid_blake3_hash("blake3:tooshort"));

// Scan file content for known references (GC root tracing)
let known: BTreeSet<String> = [hash.clone()].into();
let refs = scan_file_refs(content.as_bytes(), &known);
```

## HuggingFace Kernel Mapping (FJ-1350)

Maps model architecture to required GPU kernel contracts:

```rust
use forjar::core::store::hf_config::*;

let json = r#"{"model_type": "llama", "num_attention_heads": 32,
               "num_key_value_heads": 8}"#;
let config = parse_hf_config_str(json)?;
let kernels = required_kernels(&config);

for k in &kernels {
    println!("{} → {}", k.op, k.contract);
}
```

### Architecture-Specific Kernels

| Model | Norm | Activation | Position | Attention | Extras |
|-------|------|-----------|----------|-----------|--------|
| llama | rmsnorm | silu+swiglu | rope | GQA | — |
| gpt2 | layernorm | gelu | absolute | MHA | bias_add, tied_embeddings |
| qwen2 | rmsnorm | silu+swiglu | rope | GQA | bias_add |
| deepseek_v2 | rmsnorm | silu+swiglu | rope | GQA | qk_norm |

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_purity_reproscore_refs_hf.rs` | 29 | ~328 |
