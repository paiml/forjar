//! FJ-1341: Derivation model validation, closure hashing, DAG validation.
//!
//! Popperian rejection criteria for:
//! - FJ-1341: validate_derivation (empty inputs, empty script, empty arch)
//! - FJ-1341: derivation_closure_hash (deterministic, script-sensitive, input-sensitive)
//! - FJ-1341: collect_input_hashes (store refs, resource refs, missing refs)
//! - FJ-1341: validate_dag (valid, cycle detection)
//! - FJ-1341: derivation_purity (Full, NetworkOnly, Minimal, None, no sandbox)
//! - FJ-1341: parse_derivation (valid YAML, defaults)
//! - FJ-1341: compute_depth (from input depths)
//!
//! Usage: cargo test --test falsification_derivation_model

use forjar::core::store::derivation::{
    collect_input_hashes, compute_depth, derivation_closure_hash, derivation_purity,
    parse_derivation, validate_dag, validate_derivation, Derivation, DerivationInput,
};
use forjar::core::store::purity::PurityLevel;
use forjar::core::store::sandbox::{SandboxConfig, SandboxLevel};
use std::collections::BTreeMap;

fn make_derivation(inputs: Vec<(&str, &str)>, script: &str) -> Derivation {
    let mut map = BTreeMap::new();
    for (name, hash) in inputs {
        map.insert(name.into(), DerivationInput::Store { store: hash.into() });
    }
    Derivation {
        inputs: map,
        script: script.into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    }
}

// ============================================================================
// validate_derivation
// ============================================================================

#[test]
fn validate_valid_derivation() {
    let d = make_derivation(vec![("src", "blake3:abc")], "make install");
    assert!(validate_derivation(&d).is_empty());
}

#[test]
fn validate_empty_inputs() {
    let d = make_derivation(vec![], "echo hi");
    let errors = validate_derivation(&d);
    assert!(errors.iter().any(|e| e.contains("at least one input")));
}

#[test]
fn validate_empty_script() {
    let d = make_derivation(vec![("a", "blake3:x")], "   ");
    let errors = validate_derivation(&d);
    assert!(errors.iter().any(|e| e.contains("script")));
}

#[test]
fn validate_empty_arch() {
    let mut d = make_derivation(vec![("a", "blake3:x")], "echo hi");
    d.arch = String::new();
    let errors = validate_derivation(&d);
    assert!(errors.iter().any(|e| e.contains("arch")));
}

#[test]
fn validate_empty_store_hash() {
    let mut d = make_derivation(vec![], "echo hi");
    d.inputs
        .insert("bad".into(), DerivationInput::Store { store: "".into() });
    let errors = validate_derivation(&d);
    assert!(errors.iter().any(|e| e.contains("store hash")));
}

#[test]
fn validate_empty_resource_name() {
    let mut d = make_derivation(vec![], "echo hi");
    d.inputs.insert(
        "bad".into(),
        DerivationInput::Resource {
            resource: "".into(),
        },
    );
    let errors = validate_derivation(&d);
    assert!(errors.iter().any(|e| e.contains("resource name")));
}

// ============================================================================
// derivation_closure_hash
// ============================================================================

#[test]
fn closure_hash_deterministic() {
    let d = make_derivation(vec![("a", "blake3:aaa")], "make");
    let mut hashes = BTreeMap::new();
    hashes.insert("a".into(), "blake3:aaa".into());
    let h1 = derivation_closure_hash(&d, &hashes);
    let h2 = derivation_closure_hash(&d, &hashes);
    assert_eq!(h1, h2);
    assert!(h1.starts_with("blake3:"));
}

#[test]
fn closure_hash_script_sensitive() {
    let d1 = make_derivation(vec![("a", "blake3:aaa")], "make");
    let d2 = make_derivation(vec![("a", "blake3:aaa")], "make install");
    let mut hashes = BTreeMap::new();
    hashes.insert("a".into(), "blake3:aaa".into());
    assert_ne!(
        derivation_closure_hash(&d1, &hashes),
        derivation_closure_hash(&d2, &hashes)
    );
}

#[test]
fn closure_hash_input_sensitive() {
    let d = make_derivation(vec![("a", "blake3:aaa")], "make");
    let mut h1 = BTreeMap::new();
    h1.insert("a".into(), "blake3:aaa".into());
    let mut h2 = BTreeMap::new();
    h2.insert("a".into(), "blake3:bbb".into());
    assert_ne!(
        derivation_closure_hash(&d, &h1),
        derivation_closure_hash(&d, &h2)
    );
}

// ============================================================================
// collect_input_hashes
// ============================================================================

#[test]
fn collect_store_refs() {
    let d = make_derivation(vec![("a", "blake3:aaa"), ("b", "blake3:bbb")], "echo");
    let result = collect_input_hashes(&d, &BTreeMap::new()).unwrap();
    assert_eq!(result["a"], "blake3:aaa");
    assert_eq!(result["b"], "blake3:bbb");
}

#[test]
fn collect_resource_refs() {
    let mut d = make_derivation(vec![], "echo");
    d.inputs.insert(
        "dep".into(),
        DerivationInput::Resource {
            resource: "my-pkg".into(),
        },
    );
    let mut resolved = BTreeMap::new();
    resolved.insert("my-pkg".into(), "blake3:resolved".into());
    let result = collect_input_hashes(&d, &resolved).unwrap();
    assert_eq!(result["dep"], "blake3:resolved");
}

#[test]
fn collect_missing_resource_errors() {
    let mut d = make_derivation(vec![], "echo");
    d.inputs.insert(
        "dep".into(),
        DerivationInput::Resource {
            resource: "missing".into(),
        },
    );
    let result = collect_input_hashes(&d, &BTreeMap::new());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unresolved"));
}

// ============================================================================
// validate_dag
// ============================================================================

#[test]
fn dag_valid_linear() {
    let mut graph = BTreeMap::new();
    graph.insert("c".into(), vec!["b".into()]);
    graph.insert("b".into(), vec!["a".into()]);
    graph.insert("a".into(), vec![]);
    let order = validate_dag(&graph).unwrap();
    assert_eq!(order.len(), 3);
    let a_pos = order.iter().position(|n| n == "a").unwrap();
    let b_pos = order.iter().position(|n| n == "b").unwrap();
    let c_pos = order.iter().position(|n| n == "c").unwrap();
    assert!(a_pos < b_pos);
    assert!(b_pos < c_pos);
}

#[test]
fn dag_cycle_detected() {
    let mut graph = BTreeMap::new();
    graph.insert("a".into(), vec!["b".into()]);
    graph.insert("b".into(), vec!["a".into()]);
    let result = validate_dag(&graph);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("cycle"));
}

#[test]
fn dag_diamond() {
    let mut graph = BTreeMap::new();
    graph.insert("d".into(), vec!["b".into(), "c".into()]);
    graph.insert("b".into(), vec!["a".into()]);
    graph.insert("c".into(), vec!["a".into()]);
    graph.insert("a".into(), vec![]);
    let order = validate_dag(&graph).unwrap();
    assert_eq!(order.len(), 4);
    let a_pos = order.iter().position(|n| n == "a").unwrap();
    let d_pos = order.iter().position(|n| n == "d").unwrap();
    assert!(a_pos < d_pos, "a must come before d");
}

// ============================================================================
// derivation_purity
// ============================================================================

#[test]
fn purity_full_is_pure() {
    let mut d = make_derivation(vec![("a", "hash")], "echo");
    d.sandbox = Some(SandboxConfig {
        level: SandboxLevel::Full,
        memory_mb: 2048,
        cpus: 4.0,
        timeout: 600,
        bind_mounts: vec![],
        env: vec![],
    });
    assert_eq!(derivation_purity(&d), PurityLevel::Pure);
}

#[test]
fn purity_network_only_is_pinned() {
    let mut d = make_derivation(vec![("a", "hash")], "echo");
    d.sandbox = Some(SandboxConfig {
        level: SandboxLevel::NetworkOnly,
        memory_mb: 2048,
        cpus: 4.0,
        timeout: 600,
        bind_mounts: vec![],
        env: vec![],
    });
    assert_eq!(derivation_purity(&d), PurityLevel::Pinned);
}

#[test]
fn purity_minimal_is_constrained() {
    let mut d = make_derivation(vec![("a", "hash")], "echo");
    d.sandbox = Some(SandboxConfig {
        level: SandboxLevel::Minimal,
        memory_mb: 2048,
        cpus: 4.0,
        timeout: 600,
        bind_mounts: vec![],
        env: vec![],
    });
    assert_eq!(derivation_purity(&d), PurityLevel::Constrained);
}

#[test]
fn purity_none_is_impure() {
    let mut d = make_derivation(vec![("a", "hash")], "echo");
    d.sandbox = Some(SandboxConfig {
        level: SandboxLevel::None,
        memory_mb: 2048,
        cpus: 4.0,
        timeout: 600,
        bind_mounts: vec![],
        env: vec![],
    });
    assert_eq!(derivation_purity(&d), PurityLevel::Impure);
}

#[test]
fn purity_no_sandbox_is_impure() {
    let d = make_derivation(vec![("a", "hash")], "echo");
    assert_eq!(derivation_purity(&d), PurityLevel::Impure);
}

// ============================================================================
// parse_derivation
// ============================================================================

#[test]
fn parse_valid_yaml() {
    let yaml = r#"
inputs:
  src:
    store: "blake3:abc123"
script: "make install PREFIX=$out"
arch: "aarch64"
"#;
    let d = parse_derivation(yaml).unwrap();
    assert_eq!(d.arch, "aarch64");
    assert!(d.script.contains("make install"));
    assert!(d.inputs.contains_key("src"));
}

#[test]
fn parse_defaults() {
    let yaml = r#"
inputs:
  a:
    store: "blake3:x"
script: "echo hi"
"#;
    let d = parse_derivation(yaml).unwrap();
    assert_eq!(d.arch, "x86_64");
    assert_eq!(d.out_var, "$out");
}

#[test]
fn parse_invalid_yaml() {
    assert!(parse_derivation("not yaml: [").is_err());
}

// ============================================================================
// compute_depth
// ============================================================================

#[test]
fn depth_no_inputs() {
    assert_eq!(compute_depth(&[]), 1);
}

#[test]
fn depth_single_input() {
    assert_eq!(compute_depth(&[3]), 4);
}

#[test]
fn depth_multiple_inputs() {
    assert_eq!(compute_depth(&[1, 5, 3]), 6);
}
