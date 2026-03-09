//! FJ-1342: Derivation lifecycle execution.
//!
//! Popperian rejection criteria for:
//! - FJ-1342: plan_derivation (store hit/miss, input resolution, closure hash)
//! - FJ-1342: simulate_derivation (dry-run result)
//! - FJ-1342: execute_derivation_dag (topological execution, chained deps)
//! - FJ-1342: is_store_hit, skipped_steps
//!
//! Usage: cargo test --test falsification_derivation_exec

use forjar::core::store::derivation::{Derivation, DerivationInput};
use forjar::core::store::derivation_exec::{
    execute_derivation_dag, is_store_hit, plan_derivation, simulate_derivation, skipped_steps,
};
use std::collections::BTreeMap;
use std::path::Path;

fn test_derivation() -> Derivation {
    let mut inputs = BTreeMap::new();
    inputs.insert(
        "src".into(),
        DerivationInput::Store {
            store: "blake3:aaa111".into(),
        },
    );
    Derivation {
        inputs,
        script: "cp -r $inputs/src/* $out/".into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    }
}

// ============================================================================
// FJ-1342: plan_derivation
// ============================================================================

#[test]
fn derivation_plan_store_miss() {
    let deriv = test_derivation();
    let mut resolved = BTreeMap::new();
    resolved.insert("src-resource".into(), "blake3:aaa111".into());
    let plan = plan_derivation(&deriv, &resolved, &[], Path::new("/store")).unwrap();
    assert!(!plan.store_hit);
    assert!(plan.sandbox_plan.is_some());
    assert_eq!(plan.steps.len(), 10);
    assert!(plan.closure_hash.starts_with("blake3:"));
    assert_eq!(skipped_steps(&plan), 0);
    assert!(!is_store_hit(&plan));
}

#[test]
fn derivation_plan_store_hit() {
    let deriv = test_derivation();
    let resolved = BTreeMap::new();
    let plan_miss = plan_derivation(&deriv, &resolved, &[], Path::new("/store")).unwrap();
    let plan_hit = plan_derivation(
        &deriv,
        &resolved,
        &[plan_miss.closure_hash.clone()],
        Path::new("/store"),
    )
    .unwrap();
    assert!(plan_hit.store_hit);
    assert!(plan_hit.sandbox_plan.is_none());
    assert!(is_store_hit(&plan_hit));
    assert_eq!(skipped_steps(&plan_hit), 7);
}

#[test]
fn derivation_plan_invalid_rejected() {
    let deriv = Derivation {
        inputs: BTreeMap::new(),
        script: "echo hi".into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    };
    let result = plan_derivation(&deriv, &BTreeMap::new(), &[], Path::new("/store"));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("at least one input"));
}

// ============================================================================
// FJ-1342: simulate_derivation
// ============================================================================

#[test]
fn simulate_derivation_deterministic() {
    let deriv = test_derivation();
    let resolved = BTreeMap::new();
    let r1 = simulate_derivation(&deriv, &resolved, &[], Path::new("/s")).unwrap();
    let r2 = simulate_derivation(&deriv, &resolved, &[], Path::new("/s")).unwrap();
    assert_eq!(r1.store_hash, r2.store_hash);
    assert_eq!(r1.closure_hash, r2.closure_hash);
}

#[test]
fn simulate_derivation_store_hit() {
    let deriv = test_derivation();
    let resolved = BTreeMap::new();
    let r1 = simulate_derivation(&deriv, &resolved, &[], Path::new("/s")).unwrap();
    let r2 = simulate_derivation(
        &deriv,
        &resolved,
        &[r1.closure_hash.clone()],
        Path::new("/s"),
    )
    .unwrap();
    assert_eq!(r2.store_hash, r2.closure_hash);
}

// ============================================================================
// FJ-1342: execute_derivation_dag
// ============================================================================

#[test]
fn dag_single_derivation() {
    let deriv = test_derivation();
    let mut derivations = BTreeMap::new();
    derivations.insert("build".into(), deriv);
    let topo = vec!["build".into()];
    let results =
        execute_derivation_dag(&derivations, &topo, &BTreeMap::new(), &[], Path::new("/s"))
            .unwrap();
    assert_eq!(results.len(), 1);
    assert!(results.contains_key("build"));
}

#[test]
fn dag_chained_derivations() {
    let d1 = test_derivation();
    let mut d2_inputs = BTreeMap::new();
    d2_inputs.insert(
        "dep".into(),
        DerivationInput::Resource {
            resource: "step1".into(),
        },
    );
    let d2 = Derivation {
        inputs: d2_inputs,
        script: "link $inputs/dep $out/".into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    };
    let mut derivations = BTreeMap::new();
    derivations.insert("step1".into(), d1);
    derivations.insert("step2".into(), d2);
    let topo = vec!["step1".into(), "step2".into()];
    let results =
        execute_derivation_dag(&derivations, &topo, &BTreeMap::new(), &[], Path::new("/s"))
            .unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn dag_missing_derivation_errors() {
    let result = execute_derivation_dag(
        &BTreeMap::new(),
        &["missing".into()],
        &BTreeMap::new(),
        &[],
        Path::new("/s"),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}
