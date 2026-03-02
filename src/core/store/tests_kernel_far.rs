//! Tests for FJ-1353: Kernel contract FAR packaging and onboard pipeline.

use super::contract_coverage::{coverage_report, BindingEntry, BindingRegistry};
use super::far::{decode_far_manifest, FarManifest, KernelContractInfo};
use super::hf_config::{parse_hf_config_str, required_kernels};
use super::kernel_far::{contracts_to_far, onboard_model};

fn setup_contracts_dir(dir: &std::path::Path) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join("rmsnorm-kernel-v1.yaml"),
        "metadata:\n  name: rmsnorm-kernel-v1\n  version: '1.0.0'\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("silu-kernel-v1.yaml"),
        "metadata:\n  name: silu-kernel-v1\n  version: '1.0.0'\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("softmax-kernel-v1.yaml"),
        "metadata:\n  name: softmax-kernel-v1\n  version: '1.0.0'\n",
    )
    .unwrap();
}

fn sample_binding_yaml() -> &'static str {
    concat!(
        "version: '1.0'\n",
        "target_crate: trueno\n",
        "bindings:\n",
        "  - contract: rmsnorm-kernel-v1\n",
        "    equation: EQ-RMSNORM-01\n",
        "    status: implemented\n",
        "  - contract: silu-kernel-v1\n",
        "    equation: EQ-SILU-01\n",
        "    status: implemented\n",
        "  - contract: softmax-kernel-v1\n",
        "    equation: EQ-SOFTMAX-01\n",
        "    status: implemented\n",
    )
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

#[test]
fn test_fj1353_contracts_to_far_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let contracts_dir = tmp.path().join("contracts");
    setup_contracts_dir(&contracts_dir);

    let config = parse_hf_config_str(llama_config_json()).unwrap();
    let required = required_kernels(&config);
    let registry = BindingRegistry {
        version: "1.0".to_string(),
        target_crate: "trueno".to_string(),
        bindings: vec![
            BindingEntry {
                contract: "rmsnorm-kernel-v1".to_string(),
                equation: "EQ-01".to_string(),
                status: "implemented".to_string(),
            },
            BindingEntry {
                contract: "silu-kernel-v1".to_string(),
                equation: "EQ-02".to_string(),
                status: "implemented".to_string(),
            },
            BindingEntry {
                contract: "softmax-kernel-v1".to_string(),
                equation: "EQ-03".to_string(),
                status: "implemented".to_string(),
            },
        ],
    };
    let available = vec![
        "rmsnorm-kernel-v1".to_string(),
        "silu-kernel-v1".to_string(),
        "softmax-kernel-v1".to_string(),
    ];
    let coverage = coverage_report("llama", &required, &registry, &available);

    let far_path = tmp.path().join("output.far");
    let manifest = contracts_to_far(&contracts_dir, &config, &coverage, &far_path).unwrap();

    // Verify FAR file exists and can be decoded
    let file = std::fs::File::open(&far_path).unwrap();
    let reader = std::io::BufReader::new(file);
    let (decoded, _entries) = decode_far_manifest(reader).unwrap();
    assert_eq!(decoded, manifest);
}

#[test]
fn test_fj1353_provenance_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let contracts_dir = tmp.path().join("contracts");
    setup_contracts_dir(&contracts_dir);

    let config = parse_hf_config_str(llama_config_json()).unwrap();
    let required = required_kernels(&config);
    let registry = BindingRegistry {
        version: "1.0".to_string(),
        target_crate: "trueno".to_string(),
        bindings: vec![],
    };
    let coverage = coverage_report("llama", &required, &registry, &[]);

    let far_path = tmp.path().join("output.far");
    let manifest = contracts_to_far(&contracts_dir, &config, &coverage, &far_path).unwrap();

    assert_eq!(manifest.provenance.origin_provider, "kernel-contracts");
    assert_eq!(manifest.provenance.origin_ref, Some("llama".to_string()));
    assert!(manifest.provenance.generator.starts_with("forjar "));
    assert!(!manifest.provenance.created_at.is_empty());
}

#[test]
fn test_fj1353_kernel_contracts_in_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    let contracts_dir = tmp.path().join("contracts");
    setup_contracts_dir(&contracts_dir);

    let config = parse_hf_config_str(llama_config_json()).unwrap();
    let required = required_kernels(&config);
    let registry = BindingRegistry {
        version: "1.0".to_string(),
        target_crate: "trueno".to_string(),
        bindings: vec![
            BindingEntry {
                contract: "rmsnorm-kernel-v1".to_string(),
                equation: "EQ-01".to_string(),
                status: "implemented".to_string(),
            },
            BindingEntry {
                contract: "silu-kernel-v1".to_string(),
                equation: "EQ-02".to_string(),
                status: "implemented".to_string(),
            },
            BindingEntry {
                contract: "softmax-kernel-v1".to_string(),
                equation: "EQ-03".to_string(),
                status: "implemented".to_string(),
            },
        ],
    };
    let available = vec![
        "rmsnorm-kernel-v1".to_string(),
        "silu-kernel-v1".to_string(),
        "softmax-kernel-v1".to_string(),
    ];
    let coverage = coverage_report("llama", &required, &registry, &available);

    let far_path = tmp.path().join("output.far");
    let manifest = contracts_to_far(&contracts_dir, &config, &coverage, &far_path).unwrap();

    let kc = manifest.kernel_contracts.unwrap();
    assert_eq!(kc.model_type, "llama");
    assert!(!kc.required_ops.is_empty());
    assert!(kc.coverage_pct >= 0.0);
}

#[test]
fn test_fj1353_backward_compat_none_kernel_contracts() {
    // A manifest without kernel_contracts should decode fine
    let manifest = FarManifest {
        name: "test".to_string(),
        version: "1.0".to_string(),
        arch: "x86_64".to_string(),
        store_hash: "blake3:0000".to_string(),
        tree_hash: "blake3:0000".to_string(),
        file_count: 0,
        total_size: 0,
        files: vec![],
        provenance: super::far::FarProvenance {
            origin_provider: "test".to_string(),
            origin_ref: None,
            origin_hash: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 1.0.0".to_string(),
        },
        kernel_contracts: None,
    };
    let yaml = serde_yaml_ng::to_string(&manifest).unwrap();
    // kernel_contracts should not appear in YAML when None
    assert!(!yaml.contains("kernel_contracts"));

    // Should deserialize back
    let parsed: FarManifest = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(parsed.kernel_contracts, None);
}

#[test]
fn test_fj1353_kernel_contract_info_serde() {
    let kc = KernelContractInfo {
        model_type: "qwen2".to_string(),
        required_ops: vec![
            "rmsnorm-kernel-v1".to_string(),
            "silu-kernel-v1".to_string(),
        ],
        coverage_pct: 75.5,
    };
    let yaml = serde_yaml_ng::to_string(&kc).unwrap();
    let parsed: KernelContractInfo = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(kc, parsed);
}

#[test]
fn test_fj1353_manifest_name_format() {
    let tmp = tempfile::tempdir().unwrap();
    let contracts_dir = tmp.path().join("contracts");
    setup_contracts_dir(&contracts_dir);

    let config = parse_hf_config_str(llama_config_json()).unwrap();
    let coverage = coverage_report(
        "llama",
        &[],
        &BindingRegistry {
            version: "1.0".to_string(),
            target_crate: "trueno".to_string(),
            bindings: vec![],
        },
        &[],
    );

    let far_path = tmp.path().join("output.far");
    let manifest = contracts_to_far(&contracts_dir, &config, &coverage, &far_path).unwrap();

    assert_eq!(manifest.name, "llama-kernel-contracts");
    assert_eq!(manifest.arch, "noarch");
}

#[test]
fn test_fj1353_deterministic_store_hash() {
    let tmp = tempfile::tempdir().unwrap();
    let contracts_dir = tmp.path().join("contracts");
    setup_contracts_dir(&contracts_dir);

    let config = parse_hf_config_str(llama_config_json()).unwrap();
    let coverage = coverage_report(
        "llama",
        &[],
        &BindingRegistry {
            version: "1.0".to_string(),
            target_crate: "trueno".to_string(),
            bindings: vec![],
        },
        &[],
    );

    let far1 = tmp.path().join("out1.far");
    let far2 = tmp.path().join("out2.far");
    let m1 = contracts_to_far(&contracts_dir, &config, &coverage, &far1).unwrap();
    let m2 = contracts_to_far(&contracts_dir, &config, &coverage, &far2).unwrap();

    assert_eq!(m1.store_hash, m2.store_hash);
    assert_eq!(m1.tree_hash, m2.tree_hash);
}

#[test]
fn test_fj1353_full_onboard_pipeline() {
    let tmp = tempfile::tempdir().unwrap();

    // Set up config.json
    let config_path = tmp.path().join("config.json");
    std::fs::write(&config_path, llama_config_json()).unwrap();

    // Set up contracts dir with some existing contracts
    let contracts_dir = tmp.path().join("contracts");
    setup_contracts_dir(&contracts_dir);

    // Set up binding.yaml
    let binding_path = tmp.path().join("binding.yaml");
    std::fs::write(&binding_path, sample_binding_yaml()).unwrap();

    // Output dir
    let output_dir = tmp.path().join("output");

    let result = onboard_model(&config_path, &contracts_dir, &binding_path, &output_dir).unwrap();

    // Config was parsed
    assert_eq!(result.config.model_type, "llama");

    // Coverage report exists
    assert_eq!(result.coverage.model_type, "llama");
    assert!(result.coverage.total_required > 0);

    // FAR was created
    assert_eq!(result.far_manifest.name, "llama-kernel-contracts");
    assert!(output_dir.join("llama-kernel-contracts.far").exists());

    // FAR is valid
    let file = std::fs::File::open(output_dir.join("llama-kernel-contracts.far")).unwrap();
    let reader = std::io::BufReader::new(file);
    let (decoded, _) = decode_far_manifest(reader).unwrap();
    assert_eq!(decoded, result.far_manifest);
}

#[test]
fn test_fj1353_onboard_with_gaps() {
    let tmp = tempfile::tempdir().unwrap();

    // Config for a model that needs many kernels
    let config_path = tmp.path().join("config.json");
    std::fs::write(&config_path, llama_config_json()).unwrap();

    // Empty contracts dir — everything is a gap
    let contracts_dir = tmp.path().join("contracts");
    std::fs::create_dir_all(&contracts_dir).unwrap();

    // Empty binding registry
    let binding_path = tmp.path().join("binding.yaml");
    std::fs::write(
        &binding_path,
        "version: '1.0'\ntarget_crate: trueno\nbindings: []\n",
    )
    .unwrap();

    let output_dir = tmp.path().join("output");
    let result = onboard_model(&config_path, &contracts_dir, &binding_path, &output_dir).unwrap();

    // All contracts were scaffolded
    assert!(!result.scaffolded.is_empty());

    // All scaffolded files exist in contracts dir
    for filename in &result.scaffolded {
        assert!(
            contracts_dir.join(filename).exists(),
            "scaffolded file missing: {filename}"
        );
    }

    // FAR was created with the scaffolded contracts
    assert!(result.far_manifest.file_count > 0);
}

#[test]
fn test_fj1353_onboard_scaffolds_only_missing() {
    let tmp = tempfile::tempdir().unwrap();

    let config_path = tmp.path().join("config.json");
    std::fs::write(&config_path, llama_config_json()).unwrap();

    let contracts_dir = tmp.path().join("contracts");
    setup_contracts_dir(&contracts_dir);
    // Pre-create a contract that would otherwise be scaffolded
    std::fs::write(
        contracts_dir.join("rope-kernel-v1.yaml"),
        "metadata:\n  name: rope-kernel-v1\n  version: '1.0.0'\n",
    )
    .unwrap();

    let binding_path = tmp.path().join("binding.yaml");
    std::fs::write(&binding_path, sample_binding_yaml()).unwrap();

    let output_dir = tmp.path().join("output");
    let result = onboard_model(&config_path, &contracts_dir, &binding_path, &output_dir).unwrap();

    // rope-kernel-v1 should NOT appear in scaffolded list (it already existed)
    assert!(
        !result
            .scaffolded
            .contains(&"rope-kernel-v1.yaml".to_string()),
        "rope should not have been scaffolded"
    );
}

#[test]
fn test_fj1353_far_file_entries_sorted() {
    let tmp = tempfile::tempdir().unwrap();
    let contracts_dir = tmp.path().join("contracts");
    setup_contracts_dir(&contracts_dir);

    let config = parse_hf_config_str(llama_config_json()).unwrap();
    let coverage = coverage_report(
        "llama",
        &[],
        &BindingRegistry {
            version: "1.0".to_string(),
            target_crate: "trueno".to_string(),
            bindings: vec![],
        },
        &[],
    );

    let far_path = tmp.path().join("output.far");
    let manifest = contracts_to_far(&contracts_dir, &config, &coverage, &far_path).unwrap();

    // File entries should be sorted by path (from chunk_directory)
    let paths: Vec<&str> = manifest.files.iter().map(|f| f.path.as_str()).collect();
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(paths, sorted);
}
