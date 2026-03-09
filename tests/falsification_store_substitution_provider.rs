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

#[test]
fn substitution_local_hit() {
    let cache_config = CacheConfig {
        sources: vec![],
        auto_push: false,
        max_size_mb: 0,
    };
    let ctx = SubstitutionContext {
        closure_hash: "blake3:abc123",
        input_hashes: &["h1".into(), "h2".into()],
        local_entries: &["blake3:abc123".into()],
        cache_config: &cache_config,
        cache_inventories: &[],
        sandbox: None,
        store_dir: std::path::Path::new("/var/lib/forjar/store"),
    };
    let plan = plan_substitution(&ctx);
    assert!(matches!(plan.outcome, SubstitutionOutcome::LocalHit { .. }));
    assert!(!requires_build(&plan));
    assert!(!requires_pull(&plan));
}

// ============================================================================
// FJ-1322: plan_substitution — cache hit
// ============================================================================

#[test]
fn substitution_cache_hit() {
    let cache_config = CacheConfig {
        sources: vec![CacheSource::Ssh {
            host: "cache.example.com".into(),
            user: "forjar".into(),
            path: "/cache".into(),
            port: None,
        }],
        auto_push: false,
        max_size_mb: 0,
    };

    let mut entries = BTreeMap::new();
    entries.insert(
        "blake3:def456".into(),
        forjar::core::store::cache::CacheEntry {
            store_hash: "blake3:def456".into(),
            size_bytes: 1024,
            created_at: "2026-03-09T00:00:00Z".into(),
            provider: "apt".into(),
            arch: "x86_64".into(),
        },
    );
    let inventory = CacheInventory {
        source_name: "cache.example.com".into(),
        entries,
    };

    let ctx = SubstitutionContext {
        closure_hash: "blake3:def456",
        input_hashes: &["h1".into()],
        local_entries: &[], // not local
        cache_config: &cache_config,
        cache_inventories: &[inventory],
        sandbox: None,
        store_dir: std::path::Path::new("/store"),
    };
    let plan = plan_substitution(&ctx);
    assert!(matches!(plan.outcome, SubstitutionOutcome::CacheHit { .. }));
    assert!(!requires_build(&plan));
    assert!(requires_pull(&plan));
}

// ============================================================================
// FJ-1322: plan_substitution — cache miss
// ============================================================================

#[test]
fn substitution_cache_miss() {
    let cache_config = CacheConfig {
        sources: vec![],
        auto_push: false,
        max_size_mb: 0,
    };
    let ctx = SubstitutionContext {
        closure_hash: "blake3:missing",
        input_hashes: &["h1".into()],
        local_entries: &[],
        cache_config: &cache_config,
        cache_inventories: &[],
        sandbox: None,
        store_dir: std::path::Path::new("/store"),
    };
    let plan = plan_substitution(&ctx);
    assert!(matches!(
        plan.outcome,
        SubstitutionOutcome::CacheMiss { .. }
    ));
    assert!(requires_build(&plan));
    assert!(!requires_pull(&plan));
}

#[test]
fn substitution_cache_miss_with_sandbox() {
    let cache_config = CacheConfig {
        sources: vec![],
        auto_push: false,
        max_size_mb: 0,
    };
    let sandbox = SandboxConfig {
        level: SandboxLevel::Full,
        memory_mb: 2048,
        cpus: 4.0,
        timeout: 600,
        bind_mounts: vec![],
        env: vec![],
    };
    let ctx = SubstitutionContext {
        closure_hash: "blake3:sandboxed",
        input_hashes: &["h1".into()],
        local_entries: &[],
        cache_config: &cache_config,
        cache_inventories: &[],
        sandbox: Some(&sandbox),
        store_dir: std::path::Path::new("/store"),
    };
    let plan = plan_substitution(&ctx);
    assert!(requires_build(&plan));
    // Should include sandbox level in build step
    assert!(step_count(&plan) >= 3); // compute + check_local + build + store
}

#[test]
fn substitution_cache_miss_auto_push() {
    let cache_config = CacheConfig {
        sources: vec![CacheSource::Ssh {
            host: "cache.example.com".into(),
            user: "forjar".into(),
            path: "/cache".into(),
            port: None,
        }],
        auto_push: true,
        max_size_mb: 0,
    };
    let ctx = SubstitutionContext {
        closure_hash: "blake3:autopush",
        input_hashes: &["h1".into()],
        local_entries: &[],
        cache_config: &cache_config,
        cache_inventories: &[CacheInventory {
            source_name: "cache".into(),
            entries: BTreeMap::new(),
        }],
        sandbox: None,
        store_dir: std::path::Path::new("/store"),
    };
    let plan = plan_substitution(&ctx);
    assert!(requires_build(&plan));
    // Should include push step at the end
    let has_push = plan.steps.iter().any(|s| {
        matches!(
            s,
            forjar::core::store::substitution::SubstitutionStep::PushToCache { .. }
        )
    });
    assert!(has_push, "auto_push should generate PushToCache step");
}

// ============================================================================
// FJ-1322: step_count
// ============================================================================

#[test]
fn substitution_step_count_local_hit() {
    let cache_config = CacheConfig {
        sources: vec![],
        auto_push: false,
        max_size_mb: 0,
    };
    let ctx = SubstitutionContext {
        closure_hash: "blake3:local",
        input_hashes: &["h1".into()],
        local_entries: &["blake3:local".into()],
        cache_config: &cache_config,
        cache_inventories: &[],
        sandbox: None,
        store_dir: std::path::Path::new("/store"),
    };
    let plan = plan_substitution(&ctx);
    assert_eq!(step_count(&plan), 2); // compute_closure_hash + check_local_store
}

// ============================================================================
// FJ-1333: import_command
// ============================================================================

#[test]
fn provider_import_command_apt() {
    let config = ImportConfig {
        provider: ImportProvider::Apt,
        reference: "nginx".into(),
        version: Some("1.24.0".into()),
        arch: "x86_64".into(),
        options: BTreeMap::new(),
    };
    let cmd = import_command(&config);
    assert!(cmd.contains("apt-get install"));
    assert!(cmd.contains("nginx=1.24.0"));
}

#[test]
fn provider_import_command_apt_no_version() {
    let config = ImportConfig {
        provider: ImportProvider::Apt,
        reference: "curl".into(),
        version: None,
        arch: "x86_64".into(),
        options: BTreeMap::new(),
    };
    let cmd = import_command(&config);
    assert!(cmd.contains("curl"));
    assert!(!cmd.contains("="));
}

#[test]
fn provider_import_command_cargo() {
    let config = ImportConfig {
        provider: ImportProvider::Cargo,
        reference: "ripgrep".into(),
        version: Some("14.0.0".into()),
        arch: "x86_64".into(),
        options: BTreeMap::new(),
    };
    let cmd = import_command(&config);
    assert!(cmd.contains("cargo install"));
    assert!(cmd.contains("--version 14.0.0"));
    assert!(cmd.contains("ripgrep"));
}

#[test]
fn provider_import_command_uv() {
    let config = ImportConfig {
        provider: ImportProvider::Uv,
        reference: "numpy".into(),
        version: Some("1.26.0".into()),
        arch: "x86_64".into(),
        options: BTreeMap::new(),
    };
    let cmd = import_command(&config);
    assert!(cmd.contains("uv pip install"));
    assert!(cmd.contains("numpy==1.26.0"));
}

#[test]
fn provider_import_command_nix() {
    let config = ImportConfig {
        provider: ImportProvider::Nix,
        reference: "nixpkgs#ripgrep".into(),
        version: None,
        arch: "x86_64".into(),
        options: BTreeMap::new(),
    };
    let cmd = import_command(&config);
    assert!(cmd.contains("nix build"));
    assert!(cmd.contains("nixpkgs#ripgrep"));
}

#[test]
fn provider_import_command_docker() {
    let config = ImportConfig {
        provider: ImportProvider::Docker,
        reference: "nginx".into(),
        version: Some("1.24".into()),
        arch: "x86_64".into(),
        options: BTreeMap::new(),
    };
    let cmd = import_command(&config);
    assert!(cmd.contains("docker create nginx:1.24"));
}

#[test]
fn provider_import_command_tofu() {
    let config = ImportConfig {
        provider: ImportProvider::Tofu,
        reference: "./infra/prod".into(),
        version: None,
        arch: "x86_64".into(),
        options: BTreeMap::new(),
    };
    let cmd = import_command(&config);
    assert!(cmd.contains("tofu -chdir=./infra/prod output -json"));
}

#[test]
fn provider_import_command_terraform() {
    let config = ImportConfig {
        provider: ImportProvider::Terraform,
        reference: "./infra".into(),
        version: None,
        arch: "x86_64".into(),
        options: BTreeMap::new(),
    };
    let cmd = import_command(&config);
    assert!(cmd.contains("terraform -chdir=./infra output -json"));
}

#[test]
fn provider_import_command_apr() {
    let config = ImportConfig {
        provider: ImportProvider::Apr,
        reference: "llama-3.1-8b-q4".into(),
        version: None,
        arch: "x86_64".into(),
        options: BTreeMap::new(),
    };
    let cmd = import_command(&config);
    assert!(cmd.contains("apr pull llama-3.1-8b-q4"));
}

// ============================================================================
// FJ-1333: origin_ref_string
// ============================================================================

#[test]
fn provider_origin_ref_with_version() {
    let config = ImportConfig {
        provider: ImportProvider::Apt,
        reference: "nginx".into(),
        version: Some("1.24".into()),
        arch: "x86_64".into(),
        options: BTreeMap::new(),
    };
    assert_eq!(origin_ref_string(&config), "apt:nginx@1.24");
}

#[test]
fn provider_origin_ref_without_version() {
    let config = ImportConfig {
        provider: ImportProvider::Cargo,
        reference: "serde".into(),
        version: None,
        arch: "x86_64".into(),
        options: BTreeMap::new(),
    };
    assert_eq!(origin_ref_string(&config), "cargo:serde");
}

#[test]
fn provider_origin_ref_nix_passthrough() {
    let config = ImportConfig {
        provider: ImportProvider::Nix,
        reference: "nixpkgs#hello".into(),
        version: None,
        arch: "x86_64".into(),
        options: BTreeMap::new(),
    };
    assert_eq!(origin_ref_string(&config), "nixpkgs#hello");
}

// ============================================================================
// FJ-1333: validate_import
// ============================================================================

#[test]
fn provider_validate_valid_config() {
    let config = ImportConfig {
        provider: ImportProvider::Apt,
        reference: "nginx".into(),
        version: None,
        arch: "x86_64".into(),
        options: BTreeMap::new(),
    };
    assert!(validate_import(&config).is_empty());
}

#[test]
fn provider_validate_empty_reference() {
    let config = ImportConfig {
        provider: ImportProvider::Apt,
        reference: "".into(),
        version: None,
        arch: "x86_64".into(),
        options: BTreeMap::new(),
    };
    let errors = validate_import(&config);
    assert!(errors.iter().any(|e| e.contains("reference")));
}

#[test]
fn provider_validate_empty_arch() {
    let config = ImportConfig {
        provider: ImportProvider::Apt,
        reference: "nginx".into(),
        version: None,
        arch: "".into(),
        options: BTreeMap::new(),
    };
    let errors = validate_import(&config);
    assert!(errors.iter().any(|e| e.contains("arch")));
}

#[test]
fn provider_validate_nix_missing_flake_hash() {
    let config = ImportConfig {
        provider: ImportProvider::Nix,
        reference: "hello".into(), // should be nixpkgs#hello
        version: None,
        arch: "x86_64".into(),
        options: BTreeMap::new(),
    };
    let errors = validate_import(&config);
    assert!(errors.iter().any(|e| e.contains("flake")));
}

#[test]
fn provider_validate_docker_no_spaces() {
    let config = ImportConfig {
        provider: ImportProvider::Docker,
        reference: "my image".into(),
        version: None,
        arch: "x86_64".into(),
        options: BTreeMap::new(),
    };
    let errors = validate_import(&config);
    assert!(errors.iter().any(|e| e.contains("spaces")));
}

// ============================================================================
// FJ-1333: parse_import_config
// ============================================================================

#[test]
fn provider_parse_yaml() {
    let yaml = r#"
provider: apt
reference: nginx
version: "1.24.0"
arch: x86_64
"#;
    let config = parse_import_config(yaml).unwrap();
    assert_eq!(config.provider, ImportProvider::Apt);
    assert_eq!(config.reference, "nginx");
    assert_eq!(config.version.as_deref(), Some("1.24.0"));
}

#[test]
fn provider_parse_yaml_invalid() {
    let result = parse_import_config("not: valid: yaml: {{");
    assert!(result.is_err());
}

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
