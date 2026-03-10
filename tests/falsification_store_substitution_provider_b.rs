//! FJ-1322/1333/1350/1348: Substitution protocol, provider import, HF config, and conda
//! falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-1322: Substitution protocol
//!   - plan_substitution: local hit / cache hit / cache miss paths
//!   - requires_build / requires_pull: outcome predicates
//!   - step_count: plan step accounting
//! - FJ-1333: Universal provider import
//!   - import_command: CLI generation for 8 providers
//!   - origin_ref_string: provenance ref formatting
//!   - validate_import: input validation
//!   - parse_import_config: YAML deserialization
//!   - capture_method: output capture descriptions
//!   - all_providers: complete provider listing
//! - FJ-1350: HuggingFace config and kernel mapping
//!   - parse_hf_config_str: config.json parsing
//!   - required_kernels: architecture-to-kernel mapping
//! - FJ-1348: Conda package parsing
//!   - parse_conda_index: index.json parsing
//!
//! Usage: cargo test --test falsification_store_substitution_provider

use forjar::core::store::cache::{CacheConfig, CacheInventory, CacheSource};
use forjar::core::store::conda::parse_conda_index;
use forjar::core::store::hf_config::{parse_hf_config_str, required_kernels, HfModelConfig};
use forjar::core::store::provider::{
    all_providers, capture_method, import_command, origin_ref_string, parse_import_config,
    validate_import, ImportConfig, ImportProvider,
};
use forjar::core::store::sandbox::{SandboxConfig, SandboxLevel};
use forjar::core::store::substitution::{
    plan_substitution, requires_build, requires_pull, step_count, SubstitutionContext,
    SubstitutionOutcome,
};
use std::collections::BTreeMap;

// ============================================================================
// FJ-1322: plan_substitution — local hit
// ============================================================================
// FJ-1333: capture_method and all_providers
// ============================================================================

#[test]
fn provider_capture_methods_non_empty() {
    for provider in all_providers() {
        let method = capture_method(provider);
        assert!(
            !method.is_empty(),
            "{:?} should have capture method",
            provider
        );
    }
}

#[test]
fn provider_all_providers_complete() {
    let providers = all_providers();
    assert_eq!(providers.len(), 8);
    assert!(providers.contains(&ImportProvider::Apt));
    assert!(providers.contains(&ImportProvider::Cargo));
    assert!(providers.contains(&ImportProvider::Docker));
    assert!(providers.contains(&ImportProvider::Apr));
}

// ============================================================================
// FJ-1333: ImportConfig serde roundtrip
// ============================================================================

#[test]
fn provider_config_yaml_roundtrip() {
    let config = ImportConfig {
        provider: ImportProvider::Cargo,
        reference: "serde".into(),
        version: Some("1.0".into()),
        arch: "aarch64".into(),
        options: BTreeMap::new(),
    };
    let yaml = serde_yaml_ng::to_string(&config).unwrap();
    let parsed: ImportConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(parsed.provider, ImportProvider::Cargo);
    assert_eq!(parsed.reference, "serde");
}

// ============================================================================
// FJ-1350: parse_hf_config_str
// ============================================================================

#[test]
fn hf_parse_llama_config() {
    let json = r#"{
        "model_type": "llama",
        "architectures": ["LlamaForCausalLM"],
        "hidden_size": 4096,
        "num_attention_heads": 32,
        "num_key_value_heads": 8,
        "num_hidden_layers": 32,
        "intermediate_size": 11008,
        "vocab_size": 32000,
        "max_position_embeddings": 4096
    }"#;
    let config = parse_hf_config_str(json).unwrap();
    assert_eq!(config.model_type, "llama");
    assert_eq!(config.hidden_size, Some(4096));
    assert_eq!(config.num_attention_heads, Some(32));
    assert_eq!(config.num_key_value_heads, Some(8));
}

#[test]
fn hf_parse_qwen2_config() {
    let json = r#"{
        "model_type": "qwen2",
        "architectures": ["Qwen2ForCausalLM"],
        "hidden_size": 3584,
        "num_attention_heads": 28,
        "num_key_value_heads": 4,
        "num_hidden_layers": 28,
        "vocab_size": 151936
    }"#;
    let config = parse_hf_config_str(json).unwrap();
    assert_eq!(config.model_type, "qwen2");
    assert_eq!(config.num_key_value_heads, Some(4));
}

#[test]
fn hf_parse_minimal_config() {
    let json = r#"{"model_type": "llama"}"#;
    let config = parse_hf_config_str(json).unwrap();
    assert_eq!(config.model_type, "llama");
    assert!(config.hidden_size.is_none());
    assert!(config.architectures.is_empty());
}

#[test]
fn hf_parse_invalid_json() {
    let result = parse_hf_config_str("not json");
    assert!(result.is_err());
}

// ============================================================================
// FJ-1350: required_kernels
// ============================================================================

#[test]
fn hf_kernels_llama_gqa() {
    let config = HfModelConfig {
        model_type: "llama".into(),
        architectures: vec!["LlamaForCausalLM".into()],
        hidden_size: Some(4096),
        num_attention_heads: Some(32),
        num_key_value_heads: Some(8), // GQA: kv < heads
        num_hidden_layers: Some(32),
        intermediate_size: Some(11008),
        vocab_size: Some(32000),
        max_position_embeddings: Some(4096),
    };
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"rmsnorm"), "llama uses RMSNorm");
    assert!(ops.contains(&"silu"), "llama uses SiLU");
    assert!(ops.contains(&"rope"), "llama uses RoPE");
    assert!(ops.contains(&"swiglu"), "llama uses SwiGLU");
    assert!(ops.contains(&"gqa"), "llama with kv<heads uses GQA");
    assert!(ops.contains(&"softmax"), "universal kernel");
    assert!(ops.contains(&"matmul"), "universal kernel");
    assert!(!ops.contains(&"bias_add"), "llama has no bias");
}

#[test]
fn hf_kernels_gpt2_layernorm() {
    let config = HfModelConfig {
        model_type: "gpt2".into(),
        architectures: vec![],
        hidden_size: Some(768),
        num_attention_heads: Some(12),
        num_key_value_heads: Some(12), // MHA: kv == heads
        num_hidden_layers: Some(12),
        intermediate_size: None,
        vocab_size: Some(50257),
        max_position_embeddings: Some(1024),
    };
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"layernorm"), "gpt2 uses LayerNorm");
    assert!(ops.contains(&"gelu"), "gpt2 uses GELU");
    assert!(ops.contains(&"absolute_position"), "gpt2 uses absolute pos");
    assert!(ops.contains(&"attention"), "MHA when kv==heads");
    assert!(ops.contains(&"bias_add"), "gpt2 has bias");
    assert!(ops.contains(&"tied_embeddings"), "gpt2 ties embeddings");
    assert!(!ops.contains(&"gqa"), "MHA, not GQA");
}

#[test]
fn hf_kernels_deepseek_qk_norm() {
    let config = HfModelConfig {
        model_type: "deepseek_v2".into(),
        architectures: vec![],
        hidden_size: None,
        num_attention_heads: Some(32),
        num_key_value_heads: Some(8),
        num_hidden_layers: None,
        intermediate_size: None,
        vocab_size: None,
        max_position_embeddings: None,
    };
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"qk_norm"), "deepseek_v2 has QK norm");
}

#[test]
fn hf_kernels_always_include_universal() {
    let config = HfModelConfig {
        model_type: "unknown_model".into(),
        architectures: vec![],
        hidden_size: None,
        num_attention_heads: None,
        num_key_value_heads: None,
        num_hidden_layers: None,
        intermediate_size: None,
        vocab_size: None,
        max_position_embeddings: None,
    };
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"softmax"), "always required");
    assert!(ops.contains(&"matmul"), "always required");
    assert!(ops.contains(&"embedding_lookup"), "always required");
}

// ============================================================================
// FJ-1348: parse_conda_index
// ============================================================================

#[test]
fn conda_parse_index_full() {
    let json = r#"{
        "name": "numpy",
        "version": "1.26.4",
        "build": "py312h2c0c3fe_0",
        "arch": "x86_64",
        "subdir": "linux-64"
    }"#;
    let info = parse_conda_index(json).unwrap();
    assert_eq!(info.name, "numpy");
    assert_eq!(info.version, "1.26.4");
    assert_eq!(info.build, "py312h2c0c3fe_0");
    assert_eq!(info.arch, "x86_64");
    assert_eq!(info.subdir, "linux-64");
}

#[test]
fn conda_parse_index_minimal() {
    let json = r#"{"name": "pkg", "version": "1.0"}"#;
    let info = parse_conda_index(json).unwrap();
    assert_eq!(info.name, "pkg");
    assert_eq!(info.version, "1.0");
    assert_eq!(info.arch, "noarch"); // default
    assert_eq!(info.subdir, "noarch"); // default
}

#[test]
fn conda_parse_index_missing_name() {
    let result = parse_conda_index(r#"{"version": "1.0"}"#);
    assert!(result.is_err());
}

#[test]
fn conda_parse_index_invalid_json() {
    let result = parse_conda_index("not json");
    assert!(result.is_err());
}
