//! FJ-1350/2604/2105/1352: HF config, mutation operators, registry push, contract scaffold.
//!
//! Popperian rejection criteria for:
//! - FJ-1350: parse_hf_config_str (model parsing, field extraction)
//! - FJ-1350: required_kernels (architecture → kernel mapping)
//! - FJ-2604: mutation_script (operator → shell script generation)
//! - FJ-2604: applicable_operators (resource type → operator filtering)
//! - FJ-2604: MutationOperator (description, applicable_types, Display)
//! - FJ-2105: validate_push_config (required fields, URL rejection)
//! - FJ-2105: format_push_summary (uploaded/skipped counts, bytes)
//! - FJ-2105: head_check_command, upload/manifest command generation
//! - FJ-1352: scaffold_contracts (YAML stub generation)
//!
//! Usage: cargo test --test falsification_hf_mutation_registry

use forjar::core::store::contract_scaffold::scaffold_contracts;
use forjar::core::store::hf_config::{
    parse_hf_config_str, required_kernels, HfModelConfig, KernelRequirement,
};
use forjar::core::store::mutation_runner::{applicable_operators, mutation_script};
use forjar::core::store::registry_push::{
    format_push_summary, head_check_command, manifest_put_command, upload_complete_command,
    upload_initiate_command, validate_push_config, RegistryPushConfig,
};
use forjar::core::types::{MutationOperator, PushKind, PushResult};

// ============================================================================
// FJ-1350: parse_hf_config_str — model config parsing
// ============================================================================

#[test]
fn parse_llama_config() {
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
fn parse_qwen2_config() {
    let json = r#"{
        "model_type": "qwen2",
        "architectures": ["Qwen2ForCausalLM"],
        "hidden_size": 3584,
        "num_attention_heads": 28,
        "num_key_value_heads": 4,
        "num_hidden_layers": 28,
        "vocab_size": 152064
    }"#;
    let config = parse_hf_config_str(json).unwrap();
    assert_eq!(config.model_type, "qwen2");
    assert_eq!(config.num_key_value_heads, Some(4));
}

#[test]
fn parse_minimal_config() {
    let json = r#"{"model_type": "unknown"}"#;
    let config = parse_hf_config_str(json).unwrap();
    assert_eq!(config.model_type, "unknown");
    assert!(config.hidden_size.is_none());
    assert!(config.architectures.is_empty());
}

#[test]
fn parse_invalid_json_rejected() {
    assert!(parse_hf_config_str("not json").is_err());
}

// ============================================================================
// FJ-1350: required_kernels — architecture → kernel contract mapping
// ============================================================================

#[test]
fn llama_kernels_include_rmsnorm_silu_rope() {
    let config = HfModelConfig {
        model_type: "llama".into(),
        architectures: vec!["LlamaForCausalLM".into()],
        hidden_size: Some(4096),
        num_attention_heads: Some(32),
        num_key_value_heads: Some(8),
        num_hidden_layers: Some(32),
        intermediate_size: Some(11008),
        vocab_size: Some(32000),
        max_position_embeddings: Some(4096),
    };
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"rmsnorm"), "llama needs rmsnorm");
    assert!(ops.contains(&"silu"), "llama needs silu");
    assert!(ops.contains(&"rope"), "llama needs rope");
    assert!(ops.contains(&"swiglu"), "llama needs swiglu");
    assert!(ops.contains(&"softmax"), "universal kernel");
    assert!(ops.contains(&"matmul"), "universal kernel");
}

#[test]
fn llama_gqa_detected() {
    let config = HfModelConfig {
        model_type: "llama".into(),
        architectures: vec![],
        hidden_size: None,
        num_attention_heads: Some(32),
        num_key_value_heads: Some(8), // < 32 → GQA
        num_hidden_layers: None,
        intermediate_size: None,
        vocab_size: None,
        max_position_embeddings: None,
    };
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"gqa"), "kv_heads < heads → GQA");
    assert!(
        !ops.contains(&"attention"),
        "GQA replaces standard attention"
    );
}

#[test]
fn gpt2_uses_layernorm_gelu_absolute() {
    let config = HfModelConfig {
        model_type: "gpt2".into(),
        architectures: vec![],
        hidden_size: Some(768),
        num_attention_heads: Some(12),
        num_key_value_heads: None,
        num_hidden_layers: Some(12),
        intermediate_size: None,
        vocab_size: Some(50257),
        max_position_embeddings: Some(1024),
    };
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"layernorm"));
    assert!(ops.contains(&"gelu"));
    assert!(ops.contains(&"absolute_position"));
    assert!(ops.contains(&"tied_embeddings"));
    assert!(ops.contains(&"bias_add"));
}

#[test]
fn deepseek_v2_has_qk_norm() {
    let config = HfModelConfig {
        model_type: "deepseek_v2".into(),
        architectures: vec![],
        hidden_size: None,
        num_attention_heads: Some(128),
        num_key_value_heads: Some(16),
        num_hidden_layers: None,
        intermediate_size: None,
        vocab_size: None,
        max_position_embeddings: None,
    };
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"qk_norm"));
}

#[test]
fn gemma_has_tied_embeddings() {
    let config = HfModelConfig {
        model_type: "gemma".into(),
        architectures: vec![],
        hidden_size: None,
        num_attention_heads: Some(16),
        num_key_value_heads: Some(16), // equal → MHA not GQA
        num_hidden_layers: None,
        intermediate_size: None,
        vocab_size: None,
        max_position_embeddings: None,
    };
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"tied_embeddings"));
    assert!(ops.contains(&"attention"), "equal heads → MHA");
}

#[test]
fn unknown_model_defaults_to_llama_like() {
    let config = HfModelConfig {
        model_type: "custom_model_xyz".into(),
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
    assert!(ops.contains(&"rmsnorm"), "default is rmsnorm");
    assert!(ops.contains(&"silu"), "default is silu");
}

// ============================================================================
// FJ-2604: mutation_script — operator → shell script
// ============================================================================

#[test]
fn mutation_script_delete_file() {
    let script = mutation_script(MutationOperator::DeleteFile, "nginx.conf");
    assert!(script.contains("rm -f"));
    assert!(script.contains("nginx.conf"));
    assert!(script.contains("FORJAR_SANDBOX"));
}

#[test]
fn mutation_script_stop_service() {
    let script = mutation_script(MutationOperator::StopService, "nginx");
    assert!(script.contains("systemctl stop"));
    assert!(script.contains("nginx"));
}

#[test]
fn mutation_script_remove_package() {
    let script = mutation_script(MutationOperator::RemovePackage, "curl");
    assert!(script.contains("apt-get remove"));
    assert!(script.contains("curl"));
}

#[test]
fn mutation_script_corrupt_config() {
    let script = mutation_script(MutationOperator::CorruptConfig, "app.conf");
    assert!(script.contains("sed"));
    assert!(script.contains("CORRUPTED"));
}

// ============================================================================
// FJ-2604: applicable_operators — resource type filtering
// ============================================================================

#[test]
fn file_operators() {
    let ops = applicable_operators("file");
    assert!(ops.contains(&MutationOperator::DeleteFile));
    assert!(ops.contains(&MutationOperator::ModifyContent));
    assert!(ops.contains(&MutationOperator::ChangePermissions));
    assert!(ops.contains(&MutationOperator::CorruptConfig));
    assert!(!ops.contains(&MutationOperator::StopService));
}

#[test]
fn service_operators() {
    let ops = applicable_operators("service");
    assert!(ops.contains(&MutationOperator::StopService));
    assert!(ops.contains(&MutationOperator::KillProcess));
    assert!(!ops.contains(&MutationOperator::DeleteFile));
}

#[test]
fn package_operators() {
    let ops = applicable_operators("package");
    assert!(ops.contains(&MutationOperator::RemovePackage));
    assert!(!ops.contains(&MutationOperator::StopService));
}

#[test]
fn mount_operators() {
    let ops = applicable_operators("mount");
    assert!(ops.contains(&MutationOperator::UnmountFilesystem));
    assert_eq!(ops.len(), 1);
}

#[test]
fn unknown_type_no_operators() {
    let ops = applicable_operators("pepita");
    assert!(ops.is_empty());
}

// ============================================================================
// FJ-2604: MutationOperator — description and Display
// ============================================================================

#[test]
fn operator_descriptions_nonempty() {
    let all = [
        MutationOperator::DeleteFile,
        MutationOperator::ModifyContent,
        MutationOperator::ChangePermissions,
        MutationOperator::StopService,
        MutationOperator::RemovePackage,
        MutationOperator::KillProcess,
        MutationOperator::UnmountFilesystem,
        MutationOperator::CorruptConfig,
    ];
    for op in all {
        assert!(!op.description().is_empty(), "{op} description empty");
        assert!(!op.to_string().is_empty(), "{op} Display empty");
    }
}

// ============================================================================
// FJ-2105: validate_push_config
// ============================================================================

#[test]
fn push_config_valid() {
    let config = RegistryPushConfig {
        registry: "ghcr.io".into(),
        name: "myorg/myapp".into(),
        tag: "v1.0".into(),
        check_existing: true,
    };
    assert!(validate_push_config(&config).is_empty());
}

#[test]
fn push_config_empty_registry() {
    let config = RegistryPushConfig {
        registry: "".into(),
        name: "myapp".into(),
        tag: "v1".into(),
        check_existing: false,
    };
    let errors = validate_push_config(&config);
    assert!(errors.iter().any(|e| e.contains("registry")));
}

#[test]
fn push_config_empty_name() {
    let config = RegistryPushConfig {
        registry: "ghcr.io".into(),
        name: "".into(),
        tag: "latest".into(),
        check_existing: false,
    };
    let errors = validate_push_config(&config);
    assert!(errors.iter().any(|e| e.contains("name")));
}

#[test]
fn push_config_url_rejected() {
    let config = RegistryPushConfig {
        registry: "https://ghcr.io".into(),
        name: "myapp".into(),
        tag: "v1".into(),
        check_existing: false,
    };
    let errors = validate_push_config(&config);
    assert!(errors.iter().any(|e| e.contains("hostname")));
}

// ============================================================================
// FJ-2105: command generation
// ============================================================================

#[test]
fn head_check_command_format() {
    let cmd = head_check_command("ghcr.io", "myorg/myapp", "sha256:abc123");
    assert!(cmd.contains("--head"));
    assert!(cmd.contains("ghcr.io"));
    assert!(cmd.contains("sha256:abc123"));
}

#[test]
fn upload_initiate_command_format() {
    let cmd = upload_initiate_command("ghcr.io", "myorg/myapp");
    assert!(cmd.contains("POST"));
    assert!(cmd.contains("/v2/myorg/myapp/blobs/uploads/"));
}

#[test]
fn upload_complete_command_format() {
    let cmd = upload_complete_command("https://ghcr.io/upload/1234", "sha256:abc", "/tmp/blob.tar");
    assert!(cmd.contains("PUT"));
    assert!(cmd.contains("sha256:abc"));
    assert!(cmd.contains("/tmp/blob.tar"));
}

#[test]
fn manifest_put_command_format() {
    let cmd = manifest_put_command("ghcr.io", "myapp", "v1.0", "/tmp/manifest.json");
    assert!(cmd.contains("PUT"));
    assert!(cmd.contains("manifests/v1.0"));
    assert!(cmd.contains("application/vnd.oci.image.manifest"));
}

// ============================================================================
// FJ-2105: format_push_summary
// ============================================================================

#[test]
fn push_summary_all_uploaded() {
    let results = vec![
        PushResult {
            kind: PushKind::Layer,
            digest: "sha256:aaa".into(),
            size: 1024 * 1024,
            existed: false,
            duration_secs: 1.5,
        },
        PushResult {
            kind: PushKind::Config,
            digest: "sha256:bbb".into(),
            size: 512,
            existed: false,
            duration_secs: 0.1,
        },
    ];
    let summary = format_push_summary(&results);
    assert!(summary.contains("2 uploaded"));
    assert!(summary.contains("0 skipped"));
    assert!(summary.contains("layer"));
    assert!(summary.contains("config"));
}

#[test]
fn push_summary_with_skipped() {
    let results = vec![PushResult {
        kind: PushKind::Layer,
        digest: "sha256:ccc".into(),
        size: 2048,
        existed: true,
        duration_secs: 0.0,
    }];
    let summary = format_push_summary(&results);
    assert!(summary.contains("0 uploaded"));
    assert!(summary.contains("1 skipped"));
    assert!(summary.contains("[skip]"));
}

// ============================================================================
// FJ-1352: scaffold_contracts — YAML stub generation
// ============================================================================

#[test]
fn scaffold_single_contract() {
    let missing = vec![KernelRequirement {
        op: "softmax".into(),
        contract: "softmax-kernel-v1".into(),
    }];
    let stubs = scaffold_contracts(&missing, "forjar-ci");
    assert_eq!(stubs.len(), 1);
    assert_eq!(stubs[0].filename, "softmax-kernel-v1.yaml");
    assert!(stubs[0].yaml_content.contains("softmax"));
    assert!(stubs[0].yaml_content.contains("forjar-ci"));
    assert!(stubs[0].yaml_content.contains("EQ-SOFTMAX-01"));
    assert!(stubs[0].yaml_content.contains("PO-SOFTMAX-01"));
    assert!(stubs[0].yaml_content.contains("FALSIFY-SOFTMAX-001"));
}

#[test]
fn scaffold_multiple_contracts() {
    let missing = vec![
        KernelRequirement {
            op: "matmul".into(),
            contract: "matmul-kernel-v1".into(),
        },
        KernelRequirement {
            op: "rope".into(),
            contract: "rope-kernel-v1".into(),
        },
    ];
    let stubs = scaffold_contracts(&missing, "team");
    assert_eq!(stubs.len(), 2);
    assert!(stubs[0].yaml_content.contains("MATMUL"));
    assert!(stubs[1].yaml_content.contains("ROPE"));
}

#[test]
fn scaffold_empty_input() {
    let stubs = scaffold_contracts(&[], "author");
    assert!(stubs.is_empty());
}
