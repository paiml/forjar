//! FJ-1316/1342: Sandbox lifecycle and derivation execution.
//!
//! Demonstrates:
//! - 10-step sandbox build planning (namespace, overlay, cgroup, seccomp, build, store)
//! - Seccomp BPF rules per sandbox level
//! - Sandbox plan validation
//! - Derivation lifecycle: input resolution, closure hashing, store hit/miss
//! - Derivation DAG execution with topological ordering
//! - OCI layout planning and multi-arch index generation
//!
//! Usage: cargo run --example sandbox_derivation

use forjar::core::store::derivation::{Derivation, DerivationInput};
use forjar::core::store::derivation_exec::{
    execute_derivation_dag, is_store_hit, plan_derivation, skipped_steps,
};
use forjar::core::store::sandbox::{SandboxConfig, SandboxLevel};
use forjar::core::store::sandbox_exec::{
    oci_layout_plan, plan_sandbox_build, seccomp_rules_for_level, sha256_digest, validate_plan,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn main() {
    println!("Forjar: Sandbox Lifecycle & Derivation Execution");
    println!("{}", "=".repeat(55));

    // ── FJ-1316: Sandbox Plan ──
    println!("\n[FJ-1316] Sandbox Build Plan (Full level):");
    let config = SandboxConfig {
        level: SandboxLevel::Full,
        memory_mb: 2048,
        cpus: 4.0,
        timeout: 600,
        bind_mounts: Vec::new(),
        env: Vec::new(),
    };
    let mut inputs = BTreeMap::new();
    inputs.insert("src".into(), PathBuf::from("/store/abc123/content"));
    let plan = plan_sandbox_build(
        &config,
        "blake3:deadbeef1234567890abcdef",
        &inputs,
        "make install PREFIX=$out",
        Path::new("/forjar/store"),
    );
    println!("  Namespace: {}", plan.namespace_id);
    println!("  Steps: {}", plan.steps.len());
    for step in &plan.steps {
        println!("    {}. {}", step.step, step.description);
    }
    let errors = validate_plan(&plan);
    println!("  Valid: {}", errors.is_empty());
    assert!(errors.is_empty());

    // Seccomp rules
    println!("\n[FJ-1316] Seccomp Rules:");
    for level in &[
        SandboxLevel::Full,
        SandboxLevel::NetworkOnly,
        SandboxLevel::Minimal,
    ] {
        let rules = seccomp_rules_for_level(*level);
        let names: Vec<&str> = rules.iter().map(|r| r.syscall.as_str()).collect();
        let joined = names.join(", ");
        println!(
            "  {:?}: {}",
            level,
            if names.is_empty() { "none" } else { &joined }
        );
    }

    // SHA-256 digest
    let digest = sha256_digest(b"forjar sandbox content");
    println!("\n[FJ-1316] SHA-256: {}", &digest[..30]);

    // OCI layout
    let oci_steps = oci_layout_plan(Path::new("/output/image"), "v1.0");
    println!("\n[FJ-1316] OCI Layout Plan ({} steps):", oci_steps.len());
    for s in &oci_steps {
        println!("    {}. {}", s.step, s.description);
    }

    // ── FJ-1342: Derivation ──
    println!("\n[FJ-1342] Derivation Plan (store miss):");
    let mut deriv_inputs = BTreeMap::new();
    deriv_inputs.insert(
        "src".into(),
        DerivationInput::Store {
            store: "blake3:aaa111".into(),
        },
    );
    let deriv = Derivation {
        inputs: deriv_inputs,
        script: "gcc -o $out/app $inputs/src/main.c".into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    };
    let plan = plan_derivation(&deriv, &BTreeMap::new(), &[], Path::new("/store")).unwrap();
    println!("  Store hit: {}", is_store_hit(&plan));
    println!("  Skipped steps: {}", skipped_steps(&plan));
    println!("  Closure: {}", &plan.closure_hash[..30]);
    assert!(!is_store_hit(&plan));

    // Store hit scenario
    let plan_hit = plan_derivation(
        &deriv,
        &BTreeMap::new(),
        &[plan.closure_hash.clone()],
        Path::new("/store"),
    )
    .unwrap();
    println!("\n[FJ-1342] Derivation Plan (store hit):");
    println!("  Store hit: {}", is_store_hit(&plan_hit));
    println!("  Skipped steps: {}", skipped_steps(&plan_hit));
    assert!(is_store_hit(&plan_hit));
    assert_eq!(skipped_steps(&plan_hit), 7);

    // DAG execution
    println!("\n[FJ-1342] Derivation DAG:");
    let mut derivations = BTreeMap::new();
    derivations.insert("build".into(), deriv);
    let topo = vec!["build".into()];
    let results =
        execute_derivation_dag(&derivations, &topo, &BTreeMap::new(), &[], Path::new("/s"))
            .unwrap();
    for (name, result) in &results {
        println!("  {name}: hash={}", &result.store_hash[..30]);
    }
    assert_eq!(results.len(), 1);

    println!("\n{}", "=".repeat(55));
    println!("All sandbox/derivation criteria survived.");
}
