//! FJ-1315/1341–1344: Sandbox config, derivation model, DAG execution.
//! Usage: cargo test --test falsification_derivation_dag

use forjar::core::store::derivation::{
    collect_input_hashes, compute_depth, derivation_closure_hash, derivation_purity,
    parse_derivation, validate_dag, validate_derivation, Derivation, DerivationInput,
};
use forjar::core::store::derivation_exec::{
    is_store_hit, plan_derivation, simulate_derivation, skipped_steps,
};
use forjar::core::store::purity::PurityLevel;
use forjar::core::store::sandbox::{
    blocks_network, cgroup_path, enforces_fs_isolation, parse_sandbox_config, preset_profile,
    validate_config, SandboxConfig, SandboxLevel,
};
use std::collections::BTreeMap;
use std::path::Path;

// ── helpers ──

fn sc(level: SandboxLevel) -> SandboxConfig {
    SandboxConfig {
        level,
        memory_mb: 2048,
        cpus: 4.0,
        timeout: 600,
        bind_mounts: vec![],
        env: vec![],
    }
}

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

// ── FJ-1315: SandboxLevel serde ──

#[test]
fn sandbox_level_serde_roundtrip() {
    for level in [
        SandboxLevel::Full,
        SandboxLevel::NetworkOnly,
        SandboxLevel::Minimal,
        SandboxLevel::None,
    ] {
        let json = serde_json::to_string(&level).unwrap();
        let parsed: SandboxLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(level, parsed);
    }
}

#[test]
fn sandbox_config_serde_roundtrip() {
    let c = sc(SandboxLevel::Full);
    let json = serde_json::to_string(&c).unwrap();
    let parsed: SandboxConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(c, parsed);
}

// ── FJ-1315: validate_config ──

#[test]
fn validate_config_good() {
    assert!(validate_config(&sc(SandboxLevel::Full)).is_empty());
}

#[test]
fn validate_config_zero_memory() {
    let mut c = sc(SandboxLevel::Full);
    c.memory_mb = 0;
    assert!(validate_config(&c).iter().any(|e| e.contains("memory_mb")));
}

#[test]
fn validate_config_zero_cpus() {
    let mut c = sc(SandboxLevel::Full);
    c.cpus = 0.0;
    assert!(validate_config(&c).iter().any(|e| e.contains("cpus")));
}

#[test]
fn validate_config_zero_timeout() {
    let mut c = sc(SandboxLevel::Full);
    c.timeout = 0;
    assert!(validate_config(&c).iter().any(|e| e.contains("timeout")));
}

#[test]
fn validate_config_excessive_memory() {
    let mut c = sc(SandboxLevel::Full);
    c.memory_mb = 2_000_000;
    assert!(validate_config(&c).iter().any(|e| e.contains("TiB")));
}

#[test]
fn validate_config_excessive_cpus() {
    let mut c = sc(SandboxLevel::Full);
    c.cpus = 2000.0;
    assert!(validate_config(&c).iter().any(|e| e.contains("1024")));
}

// ── FJ-1315: preset_profile ──

#[test]
fn preset_profiles_exist() {
    for name in ["full", "network-only", "minimal", "gpu"] {
        assert!(preset_profile(name).is_some(), "{name} should exist");
    }
    assert!(preset_profile("nonexistent").is_none());
}

#[test]
fn preset_gpu_has_bind_mounts() {
    let gpu = preset_profile("gpu").unwrap();
    assert!(!gpu.bind_mounts.is_empty());
    assert!(!gpu.env.is_empty());
}

// ── FJ-1315: parse_sandbox_config ──

#[test]
fn parse_sandbox_config_yaml() {
    let yaml = "level: full\nmemory_mb: 4096\ncpus: 8.0\ntimeout: 1200\n";
    let c = parse_sandbox_config(yaml).unwrap();
    assert_eq!(c.level, SandboxLevel::Full);
    assert_eq!(c.memory_mb, 4096);
    assert_eq!(c.cpus, 8.0);
}

#[test]
fn parse_sandbox_config_invalid() {
    assert!(parse_sandbox_config("invalid: [[[").is_err());
}

// ── FJ-1315: blocks_network, enforces_fs_isolation, cgroup_path ──

#[test]
fn blocks_network_full_only() {
    assert!(blocks_network(SandboxLevel::Full));
    assert!(!blocks_network(SandboxLevel::NetworkOnly));
    assert!(!blocks_network(SandboxLevel::Minimal));
    assert!(!blocks_network(SandboxLevel::None));
}

#[test]
fn enforces_fs_isolation_except_none() {
    assert!(enforces_fs_isolation(SandboxLevel::Full));
    assert!(enforces_fs_isolation(SandboxLevel::NetworkOnly));
    assert!(enforces_fs_isolation(SandboxLevel::Minimal));
    assert!(!enforces_fs_isolation(SandboxLevel::None));
}

#[test]
fn cgroup_path_format() {
    let p = cgroup_path("blake3:abcdef1234567890rest");
    assert!(p.starts_with("/sys/fs/cgroup/forjar-build-"));
    assert!(p.contains("abcdef1234567890"));
}

// ── FJ-1341: validate_derivation ──

#[test]
fn validate_derivation_good() {
    assert!(validate_derivation(&drv(&["a"], "make")).is_empty());
}

#[test]
fn validate_derivation_no_inputs() {
    let d = Derivation {
        inputs: BTreeMap::new(),
        script: "make".into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    };
    assert!(validate_derivation(&d)
        .iter()
        .any(|e| e.contains("at least one input")));
}

#[test]
fn validate_derivation_empty_script() {
    let mut d = drv(&["a"], "x");
    d.script = "   ".into();
    assert!(validate_derivation(&d).iter().any(|e| e.contains("script")));
}

#[test]
fn validate_derivation_empty_store_hash() {
    let inputs: BTreeMap<String, DerivationInput> =
        [("x".into(), DerivationInput::Store { store: "".into() })].into();
    let d = Derivation {
        inputs,
        script: "make".into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    };
    assert!(validate_derivation(&d)
        .iter()
        .any(|e| e.contains("store hash cannot be empty")));
}

// ── FJ-1341: closure hash ──

#[test]
fn closure_hash_deterministic() {
    let d = drv(&["a"], "build");
    let hashes: BTreeMap<String, String> = [("a".into(), "blake3:ahash".into())].into();
    let h1 = derivation_closure_hash(&d, &hashes);
    let h2 = derivation_closure_hash(&d, &hashes);
    assert_eq!(h1, h2);
    assert!(h1.starts_with("blake3:"));
}

#[test]
fn closure_hash_sensitive_to_script() {
    let d1 = drv(&["a"], "build-a");
    let d2 = drv(&["a"], "build-b");
    let hashes: BTreeMap<String, String> = [("a".into(), "blake3:ah".into())].into();
    assert_ne!(
        derivation_closure_hash(&d1, &hashes),
        derivation_closure_hash(&d2, &hashes)
    );
}

// ── FJ-1341: collect_input_hashes ──

#[test]
fn collect_store_inputs() {
    let d = drv(&["src"], "make");
    let hashes = collect_input_hashes(&d, &BTreeMap::new()).unwrap();
    assert_eq!(hashes["src"], "blake3:srchash");
}

#[test]
fn collect_resource_inputs_resolved() {
    let inputs: BTreeMap<String, DerivationInput> = [(
        "dep".into(),
        DerivationInput::Resource {
            resource: "nginx".into(),
        },
    )]
    .into();
    let d = Derivation {
        inputs,
        script: "make".into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    };
    let resolved: BTreeMap<String, String> = [("nginx".into(), "blake3:r".into())].into();
    assert_eq!(
        collect_input_hashes(&d, &resolved).unwrap()["dep"],
        "blake3:r"
    );
}

#[test]
fn collect_resource_inputs_unresolved() {
    let inputs: BTreeMap<String, DerivationInput> = [(
        "dep".into(),
        DerivationInput::Resource {
            resource: "missing".into(),
        },
    )]
    .into();
    let d = Derivation {
        inputs,
        script: "make".into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    };
    assert!(collect_input_hashes(&d, &BTreeMap::new()).is_err());
}

// ── FJ-1341: derivation_purity ──

#[test]
fn purity_from_sandbox() {
    assert_eq!(derivation_purity(&drv(&["a"], "s")), PurityLevel::Impure);
    let mut d = drv(&["a"], "s");
    d.sandbox = Some(sc(SandboxLevel::Full));
    assert_eq!(derivation_purity(&d), PurityLevel::Pure);
    d.sandbox = Some(sc(SandboxLevel::NetworkOnly));
    assert_eq!(derivation_purity(&d), PurityLevel::Pinned);
    d.sandbox = Some(sc(SandboxLevel::Minimal));
    assert_eq!(derivation_purity(&d), PurityLevel::Constrained);
}

// ── FJ-1341: parse_derivation, compute_depth, serde ──

#[test]
fn parse_derivation_yaml() {
    let yaml = "inputs:\n  src:\n    store: blake3:abc\nscript: make build\n";
    let d = parse_derivation(yaml).unwrap();
    assert_eq!(d.inputs.len(), 1);
    assert_eq!(d.script, "make build");
}

#[test]
fn parse_derivation_invalid() {
    assert!(parse_derivation("not valid yaml: [[").is_err());
}

#[test]
fn compute_depth_values() {
    assert_eq!(compute_depth(&[]), 1);
    assert_eq!(compute_depth(&[1]), 2);
    assert_eq!(compute_depth(&[3, 1, 2]), 4);
}

#[test]
fn derivation_serde_roundtrip() {
    let d = drv(&["src"], "make build");
    let json = serde_json::to_string(&d).unwrap();
    let parsed: Derivation = serde_json::from_str(&json).unwrap();
    assert_eq!(d, parsed);
}

// ── FJ-1341: validate_dag ──

#[test]
fn dag_linear_order() {
    let graph: BTreeMap<String, Vec<String>> = [
        ("c".into(), vec!["b".into()]),
        ("b".into(), vec!["a".into()]),
        ("a".into(), vec![]),
    ]
    .into();
    let order = validate_dag(&graph).unwrap();
    let a_pos = order.iter().position(|n| n == "a").unwrap();
    let b_pos = order.iter().position(|n| n == "b").unwrap();
    let c_pos = order.iter().position(|n| n == "c").unwrap();
    assert!(a_pos < b_pos);
    assert!(b_pos < c_pos);
}

#[test]
fn dag_cycle_detected() {
    let graph: BTreeMap<String, Vec<String>> = [
        ("a".into(), vec!["b".into()]),
        ("b".into(), vec!["a".into()]),
    ]
    .into();
    assert!(validate_dag(&graph).is_err());
}

// ── FJ-1342: plan_derivation ──

#[test]
fn plan_derivation_store_miss() {
    let d = drv(&["src"], "make");
    let plan = plan_derivation(&d, &BTreeMap::new(), &[], Path::new("/store")).unwrap();
    assert!(!plan.store_hit);
    assert!(plan.sandbox_plan.is_some());
    assert_eq!(plan.steps.len(), 10);
    assert_eq!(skipped_steps(&plan), 0);
    assert!(!is_store_hit(&plan));
}

#[test]
fn plan_derivation_store_hit_skips_build() {
    let d = drv(&["src"], "make");
    let hashes: BTreeMap<String, String> = [("src".into(), "blake3:srchash".into())].into();
    let closure = derivation_closure_hash(&d, &hashes);
    let plan = plan_derivation(&d, &BTreeMap::new(), &[closure], Path::new("/store")).unwrap();
    assert!(plan.store_hit);
    assert!(plan.sandbox_plan.is_none());
    assert_eq!(skipped_steps(&plan), 7);
    assert!(is_store_hit(&plan));
}

#[test]
fn plan_derivation_validation_error() {
    let d = Derivation {
        inputs: BTreeMap::new(),
        script: "make".into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    };
    assert!(plan_derivation(&d, &BTreeMap::new(), &[], Path::new("/s")).is_err());
}

// ── FJ-1343: simulate_derivation ──

#[test]
fn simulate_derivation_produces_result() {
    let d = drv(&["src"], "make");
    let r = simulate_derivation(&d, &BTreeMap::new(), &[], Path::new("/store")).unwrap();
    assert!(r.store_hash.starts_with("blake3:"));
    assert_eq!(r.derivation_depth, 1);
}

#[test]
fn simulate_derivation_store_hit_returns_cached() {
    let d = drv(&["src"], "make");
    let hashes: BTreeMap<String, String> = [("src".into(), "blake3:srchash".into())].into();
    let closure = derivation_closure_hash(&d, &hashes);
    let r = simulate_derivation(
        &d,
        &BTreeMap::new(),
        std::slice::from_ref(&closure),
        Path::new("/store"),
    )
    .unwrap();
    assert_eq!(r.closure_hash, closure);
}
