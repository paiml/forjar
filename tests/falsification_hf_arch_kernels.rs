//! FJ-1350: Architecture constraint branches for required_kernels.
//!
//! Popperian rejection criteria for all model_type branches in arch_constraints:
//! mistral/mixtral, phi/phi3, starcoder2, falcon, internlm2, codellama,
//! qwen2_moe, gemma2, gpt_neo, gpt_neox.
//!
//! Usage: cargo test --test falsification_hf_arch_kernels

use forjar::core::store::hf_config::{required_kernels, HfModelConfig};

fn config_for(model_type: &str) -> HfModelConfig {
    HfModelConfig {
        model_type: model_type.into(),
        architectures: vec![],
        hidden_size: None,
        num_attention_heads: Some(32),
        num_key_value_heads: Some(8),
        num_hidden_layers: None,
        intermediate_size: None,
        vocab_size: None,
        max_position_embeddings: None,
    }
}

fn ops_for(model_type: &str) -> Vec<String> {
    required_kernels(&config_for(model_type))
        .iter()
        .map(|k| k.op.clone())
        .collect()
}

#[test]
fn mistral_llama_like() {
    let ops = ops_for("mistral");
    assert!(ops.contains(&"rmsnorm".into()));
    assert!(ops.contains(&"silu".into()));
    assert!(ops.contains(&"rope".into()));
    assert!(ops.contains(&"swiglu".into()));
    assert!(!ops.contains(&"bias_add".into()), "mistral has no bias");
    assert!(!ops.contains(&"tied_embeddings".into()));
}

#[test]
fn mixtral_same_as_mistral() {
    let ops = ops_for("mixtral");
    assert!(ops.contains(&"rmsnorm".into()));
    assert!(ops.contains(&"swiglu".into()));
    assert!(!ops.contains(&"bias_add".into()));
}

#[test]
fn phi_has_bias() {
    let ops = ops_for("phi");
    assert!(ops.contains(&"rmsnorm".into()));
    assert!(ops.contains(&"silu".into()));
    assert!(ops.contains(&"rope".into()));
    assert!(ops.contains(&"swiglu".into()));
    assert!(ops.contains(&"bias_add".into()), "phi has bias");
    assert!(!ops.contains(&"tied_embeddings".into()));
}

#[test]
fn phi3_same_as_phi() {
    let ops = ops_for("phi3");
    assert!(ops.contains(&"bias_add".into()));
    assert!(ops.contains(&"swiglu".into()));
}

#[test]
fn starcoder2_layernorm_gelu_rope() {
    let ops = ops_for("starcoder2");
    assert!(ops.contains(&"layernorm".into()));
    assert!(ops.contains(&"gelu".into()));
    assert!(ops.contains(&"rope".into()));
    assert!(ops.contains(&"gelu_mlp".into()));
    assert!(ops.contains(&"bias_add".into()));
    assert!(!ops.contains(&"rmsnorm".into()));
    assert!(!ops.contains(&"silu".into()));
}

#[test]
fn falcon_layernorm_gelu_rope_no_bias() {
    let ops = ops_for("falcon");
    assert!(ops.contains(&"layernorm".into()));
    assert!(ops.contains(&"gelu".into()));
    assert!(ops.contains(&"rope".into()));
    assert!(ops.contains(&"gelu_mlp".into()));
    assert!(!ops.contains(&"bias_add".into()), "falcon has no bias");
    assert!(!ops.contains(&"tied_embeddings".into()));
}

#[test]
fn internlm2_llama_like() {
    let ops = ops_for("internlm2");
    assert!(ops.contains(&"rmsnorm".into()));
    assert!(ops.contains(&"silu".into()));
    assert!(ops.contains(&"rope".into()));
    assert!(ops.contains(&"swiglu".into()));
    assert!(!ops.contains(&"bias_add".into()));
}

#[test]
fn codellama_same_as_llama() {
    let ops = ops_for("codellama");
    assert!(ops.contains(&"rmsnorm".into()));
    assert!(ops.contains(&"silu".into()));
    assert!(ops.contains(&"rope".into()));
    assert!(ops.contains(&"swiglu".into()));
    assert!(!ops.contains(&"bias_add".into()));
}

#[test]
fn qwen2_moe_same_as_qwen2() {
    let ops = ops_for("qwen2_moe");
    assert!(ops.contains(&"rmsnorm".into()));
    assert!(ops.contains(&"silu".into()));
    assert!(ops.contains(&"swiglu".into()));
    assert!(ops.contains(&"bias_add".into()), "qwen2_moe has bias");
}

#[test]
fn gemma2_tied_embeddings() {
    let ops = ops_for("gemma2");
    assert!(ops.contains(&"rmsnorm".into()));
    assert!(ops.contains(&"gelu".into()));
    assert!(ops.contains(&"rope".into()));
    assert!(ops.contains(&"gelu_mlp".into()));
    assert!(ops.contains(&"tied_embeddings".into()));
    assert!(!ops.contains(&"bias_add".into()));
}

#[test]
fn gpt_neo_same_as_gpt2() {
    let ops = ops_for("gpt_neo");
    assert!(ops.contains(&"layernorm".into()));
    assert!(ops.contains(&"gelu".into()));
    assert!(ops.contains(&"absolute_position".into()));
    assert!(ops.contains(&"tied_embeddings".into()));
    assert!(ops.contains(&"bias_add".into()));
}

#[test]
fn gpt_neox_same_as_gpt2() {
    let ops = ops_for("gpt_neox");
    assert!(ops.contains(&"layernorm".into()));
    assert!(ops.contains(&"gelu".into()));
    assert!(ops.contains(&"absolute_position".into()));
    assert!(ops.contains(&"bias_add".into()));
}

#[test]
fn all_models_have_universal_kernels() {
    for model in &[
        "llama",
        "mistral",
        "phi",
        "starcoder2",
        "falcon",
        "internlm2",
        "deepseek_v2",
        "gemma",
        "gpt2",
        "qwen2",
    ] {
        let ops = ops_for(model);
        assert!(ops.contains(&"softmax".into()), "{model} missing softmax");
        assert!(ops.contains(&"matmul".into()), "{model} missing matmul");
        assert!(
            ops.contains(&"embedding_lookup".into()),
            "{model} missing embedding_lookup"
        );
    }
}
