//! Tests for FJ-1341–FJ-1344: Store derivation model.

use super::derivation::{
    collect_input_hashes, compute_depth, derivation_closure_hash, derivation_purity,
    parse_derivation, validate_dag, validate_derivation, Derivation, DerivationInput,
};
use super::purity::PurityLevel;
use super::sandbox::{SandboxConfig, SandboxLevel};
use std::collections::BTreeMap;

fn sample_derivation() -> Derivation {
    let mut inputs = BTreeMap::new();
    inputs.insert(
        "base".to_string(),
        DerivationInput::Store {
            store: "blake3:aaa111".to_string(),
        },
    );
    inputs.insert(
        "config".to_string(),
        DerivationInput::Resource {
            resource: "nginx-config".to_string(),
        },
    );
    Derivation {
        inputs,
        script: "cp -r $inputs/base/* $out/\ncp $inputs/config/* $out/etc/".to_string(),
        sandbox: Some(SandboxConfig {
            level: SandboxLevel::Full,
            memory_mb: 2048,
            cpus: 4.0,
            timeout: 600,
            bind_mounts: Vec::new(),
            env: Vec::new(),
        }),
        arch: "x86_64".to_string(),
        out_var: "$out".to_string(),
    }
}

#[test]
fn test_fj1341_validate_valid() {
    let d = sample_derivation();
    let errors = validate_derivation(&d);
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_fj1341_validate_no_inputs() {
    let mut d = sample_derivation();
    d.inputs.clear();
    let errors = validate_derivation(&d);
    assert!(errors.iter().any(|e| e.contains("at least one input")));
}

#[test]
fn test_fj1341_validate_empty_script() {
    let mut d = sample_derivation();
    d.script = "  ".to_string();
    let errors = validate_derivation(&d);
    assert!(errors.iter().any(|e| e.contains("script")));
}

#[test]
fn test_fj1341_validate_empty_store_hash() {
    let mut inputs = BTreeMap::new();
    inputs.insert(
        "bad".to_string(),
        DerivationInput::Store {
            store: String::new(),
        },
    );
    let d = Derivation {
        inputs,
        script: "echo ok".to_string(),
        sandbox: None,
        arch: "x86_64".to_string(),
        out_var: "$out".to_string(),
    };
    let errors = validate_derivation(&d);
    assert!(errors.iter().any(|e| e.contains("store hash")));
}

#[test]
fn test_fj1341_validate_empty_resource_name() {
    let mut inputs = BTreeMap::new();
    inputs.insert(
        "bad".to_string(),
        DerivationInput::Resource {
            resource: String::new(),
        },
    );
    let d = Derivation {
        inputs,
        script: "echo ok".to_string(),
        sandbox: None,
        arch: "x86_64".to_string(),
        out_var: "$out".to_string(),
    };
    let errors = validate_derivation(&d);
    assert!(errors.iter().any(|e| e.contains("resource name")));
}

#[test]
fn test_fj1341_closure_hash_deterministic() {
    let d = sample_derivation();
    let mut hashes = BTreeMap::new();
    hashes.insert("base".to_string(), "blake3:aaa111".to_string());
    hashes.insert("config".to_string(), "blake3:bbb222".to_string());
    let h1 = derivation_closure_hash(&d, &hashes);
    let h2 = derivation_closure_hash(&d, &hashes);
    assert_eq!(h1, h2);
    assert!(h1.starts_with("blake3:"));
}

#[test]
fn test_fj1341_closure_hash_changes_with_script() {
    let d1 = sample_derivation();
    let mut d2 = sample_derivation();
    d2.script = "different script".to_string();

    let mut hashes = BTreeMap::new();
    hashes.insert("base".to_string(), "blake3:aaa111".to_string());
    hashes.insert("config".to_string(), "blake3:bbb222".to_string());

    assert_ne!(
        derivation_closure_hash(&d1, &hashes),
        derivation_closure_hash(&d2, &hashes)
    );
}

#[test]
fn test_fj1341_closure_hash_changes_with_inputs() {
    let d = sample_derivation();

    let mut h1 = BTreeMap::new();
    h1.insert("base".to_string(), "blake3:aaa111".to_string());
    h1.insert("config".to_string(), "blake3:bbb222".to_string());

    let mut h2 = BTreeMap::new();
    h2.insert("base".to_string(), "blake3:aaa111".to_string());
    h2.insert("config".to_string(), "blake3:ccc333".to_string());

    assert_ne!(
        derivation_closure_hash(&d, &h1),
        derivation_closure_hash(&d, &h2)
    );
}

#[test]
fn test_fj1341_collect_input_hashes() {
    let d = sample_derivation();
    let mut resolved = BTreeMap::new();
    resolved.insert("nginx-config".to_string(), "blake3:bbb222".to_string());
    let result = collect_input_hashes(&d, &resolved).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result["base"], "blake3:aaa111");
    assert_eq!(result["config"], "blake3:bbb222");
}

#[test]
fn test_fj1341_collect_input_hashes_unresolved() {
    let d = sample_derivation();
    let resolved = BTreeMap::new();
    let err = collect_input_hashes(&d, &resolved).unwrap_err();
    assert!(err.contains("unresolved"));
}

#[test]
fn test_fj1344_validate_dag_valid() {
    let mut graph = BTreeMap::new();
    graph.insert("base".to_string(), vec![]);
    graph.insert("derived".to_string(), vec!["base".to_string()]);
    graph.insert(
        "final".to_string(),
        vec!["derived".to_string(), "base".to_string()],
    );
    let order = validate_dag(&graph).unwrap();
    assert_eq!(order.len(), 3);
    // base must come before derived
    let base_pos = order.iter().position(|x| x == "base").unwrap();
    let derived_pos = order.iter().position(|x| x == "derived").unwrap();
    assert!(base_pos < derived_pos);
}

#[test]
fn test_fj1344_validate_dag_cycle() {
    let mut graph = BTreeMap::new();
    graph.insert("a".to_string(), vec!["b".to_string()]);
    graph.insert("b".to_string(), vec!["a".to_string()]);
    let err = validate_dag(&graph).unwrap_err();
    assert!(err.contains("cycle"));
}

#[test]
fn test_fj1344_validate_dag_self_cycle() {
    let mut graph = BTreeMap::new();
    graph.insert("a".to_string(), vec!["a".to_string()]);
    let err = validate_dag(&graph).unwrap_err();
    assert!(err.contains("cycle"));
}

#[test]
fn test_fj1344_validate_dag_empty() {
    let graph = BTreeMap::new();
    let order = validate_dag(&graph).unwrap();
    assert!(order.is_empty());
}

#[test]
fn test_fj1341_derivation_purity_full() {
    let d = sample_derivation();
    assert_eq!(derivation_purity(&d), PurityLevel::Pure);
}

#[test]
fn test_fj1341_derivation_purity_network_only() {
    let mut d = sample_derivation();
    d.sandbox.as_mut().unwrap().level = SandboxLevel::NetworkOnly;
    assert_eq!(derivation_purity(&d), PurityLevel::Pinned);
}

#[test]
fn test_fj1341_derivation_purity_no_sandbox() {
    let mut d = sample_derivation();
    d.sandbox = None;
    assert_eq!(derivation_purity(&d), PurityLevel::Impure);
}

#[test]
fn test_fj1341_compute_depth_from_imports() {
    assert_eq!(compute_depth(&[0]), 1);
    assert_eq!(compute_depth(&[0, 0, 0]), 1);
}

#[test]
fn test_fj1341_compute_depth_chain() {
    assert_eq!(compute_depth(&[1]), 2);
    assert_eq!(compute_depth(&[2, 1]), 3);
}

#[test]
fn test_fj1341_compute_depth_empty() {
    assert_eq!(compute_depth(&[]), 1);
}

#[test]
fn test_fj1341_parse_yaml() {
    let yaml = r#"
inputs:
  base:
    store: "blake3:abc123"
  config:
    resource: "my-config"
script: "cp -r $inputs/base/* $out/"
arch: x86_64
"#;
    let d = parse_derivation(yaml).unwrap();
    assert_eq!(d.inputs.len(), 2);
    assert!(d.script.contains("$out/"));
}

#[test]
fn test_fj1341_parse_yaml_with_sandbox() {
    let yaml = r#"
inputs:
  base:
    store: "blake3:abc123"
script: "echo hello"
sandbox:
  level: full
  memory_mb: 4096
  cpus: 8.0
  timeout: 1800
"#;
    let d = parse_derivation(yaml).unwrap();
    let sandbox = d.sandbox.unwrap();
    assert_eq!(sandbox.level, SandboxLevel::Full);
    assert_eq!(sandbox.memory_mb, 4096);
}

#[test]
fn test_fj1341_parse_yaml_invalid() {
    assert!(parse_derivation("not valid yaml [").is_err());
}

#[test]
fn test_fj1341_serde_roundtrip() {
    let d = sample_derivation();
    let yaml = serde_yaml_ng::to_string(&d).unwrap();
    let parsed: Derivation = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(d, parsed);
}
