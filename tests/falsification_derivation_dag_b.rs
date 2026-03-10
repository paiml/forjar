//! FJ-1344: DAG execution (split from falsification_derivation_dag).
//! Usage: cargo test --test falsification_derivation_dag_b
#![allow(dead_code)]

use forjar::core::store::derivation::{Derivation, DerivationInput};
use forjar::core::store::derivation_exec::{execute_derivation_dag, execute_derivation_dag_live};
use std::collections::BTreeMap;
use std::path::Path;

// ── helpers ──

fn drv(input_names: &[&str], script: &str) -> Derivation {
    let inputs = input_names
        .iter()
        .map(|n| {
            (
                n.to_string(),
                DerivationInput::Store {
                    store: format!("blake3:{n}hash"),
                },
            )
        })
        .collect();
    Derivation {
        inputs,
        script: script.to_string(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    }
}

// ── FJ-1344: DAG execution ──

#[test]
fn execute_dag_single() {
    let derivations: BTreeMap<String, Derivation> =
        [("build".into(), drv(&["src"], "make"))].into();
    let results = execute_derivation_dag(
        &derivations,
        &["build".into()],
        &BTreeMap::new(),
        &[],
        Path::new("/s"),
    )
    .unwrap();
    assert_eq!(results.len(), 1);
    assert!(results.contains_key("build"));
}

#[test]
fn execute_dag_live_dry_run() {
    let derivations: BTreeMap<String, Derivation> =
        [("build".into(), drv(&["src"], "make"))].into();
    let results = execute_derivation_dag_live(
        &derivations,
        &["build".into()],
        &BTreeMap::new(),
        &[],
        Path::new("/s"),
        true,
    )
    .unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn execute_dag_missing_derivation() {
    let derivations = BTreeMap::new();
    assert!(execute_derivation_dag(
        &derivations,
        &["missing".into()],
        &BTreeMap::new(),
        &[],
        Path::new("/s")
    )
    .is_err());
}

#[test]
fn execute_dag_chain() {
    let mut derivations = BTreeMap::new();
    derivations.insert("step1".into(), drv(&["src"], "make step1"));
    // step2 depends on step1 via resource reference
    let step2_inputs: BTreeMap<String, DerivationInput> = [(
        "dep".into(),
        DerivationInput::Resource {
            resource: "step1".into(),
        },
    )]
    .into();
    derivations.insert(
        "step2".into(),
        Derivation {
            inputs: step2_inputs,
            script: "make step2".into(),
            sandbox: None,
            arch: "x86_64".into(),
            out_var: "$out".into(),
        },
    );
    let results = execute_derivation_dag(
        &derivations,
        &["step1".into(), "step2".into()],
        &BTreeMap::new(),
        &[],
        Path::new("/s"),
    )
    .unwrap();
    assert_eq!(results.len(), 2);
}
