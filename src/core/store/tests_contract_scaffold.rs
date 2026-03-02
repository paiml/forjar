//! Tests for FJ-1352: Contract YAML scaffolder.

use super::contract_scaffold::{scaffold_contracts, write_stubs};
use super::hf_config::KernelRequirement;

fn sample_missing() -> Vec<KernelRequirement> {
    vec![
        KernelRequirement {
            op: "rmsnorm".to_string(),
            contract: "rmsnorm-kernel-v1".to_string(),
        },
        KernelRequirement {
            op: "silu".to_string(),
            contract: "silu-kernel-v1".to_string(),
        },
    ]
}

#[test]
fn test_fj1352_scaffold_single_contract() {
    let missing = vec![KernelRequirement {
        op: "rmsnorm".to_string(),
        contract: "rmsnorm-kernel-v1".to_string(),
    }];
    let stubs = scaffold_contracts(&missing, "test-author");
    assert_eq!(stubs.len(), 1);
    assert_eq!(stubs[0].filename, "rmsnorm-kernel-v1.yaml");
    assert!(stubs[0].yaml_content.contains("rmsnorm-kernel-v1"));
    assert!(stubs[0].yaml_content.contains("test-author"));
    assert!(stubs[0].yaml_content.contains("0.1.0-stub"));
    assert!(stubs[0].yaml_content.contains("kernel_op: rmsnorm"));
}

#[test]
fn test_fj1352_scaffold_multiple_contracts() {
    let stubs = scaffold_contracts(&sample_missing(), "dev");
    assert_eq!(stubs.len(), 2);
    assert_eq!(stubs[0].filename, "rmsnorm-kernel-v1.yaml");
    assert_eq!(stubs[1].filename, "silu-kernel-v1.yaml");
}

#[test]
fn test_fj1352_scaffold_empty_list() {
    let stubs = scaffold_contracts(&[], "dev");
    assert!(stubs.is_empty());
}

#[test]
fn test_fj1352_scaffold_has_equations_section() {
    let missing = vec![KernelRequirement {
        op: "rope".to_string(),
        contract: "rope-kernel-v1".to_string(),
    }];
    let stubs = scaffold_contracts(&missing, "dev");
    let yaml = &stubs[0].yaml_content;
    assert!(yaml.contains("equations:"));
    assert!(yaml.contains("EQ-ROPE-01"));
}

#[test]
fn test_fj1352_scaffold_has_proof_obligations() {
    let missing = vec![KernelRequirement {
        op: "silu".to_string(),
        contract: "silu-kernel-v1".to_string(),
    }];
    let stubs = scaffold_contracts(&missing, "dev");
    let yaml = &stubs[0].yaml_content;
    assert!(yaml.contains("proof_obligations:"));
    assert!(yaml.contains("PO-SILU-01"));
}

#[test]
fn test_fj1352_scaffold_has_falsification_tests() {
    let missing = vec![KernelRequirement {
        op: "matmul".to_string(),
        contract: "matmul-kernel-v1".to_string(),
    }];
    let stubs = scaffold_contracts(&missing, "dev");
    let yaml = &stubs[0].yaml_content;
    assert!(yaml.contains("falsification_tests:"));
    assert!(yaml.contains("FALSIFY-MATMUL-001"));
}

#[test]
fn test_fj1352_scaffold_valid_yaml_parse() {
    let stubs = scaffold_contracts(&sample_missing(), "dev");
    for stub in &stubs {
        let parsed: Result<serde_yaml_ng::Value, _> = serde_yaml_ng::from_str(&stub.yaml_content);
        assert!(
            parsed.is_ok(),
            "Invalid YAML in {}: {:?}",
            stub.filename,
            parsed.err()
        );
    }
}

#[test]
fn test_fj1352_write_stubs_creates_files() {
    let dir = tempfile::tempdir().unwrap();
    let stubs = scaffold_contracts(&sample_missing(), "dev");
    let written = write_stubs(&stubs, dir.path()).unwrap();

    assert_eq!(written.len(), 2);
    assert!(dir.path().join("rmsnorm-kernel-v1.yaml").exists());
    assert!(dir.path().join("silu-kernel-v1.yaml").exists());
}

#[test]
fn test_fj1352_write_stubs_no_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    // Pre-create one file
    std::fs::write(dir.path().join("rmsnorm-kernel-v1.yaml"), "existing").unwrap();

    let stubs = scaffold_contracts(&sample_missing(), "dev");
    let written = write_stubs(&stubs, dir.path()).unwrap();

    // Only silu should be written (rmsnorm already exists)
    assert_eq!(written, vec!["silu-kernel-v1.yaml"]);
    // Verify existing file was NOT overwritten
    let content = std::fs::read_to_string(dir.path().join("rmsnorm-kernel-v1.yaml")).unwrap();
    assert_eq!(content, "existing");
}

#[test]
fn test_fj1352_write_stubs_creates_dir() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("sub").join("contracts");
    let stubs = scaffold_contracts(&sample_missing(), "dev");
    let written = write_stubs(&stubs, &nested).unwrap();
    assert_eq!(written.len(), 2);
    assert!(nested.join("rmsnorm-kernel-v1.yaml").exists());
}

#[test]
fn test_fj1352_write_stubs_empty_list() {
    let dir = tempfile::tempdir().unwrap();
    let written = write_stubs(&[], dir.path()).unwrap();
    assert!(written.is_empty());
}

#[test]
fn test_fj1352_scaffold_metadata_block() {
    let missing = vec![KernelRequirement {
        op: "gqa".to_string(),
        contract: "gqa-kernel-v1".to_string(),
    }];
    let stubs = scaffold_contracts(&missing, "team-ml");
    let yaml = &stubs[0].yaml_content;
    assert!(yaml.contains("metadata:"));
    assert!(yaml.contains("name: gqa-kernel-v1"));
    assert!(yaml.contains("author: \"team-ml\""));
    assert!(yaml.contains("kernel_op: gqa"));
}
