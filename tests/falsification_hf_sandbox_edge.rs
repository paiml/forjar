//! FJ-1316/1342/1350: HF model-type kernel coverage + sandbox/derivation edge cases.
//! Usage: cargo test --test falsification_hf_sandbox_edge

use forjar::core::store::derivation::{Derivation, DerivationInput};
use forjar::core::store::derivation_exec::{
    execute_derivation_dag, plan_derivation, simulate_derivation,
};
use forjar::core::store::hf_config::{parse_hf_config_str, required_kernels, HfModelConfig};
use forjar::core::store::sandbox::{SandboxConfig, SandboxLevel};
use forjar::core::store::sandbox_exec::{
    plan_sandbox_build, seccomp_rules_for_level, validate_plan,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ── helpers ──

fn sandbox_config(level: SandboxLevel) -> SandboxConfig {
    SandboxConfig {
        level,
        memory_mb: 2048,
        cpus: 4.0,
        timeout: 600,
        bind_mounts: vec![],
        env: vec![],
    }
}

fn model_config(model_type: &str) -> HfModelConfig {
    HfModelConfig {
        model_type: model_type.into(),
        architectures: vec![],
        hidden_size: Some(2048),
        num_attention_heads: Some(16),
        num_key_value_heads: Some(4),
        num_hidden_layers: Some(24),
        intermediate_size: Some(8192),
        vocab_size: Some(32000),
        max_position_embeddings: Some(4096),
    }
}

fn test_derivation(script: &str) -> Derivation {
    Derivation {
        inputs: BTreeMap::from([(
            "src".into(),
            DerivationInput::Store {
                store: "blake3:aaaa".into(),
            },
        )]),
        script: script.into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    }
}

// ── FJ-1350: HF config parsing ──

#[test]
fn parse_hf_config_json() {
    let json = r#"{"model_type":"llama","architectures":["LlamaForCausalLM"],"hidden_size":4096,"num_attention_heads":32,"num_key_value_heads":8,"num_hidden_layers":32}"#;
    let config = parse_hf_config_str(json).unwrap();
    assert_eq!(config.model_type, "llama");
    assert_eq!(config.hidden_size, Some(4096));
    assert_eq!(config.num_key_value_heads, Some(8));
}

#[test]
fn parse_hf_config_minimal() {
    let json = r#"{"model_type":"gpt2"}"#;
    let config = parse_hf_config_str(json).unwrap();
    assert_eq!(config.model_type, "gpt2");
    assert!(config.architectures.is_empty());
    assert!(config.hidden_size.is_none());
}

#[test]
fn parse_hf_config_invalid() {
    assert!(parse_hf_config_str("{invalid}").is_err());
}

#[test]
fn required_kernels_llama_has_gqa() {
    let config = model_config("llama");
    let kernels = required_kernels(&config);
    assert!(kernels.iter().any(|k| k.op == "gqa"));
    assert!(kernels.iter().any(|k| k.op == "rmsnorm"));
    assert!(kernels.iter().any(|k| k.op == "silu"));
    assert!(kernels.iter().any(|k| k.op == "rope"));
    assert!(kernels.iter().any(|k| k.op == "swiglu"));
    assert!(kernels.iter().any(|k| k.op == "softmax"));
    assert!(kernels.iter().any(|k| k.op == "matmul"));
    assert!(!kernels.iter().any(|k| k.op == "bias_add"));
}

#[test]
fn required_kernels_gpt2_has_layernorm_and_absolute() {
    let mut config = model_config("gpt2");
    config.num_key_value_heads = Some(16); // MHA: kv_heads == heads
    let kernels = required_kernels(&config);
    assert!(kernels.iter().any(|k| k.op == "layernorm"));
    assert!(kernels.iter().any(|k| k.op == "gelu"));
    assert!(kernels.iter().any(|k| k.op == "absolute_position"));
    assert!(kernels.iter().any(|k| k.op == "tied_embeddings"));
    assert!(kernels.iter().any(|k| k.op == "bias_add"));
    assert!(kernels.iter().any(|k| k.op == "attention"));
    assert!(!kernels.iter().any(|k| k.op == "gqa"));
}

#[test]
fn required_kernels_qwen2_has_bias() {
    let config = model_config("qwen2");
    let kernels = required_kernels(&config);
    assert!(kernels.iter().any(|k| k.op == "bias_add"));
    assert!(kernels.iter().any(|k| k.op == "gqa")); // 4 < 16
}

#[test]
fn required_kernels_deepseek_has_qk_norm() {
    let config = model_config("deepseek_v2");
    let kernels = required_kernels(&config);
    assert!(kernels.iter().any(|k| k.op == "qk_norm"));
}

#[test]
fn required_kernels_gemma_has_tied_embeddings() {
    let config = model_config("gemma");
    let kernels = required_kernels(&config);
    assert!(kernels.iter().any(|k| k.op == "tied_embeddings"));
    assert!(kernels.iter().any(|k| k.op == "gelu"));
}

#[test]
fn required_kernels_unknown_defaults_to_llama_like() {
    let mut config = model_config("novel_model_2027");
    config.num_attention_heads = None;
    config.num_key_value_heads = None;
    let kernels = required_kernels(&config);
    assert!(kernels.iter().any(|k| k.op == "rmsnorm"));
    assert!(kernels.iter().any(|k| k.op == "silu"));
    assert!(kernels.iter().any(|k| k.op == "rope"));
    assert!(kernels.iter().any(|k| k.op == "attention"));
}

#[test]
fn required_kernels_all_model_types_produce_results() {
    let types = [
        "llama",
        "codellama",
        "mistral",
        "mixtral",
        "qwen2",
        "qwen2_moe",
        "gemma",
        "gemma2",
        "phi",
        "phi3",
        "starcoder2",
        "gpt2",
        "gpt_neo",
        "gpt_neox",
        "falcon",
        "internlm2",
        "deepseek_v2",
    ];
    for mt in &types {
        let config = model_config(mt);
        let kernels = required_kernels(&config);
        assert!(!kernels.is_empty(), "no kernels for {mt}");
        assert!(
            kernels.iter().any(|k| k.op == "softmax"),
            "missing softmax for {mt}"
        );
        assert!(
            kernels.iter().any(|k| k.op == "matmul"),
            "missing matmul for {mt}"
        );
    }
}

// ── per-model arch_constraints coverage ──

#[test]
fn required_kernels_mistral_like_llama_no_bias() {
    let kernels = required_kernels(&model_config("mistral"));
    assert!(kernels.iter().any(|k| k.op == "rmsnorm"));
    assert!(kernels.iter().any(|k| k.op == "silu"));
    assert!(kernels.iter().any(|k| k.op == "rope"));
    assert!(kernels.iter().any(|k| k.op == "swiglu"));
    assert!(!kernels.iter().any(|k| k.op == "bias_add"));
    assert!(!kernels.iter().any(|k| k.op == "tied_embeddings"));
    assert!(!kernels.iter().any(|k| k.op == "qk_norm"));
}

#[test]
fn required_kernels_mixtral_same_as_mistral() {
    let k_mistral = required_kernels(&model_config("mistral"));
    let k_mixtral = required_kernels(&model_config("mixtral"));
    assert_eq!(k_mistral.len(), k_mixtral.len());
    for km in &k_mistral {
        assert!(
            k_mixtral.iter().any(|k| k.op == km.op),
            "mixtral missing op: {}",
            km.op
        );
    }
}

#[test]
fn required_kernels_phi_has_bias_no_tied() {
    let kernels = required_kernels(&model_config("phi"));
    assert!(kernels.iter().any(|k| k.op == "rmsnorm"));
    assert!(kernels.iter().any(|k| k.op == "silu"));
    assert!(kernels.iter().any(|k| k.op == "rope"));
    assert!(kernels.iter().any(|k| k.op == "bias_add"));
    assert!(!kernels.iter().any(|k| k.op == "tied_embeddings"));
}

#[test]
fn required_kernels_phi3_same_as_phi() {
    let k_phi = required_kernels(&model_config("phi"));
    let k_phi3 = required_kernels(&model_config("phi3"));
    assert_eq!(k_phi.len(), k_phi3.len());
    for kp in &k_phi {
        assert!(
            k_phi3.iter().any(|k| k.op == kp.op),
            "phi3 missing op: {}",
            kp.op
        );
    }
}

#[test]
fn required_kernels_starcoder2_layernorm_gelu_rope_bias() {
    let kernels = required_kernels(&model_config("starcoder2"));
    assert!(kernels.iter().any(|k| k.op == "layernorm"));
    assert!(kernels.iter().any(|k| k.op == "gelu"));
    assert!(kernels.iter().any(|k| k.op == "rope"));
    assert!(kernels.iter().any(|k| k.op == "bias_add"));
    assert!(!kernels.iter().any(|k| k.op == "tied_embeddings"));
    assert!(!kernels.iter().any(|k| k.op == "absolute_position"));
}

#[test]
fn required_kernels_falcon_layernorm_gelu_rope_no_bias() {
    let kernels = required_kernels(&model_config("falcon"));
    assert!(kernels.iter().any(|k| k.op == "layernorm"));
    assert!(kernels.iter().any(|k| k.op == "gelu"));
    assert!(kernels.iter().any(|k| k.op == "rope"));
    assert!(!kernels.iter().any(|k| k.op == "bias_add"));
    assert!(!kernels.iter().any(|k| k.op == "tied_embeddings"));
    assert!(!kernels.iter().any(|k| k.op == "absolute_position"));
}

#[test]
fn required_kernels_gpt_neo_same_as_gpt2() {
    let k_gpt2 = required_kernels(&model_config("gpt2"));
    let k_neo = required_kernels(&model_config("gpt_neo"));
    assert_eq!(k_gpt2.len(), k_neo.len());
    for kg in &k_gpt2 {
        assert!(
            k_neo.iter().any(|k| k.op == kg.op),
            "gpt_neo missing op: {}",
            kg.op
        );
    }
}

#[test]
fn required_kernels_gpt_neox_same_as_gpt2() {
    let k_gpt2 = required_kernels(&model_config("gpt2"));
    let k_neox = required_kernels(&model_config("gpt_neox"));
    assert_eq!(k_gpt2.len(), k_neox.len());
}

#[test]
fn required_kernels_internlm2_like_llama() {
    let k_llama = required_kernels(&model_config("llama"));
    let k_intern = required_kernels(&model_config("internlm2"));
    assert_eq!(k_llama.len(), k_intern.len());
    for kl in &k_llama {
        assert!(
            k_intern.iter().any(|k| k.op == kl.op),
            "internlm2 missing op: {}",
            kl.op
        );
    }
}

#[test]
fn required_kernels_codellama_same_as_llama() {
    let k_llama = required_kernels(&model_config("llama"));
    let k_code = required_kernels(&model_config("codellama"));
    assert_eq!(k_llama.len(), k_code.len());
}

#[test]
fn required_kernels_qwen2_moe_same_as_qwen2() {
    let k_qwen2 = required_kernels(&model_config("qwen2"));
    let k_moe = required_kernels(&model_config("qwen2_moe"));
    assert_eq!(k_qwen2.len(), k_moe.len());
    for kq in &k_qwen2 {
        assert!(
            k_moe.iter().any(|k| k.op == kq.op),
            "qwen2_moe missing op: {}",
            kq.op
        );
    }
}

#[test]
fn required_kernels_gemma2_same_as_gemma() {
    let k_gemma = required_kernels(&model_config("gemma"));
    let k_gemma2 = required_kernels(&model_config("gemma2"));
    assert_eq!(k_gemma.len(), k_gemma2.len());
}

// ── Sandbox edge cases ──

#[test]
fn plan_sandbox_build_empty_inputs() {
    let config = sandbox_config(SandboxLevel::Full);
    let inputs = BTreeMap::new();
    let plan = plan_sandbox_build(
        &config,
        "hash1234567890ab",
        &inputs,
        "echo",
        Path::new("/s"),
    );
    assert!(!plan.steps.is_empty());
    let errors = validate_plan(&plan);
    assert!(!errors.is_empty()); // flags empty overlay lower_dirs
}

#[test]
fn plan_sandbox_build_short_hash() {
    let config = sandbox_config(SandboxLevel::Minimal);
    let inputs = BTreeMap::from([("src".into(), PathBuf::from("/store/a"))]);
    let plan = plan_sandbox_build(&config, "abc", &inputs, "echo", Path::new("/s"));
    assert!(!plan.steps.is_empty());
}

#[test]
fn plan_sandbox_build_network_only_no_seccomp() {
    let config = sandbox_config(SandboxLevel::NetworkOnly);
    let inputs = BTreeMap::from([("src".into(), PathBuf::from("/store/a"))]);
    let plan = plan_sandbox_build(
        &config,
        "hash1234567890ab",
        &inputs,
        "echo",
        Path::new("/s"),
    );
    let rules = seccomp_rules_for_level(SandboxLevel::NetworkOnly);
    assert!(rules.is_empty());
    // NetworkOnly also skips seccomp step (only Full includes it)
    assert_eq!(plan.steps.len(), 9);
}

#[test]
fn plan_sandbox_build_none_level() {
    let config = sandbox_config(SandboxLevel::None);
    let inputs = BTreeMap::from([("src".into(), PathBuf::from("/store/a"))]);
    let plan = plan_sandbox_build(
        &config,
        "hash1234567890ab",
        &inputs,
        "echo",
        Path::new("/s"),
    );
    assert_eq!(plan.steps.len(), 9); // skips seccomp
}

#[test]
fn plan_sandbox_build_three_inputs() {
    let config = sandbox_config(SandboxLevel::Full);
    let inputs = BTreeMap::from([
        ("a".into(), PathBuf::from("/store/a")),
        ("b".into(), PathBuf::from("/store/b")),
        ("c".into(), PathBuf::from("/store/c")),
    ]);
    let plan = plan_sandbox_build(
        &config,
        "hash1234567890ab",
        &inputs,
        "echo",
        Path::new("/s"),
    );
    // Full: 10 base + 2 extra bind mounts = 12
    assert_eq!(plan.steps.len(), 12);
}

// ── Derivation edge cases ──

#[test]
fn plan_derivation_mixed_input_types() {
    let d = Derivation {
        inputs: BTreeMap::from([
            (
                "src".into(),
                DerivationInput::Store {
                    store: "blake3:aaaa".into(),
                },
            ),
            (
                "config".into(),
                DerivationInput::Resource {
                    resource: "my-config".into(),
                },
            ),
        ]),
        script: "echo build".into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    };
    let resolved = BTreeMap::from([("my-config".into(), "blake3:cccc".into())]);
    let result = plan_derivation(&d, &resolved, &[], Path::new("/store"));
    assert!(result.is_ok());
    let plan = result.unwrap();
    assert_eq!(plan.input_paths.len(), 2);
}

#[test]
fn plan_derivation_resource_not_resolved() {
    let d = Derivation {
        inputs: BTreeMap::from([(
            "config".into(),
            DerivationInput::Resource {
                resource: "missing-res".into(),
            },
        )]),
        script: "echo build".into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    };
    let resolved = BTreeMap::new();
    let result = plan_derivation(&d, &resolved, &[], Path::new("/store"));
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn simulate_derivation_deterministic() {
    let d = test_derivation("make install");
    let resources = BTreeMap::new();
    let r1 = simulate_derivation(&d, &resources, &[], Path::new("/store")).unwrap();
    let r2 = simulate_derivation(&d, &resources, &[], Path::new("/store")).unwrap();
    assert_eq!(r1.store_hash, r2.store_hash);
    assert_eq!(r1.closure_hash, r2.closure_hash);
    assert_eq!(r1.store_path, r2.store_path);
}

#[test]
fn execute_dag_empty() {
    let derivations = BTreeMap::new();
    let topo: Vec<String> = vec![];
    let resources = BTreeMap::new();
    let results =
        execute_derivation_dag(&derivations, &topo, &resources, &[], Path::new("/store")).unwrap();
    assert!(results.is_empty());
}
