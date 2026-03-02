//! Tests for FJ-1350: HuggingFace config parser and kernel mapping.

use super::hf_config::{parse_hf_config_str, required_kernels, HfModelConfig};

fn qwen2_config_json() -> &'static str {
    r#"{
        "model_type": "qwen2",
        "architectures": ["Qwen2ForCausalLM"],
        "hidden_size": 3584,
        "num_attention_heads": 28,
        "num_key_value_heads": 4,
        "num_hidden_layers": 28,
        "intermediate_size": 18944,
        "vocab_size": 152064,
        "max_position_embeddings": 32768
    }"#
}

fn llama_config_json() -> &'static str {
    r#"{
        "model_type": "llama",
        "architectures": ["LlamaForCausalLM"],
        "hidden_size": 4096,
        "num_attention_heads": 32,
        "num_key_value_heads": 8,
        "num_hidden_layers": 32,
        "intermediate_size": 11008,
        "vocab_size": 32000,
        "max_position_embeddings": 4096
    }"#
}

fn gpt2_config_json() -> &'static str {
    r#"{
        "model_type": "gpt2",
        "architectures": ["GPT2LMHeadModel"],
        "hidden_size": 768,
        "num_attention_heads": 12,
        "num_hidden_layers": 12,
        "intermediate_size": 3072,
        "vocab_size": 50257,
        "max_position_embeddings": 1024
    }"#
}

#[test]
fn test_fj1350_parse_qwen2_config() {
    let config = parse_hf_config_str(qwen2_config_json()).unwrap();
    assert_eq!(config.model_type, "qwen2");
    assert_eq!(config.architectures, vec!["Qwen2ForCausalLM"]);
    assert_eq!(config.hidden_size, Some(3584));
    assert_eq!(config.num_attention_heads, Some(28));
    assert_eq!(config.num_key_value_heads, Some(4));
    assert_eq!(config.num_hidden_layers, Some(28));
    assert_eq!(config.intermediate_size, Some(18944));
    assert_eq!(config.vocab_size, Some(152064));
    assert_eq!(config.max_position_embeddings, Some(32768));
}

#[test]
fn test_fj1350_parse_llama_config() {
    let config = parse_hf_config_str(llama_config_json()).unwrap();
    assert_eq!(config.model_type, "llama");
    assert_eq!(config.num_key_value_heads, Some(8));
}

#[test]
fn test_fj1350_parse_gpt2_config() {
    let config = parse_hf_config_str(gpt2_config_json()).unwrap();
    assert_eq!(config.model_type, "gpt2");
    // GPT-2 has no num_key_value_heads field
    assert_eq!(config.num_key_value_heads, None);
}

#[test]
fn test_fj1350_parse_minimal_config() {
    let json = r#"{"model_type": "llama"}"#;
    let config = parse_hf_config_str(json).unwrap();
    assert_eq!(config.model_type, "llama");
    assert!(config.architectures.is_empty());
    assert_eq!(config.hidden_size, None);
}

#[test]
fn test_fj1350_parse_invalid_json_error() {
    let result = parse_hf_config_str("not json");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("parse config.json"));
}

#[test]
fn test_fj1350_parse_missing_model_type_error() {
    let json = r#"{"hidden_size": 4096}"#;
    let result = parse_hf_config_str(json);
    assert!(result.is_err());
}

#[test]
fn test_fj1350_qwen2_kernels() {
    let config = parse_hf_config_str(qwen2_config_json()).unwrap();
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();

    // Qwen2: RmsNorm, Silu, Rope, SwiGlu, has_bias, GQA (4 < 28)
    assert!(ops.contains(&"rmsnorm"));
    assert!(ops.contains(&"silu"));
    assert!(ops.contains(&"rope"));
    assert!(ops.contains(&"swiglu"));
    assert!(ops.contains(&"bias_add"));
    assert!(ops.contains(&"gqa"));
    // Universal
    assert!(ops.contains(&"softmax"));
    assert!(ops.contains(&"matmul"));
    assert!(ops.contains(&"embedding_lookup"));
    // Should NOT have these
    assert!(!ops.contains(&"layernorm"));
    assert!(!ops.contains(&"attention"));
    assert!(!ops.contains(&"tied_embeddings"));
}

#[test]
fn test_fj1350_qwen2_contract_names() {
    let config = parse_hf_config_str(qwen2_config_json()).unwrap();
    let kernels = required_kernels(&config);
    let contracts: Vec<&str> = kernels.iter().map(|k| k.contract.as_str()).collect();

    assert!(contracts.contains(&"rmsnorm-kernel-v1"));
    assert!(contracts.contains(&"silu-kernel-v1"));
    assert!(contracts.contains(&"rope-kernel-v1"));
    assert!(contracts.contains(&"swiglu-kernel-v1"));
    assert!(contracts.contains(&"bias-add-v1"));
    assert!(contracts.contains(&"gqa-kernel-v1"));
    assert!(contracts.contains(&"softmax-kernel-v1"));
    assert!(contracts.contains(&"matmul-kernel-v1"));
    assert!(contracts.contains(&"embedding-lookup-v1"));
}

#[test]
fn test_fj1350_llama_kernels() {
    let config = parse_hf_config_str(llama_config_json()).unwrap();
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();

    // Llama: RmsNorm, Silu, Rope, SwiGlu, no bias, GQA (8 < 32)
    assert!(ops.contains(&"rmsnorm"));
    assert!(ops.contains(&"silu"));
    assert!(ops.contains(&"rope"));
    assert!(ops.contains(&"swiglu"));
    assert!(ops.contains(&"gqa"));
    assert!(!ops.contains(&"bias_add"));
    assert!(!ops.contains(&"tied_embeddings"));
}

#[test]
fn test_fj1350_gpt2_kernels() {
    let config = parse_hf_config_str(gpt2_config_json()).unwrap();
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();

    // GPT-2: LayerNorm, Gelu, Absolute, GeluMlp, has_bias, tied_embeddings, MHA
    assert!(ops.contains(&"layernorm"));
    assert!(ops.contains(&"gelu"));
    assert!(ops.contains(&"absolute_position"));
    assert!(ops.contains(&"gelu_mlp"));
    assert!(ops.contains(&"bias_add"));
    assert!(ops.contains(&"tied_embeddings"));
    assert!(ops.contains(&"attention"));
    // Should NOT have these
    assert!(!ops.contains(&"rmsnorm"));
    assert!(!ops.contains(&"silu"));
    assert!(!ops.contains(&"rope"));
    assert!(!ops.contains(&"gqa"));
}

#[test]
fn test_fj1350_gqa_detection() {
    // GQA: kv_heads < attention_heads
    let config = HfModelConfig {
        model_type: "llama".to_string(),
        architectures: vec![],
        hidden_size: Some(4096),
        num_attention_heads: Some(32),
        num_key_value_heads: Some(8),
        num_hidden_layers: None,
        intermediate_size: None,
        vocab_size: None,
        max_position_embeddings: None,
    };
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"gqa"));
    assert!(!ops.contains(&"attention"));
}

#[test]
fn test_fj1350_mha_detection() {
    // MHA: kv_heads == attention_heads (or missing)
    let config = HfModelConfig {
        model_type: "llama".to_string(),
        architectures: vec![],
        hidden_size: Some(4096),
        num_attention_heads: Some(32),
        num_key_value_heads: Some(32),
        num_hidden_layers: None,
        intermediate_size: None,
        vocab_size: None,
        max_position_embeddings: None,
    };
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"attention"));
    assert!(!ops.contains(&"gqa"));
}

#[test]
fn test_fj1350_mha_when_kv_heads_missing() {
    let config = HfModelConfig {
        model_type: "llama".to_string(),
        architectures: vec![],
        hidden_size: None,
        num_attention_heads: Some(32),
        num_key_value_heads: None,
        num_hidden_layers: None,
        intermediate_size: None,
        vocab_size: None,
        max_position_embeddings: None,
    };
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"attention"));
    assert!(!ops.contains(&"gqa"));
}

#[test]
fn test_fj1350_unknown_model_defaults_to_llama() {
    let config = HfModelConfig {
        model_type: "totally_unknown_arch".to_string(),
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
    // Default = llama-like: RmsNorm, Silu, Rope, SwiGlu, no bias, no tied
    assert!(ops.contains(&"rmsnorm"));
    assert!(ops.contains(&"silu"));
    assert!(ops.contains(&"rope"));
    assert!(ops.contains(&"swiglu"));
    assert!(!ops.contains(&"bias_add"));
    assert!(!ops.contains(&"tied_embeddings"));
}

#[test]
fn test_fj1350_universal_kernels_always_present() {
    for model_type in &["qwen2", "llama", "gpt2", "gemma", "falcon", "phi"] {
        let config = HfModelConfig {
            model_type: model_type.to_string(),
            architectures: vec![],
            hidden_size: None,
            num_attention_heads: Some(32),
            num_key_value_heads: Some(32),
            num_hidden_layers: None,
            intermediate_size: None,
            vocab_size: None,
            max_position_embeddings: None,
        };
        let kernels = required_kernels(&config);
        let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
        assert!(ops.contains(&"softmax"), "{model_type} missing softmax");
        assert!(ops.contains(&"matmul"), "{model_type} missing matmul");
        assert!(
            ops.contains(&"embedding_lookup"),
            "{model_type} missing embedding_lookup"
        );
    }
}

#[test]
fn test_fj1350_deepseek_v2_has_qk_norm() {
    let config = HfModelConfig {
        model_type: "deepseek_v2".to_string(),
        architectures: vec![],
        hidden_size: None,
        num_attention_heads: Some(128),
        num_key_value_heads: Some(128),
        num_hidden_layers: None,
        intermediate_size: None,
        vocab_size: None,
        max_position_embeddings: None,
    };
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"qk_norm"));

    let contracts: Vec<&str> = kernels.iter().map(|k| k.contract.as_str()).collect();
    assert!(contracts.contains(&"qk-norm-v1"));
}

#[test]
fn test_fj1350_gemma_tied_embeddings() {
    let config = HfModelConfig {
        model_type: "gemma".to_string(),
        architectures: vec![],
        hidden_size: None,
        num_attention_heads: Some(16),
        num_key_value_heads: Some(16),
        num_hidden_layers: None,
        intermediate_size: None,
        vocab_size: None,
        max_position_embeddings: None,
    };
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"tied_embeddings"));
    assert!(ops.contains(&"gelu"));
    assert!(!ops.contains(&"silu"));
}

#[test]
fn test_fj1350_starcoder2_layernorm_gelu() {
    let config = HfModelConfig {
        model_type: "starcoder2".to_string(),
        architectures: vec![],
        hidden_size: None,
        num_attention_heads: Some(24),
        num_key_value_heads: Some(2),
        num_hidden_layers: None,
        intermediate_size: None,
        vocab_size: None,
        max_position_embeddings: None,
    };
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"layernorm"));
    assert!(ops.contains(&"gelu"));
    assert!(ops.contains(&"rope"));
    assert!(ops.contains(&"bias_add"));
    assert!(ops.contains(&"gqa")); // 2 < 24
}

#[test]
fn test_fj1350_parse_hf_config_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    std::fs::write(&path, qwen2_config_json()).unwrap();

    let config = super::hf_config::parse_hf_config(&path).unwrap();
    assert_eq!(config.model_type, "qwen2");
}

#[test]
fn test_fj1350_parse_hf_config_file_not_found() {
    let result = super::hf_config::parse_hf_config(std::path::Path::new("/no/such/file.json"));
    assert!(result.is_err());
}

#[test]
fn test_fj1350_extra_fields_ignored() {
    let json = r#"{
        "model_type": "llama",
        "torch_dtype": "bfloat16",
        "transformers_version": "4.40.0",
        "use_cache": true,
        "rope_theta": 10000.0
    }"#;
    let config = parse_hf_config_str(json).unwrap();
    assert_eq!(config.model_type, "llama");
}
