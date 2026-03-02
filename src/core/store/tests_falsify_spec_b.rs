//! Spec falsification tests: Phases E–H
//!
//! Phase E: Cache & GC (SSH transport, substitution protocol, GC roots)
//! Phase F: Derivations (7+ providers, DAG validation, closure hashing)
//! Phase G: FAR archive (magic, encode/decode, chunking)
//! Phase H: Convert (5-step ladder, auto changes, manual reports)
#![allow(unused_imports)]

use super::cache::{
    build_inventory, parse_cache_config, resolve_substitution, ssh_command, validate_cache_config,
    CacheConfig, CacheEntry, CacheInventory, CacheSource, SubstitutionResult,
};
use super::chunker;
use super::convert::{
    analyze_conversion, ChangeType, ConversionChange, ConversionReport, ConversionSignals,
};
use super::derivation::{
    collect_input_hashes, compute_depth, derivation_closure_hash, derivation_purity, validate_dag,
    validate_derivation, Derivation, DerivationInput, DerivationResult,
};
use super::far::{decode_far_manifest, encode_far, ChunkEntry, FarFileEntry, FarManifest,
    FarProvenance, FAR_MAGIC};
use super::gc::{collect_roots, GcConfig, GcReport};
use super::provider::{
    all_providers, capture_method, import_command, origin_ref_string, validate_import,
    ImportConfig, ImportProvider,
};
use super::purity::PurityLevel;
use super::substitution::{
    plan_substitution, requires_build, requires_pull, SubstitutionContext, SubstitutionOutcome,
    SubstitutionPlan, SubstitutionStep,
};
use std::collections::BTreeMap;
use std::path::Path;

// ═══════════════════════════════════════════════════════════════════
// Phase E: Cache & GC
// ═══════════════════════════════════════════════════════════════════

/// E-01: CacheSource has Ssh and Local variants.
#[test]
fn falsify_e01_cache_source_variants() {
    let _ssh = CacheSource::Ssh {
        host: "cache.internal".to_string(),
        user: "deploy".to_string(),
        path: "/var/forjar/cache".to_string(),
        port: None,
    };
    let _local = CacheSource::Local {
        path: "/var/forjar/store".to_string(),
    };
}

/// E-02: SubstitutionResult has LocalHit, CacheHit, CacheMiss.
#[test]
fn falsify_e02_substitution_result_variants() {
    let _local = SubstitutionResult::LocalHit {
        store_path: "/store/abc".to_string(),
    };
    let _cache = SubstitutionResult::CacheHit {
        source_index: 0,
        store_hash: "blake3:abc".to_string(),
    };
    let _miss = SubstitutionResult::CacheMiss;
}

/// E-03: Substitution protocol: local store checked first.
#[test]
fn falsify_e03_substitution_local_first() {
    let local = vec!["blake3:abc".to_string()];
    let result = resolve_substitution("blake3:abc", &local, &[]);
    assert!(
        matches!(result, SubstitutionResult::LocalHit { .. }),
        "local store must be checked first"
    );
}

/// E-04: Substitution protocol: SSH cache checked after local miss.
#[test]
fn falsify_e04_substitution_cache_second() {
    let entry = CacheEntry {
        store_hash: "blake3:abc".to_string(),
        size_bytes: 1024,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        provider: "apt".to_string(),
        arch: "x86_64".to_string(),
    };
    let inv = build_inventory("remote", vec![entry]);
    let result = resolve_substitution("blake3:abc", &[], &[inv]);
    assert!(
        matches!(result, SubstitutionResult::CacheHit { .. }),
        "SSH cache must be checked after local miss"
    );
}

/// E-05: Substitution protocol: CacheMiss when neither has entry.
#[test]
fn falsify_e05_substitution_miss() {
    let result = resolve_substitution("blake3:xyz", &[], &[]);
    assert!(
        matches!(result, SubstitutionResult::CacheMiss),
        "must return CacheMiss when not found"
    );
}

/// E-06: SSH-only transport — ssh_command() generates ssh prefix.
#[test]
fn falsify_e06_ssh_command() {
    let source = CacheSource::Ssh {
        host: "cache.internal".to_string(),
        user: "deploy".to_string(),
        path: "/cache".to_string(),
        port: None,
    };
    let cmd = ssh_command(&source).expect("SSH source must produce command");
    assert!(cmd.contains("ssh"), "command must use ssh: {cmd}");
    assert!(cmd.contains("deploy@cache.internal"), "must include user@host");
}

/// E-07: Local source returns None from ssh_command.
#[test]
fn falsify_e07_local_no_ssh_command() {
    let source = CacheSource::Local {
        path: "/cache".to_string(),
    };
    assert!(ssh_command(&source).is_none(), "local source has no SSH command");
}

/// E-08: GC roots from profiles + lockfile + gc-roots dir.
#[test]
fn falsify_e08_gc_roots_sources() {
    let profiles = vec!["blake3:aaa".to_string()];
    let locks = vec!["blake3:bbb".to_string()];
    let roots = collect_roots(&profiles, &locks, None);
    assert!(roots.contains("blake3:aaa"), "must include profile hashes");
    assert!(roots.contains("blake3:bbb"), "must include lockfile hashes");
}

/// E-09: GcConfig default keep_generations = 5.
#[test]
fn falsify_e09_gc_default_keep_gens() {
    let cfg = GcConfig::default();
    assert_eq!(cfg.keep_generations, 5, "default keep_generations must be 5");
}

/// E-10: plan_substitution local hit exits early.
#[test]
fn falsify_e10_substitution_plan_local_hit() {
    let cfg = CacheConfig {
        sources: vec![],
        auto_push: false,
        max_size_mb: 0,
    };
    let local = vec!["blake3:abc".to_string()];
    let ctx = SubstitutionContext {
        closure_hash: "blake3:abc",
        input_hashes: &["blake3:111".to_string()],
        local_entries: &local,
        cache_config: &cfg,
        cache_inventories: &[],
        sandbox: None,
        store_dir: Path::new("/store"),
    };
    let plan = plan_substitution(&ctx);
    assert!(
        matches!(plan.outcome, SubstitutionOutcome::LocalHit { .. }),
        "local hit must produce LocalHit outcome"
    );
    assert!(!requires_build(&plan), "local hit must not require build");
}

/// E-11: plan_substitution cache miss requires build.
#[test]
fn falsify_e11_substitution_plan_cache_miss() {
    let cfg = CacheConfig {
        sources: vec![],
        auto_push: false,
        max_size_mb: 0,
    };
    let ctx = SubstitutionContext {
        closure_hash: "blake3:xxx",
        input_hashes: &[],
        local_entries: &[],
        cache_config: &cfg,
        cache_inventories: &[],
        sandbox: None,
        store_dir: Path::new("/store"),
    };
    let plan = plan_substitution(&ctx);
    assert!(requires_build(&plan), "cache miss must require build");
}

// ═══════════════════════════════════════════════════════════════════
// Phase F: Derivations & Providers
// ═══════════════════════════════════════════════════════════════════

/// F-01: ImportProvider has 7+ variants.
#[test]
fn falsify_f01_provider_count() {
    let providers = all_providers();
    assert!(
        providers.len() >= 7,
        "spec requires at least 7 providers, got {}",
        providers.len()
    );
}

/// F-02: Required providers: apt, cargo, uv, nix, docker, tofu, terraform.
#[test]
fn falsify_f02_required_providers() {
    let providers = all_providers();
    let names: Vec<String> = providers.iter().map(|p| format!("{p:?}")).collect();
    for required in &["Apt", "Cargo", "Uv", "Nix", "Docker", "Tofu", "Terraform"] {
        assert!(
            names.iter().any(|n| n == required),
            "missing required provider: {required}"
        );
    }
}

/// F-03: apt import uses "apt-get install -y --download-only".
#[test]
fn falsify_f03_apt_import_command() {
    let cfg = ImportConfig {
        provider: ImportProvider::Apt,
        reference: "nginx".to_string(),
        version: Some("1.24.0".to_string()),
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    let cmd = import_command(&cfg);
    assert!(cmd.contains("apt-get install"), "apt: {cmd}");
    assert!(cmd.contains("--download-only"), "apt must use --download-only: {cmd}");
    assert!(cmd.contains("=1.24.0"), "apt must include version: {cmd}");
}

/// F-04: cargo import uses "cargo install --root $STAGING".
#[test]
fn falsify_f04_cargo_import_command() {
    let cfg = ImportConfig {
        provider: ImportProvider::Cargo,
        reference: "ripgrep".to_string(),
        version: Some("14.1.0".to_string()),
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    let cmd = import_command(&cfg);
    assert!(cmd.contains("cargo install"), "cargo: {cmd}");
    assert!(cmd.contains("$STAGING"), "cargo must use $STAGING: {cmd}");
}

/// F-05: nix import uses "nix build --print-out-paths".
#[test]
fn falsify_f05_nix_import_command() {
    let cfg = ImportConfig {
        provider: ImportProvider::Nix,
        reference: "nixpkgs#ripgrep".to_string(),
        version: None,
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    let cmd = import_command(&cfg);
    assert!(cmd.contains("nix build --print-out-paths"), "nix: {cmd}");
}

/// F-06: docker import uses "docker create" + "docker export".
#[test]
fn falsify_f06_docker_import_command() {
    let cfg = ImportConfig {
        provider: ImportProvider::Docker,
        reference: "ubuntu".to_string(),
        version: Some("24.04".to_string()),
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    let cmd = import_command(&cfg);
    assert!(cmd.contains("docker create"), "docker: {cmd}");
    assert!(cmd.contains("docker export"), "docker export: {cmd}");
}

/// F-07: tofu import uses "tofu -chdir=<dir> output -json".
#[test]
fn falsify_f07_tofu_import_command() {
    let cfg = ImportConfig {
        provider: ImportProvider::Tofu,
        reference: "infra/".to_string(),
        version: None,
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    let cmd = import_command(&cfg);
    assert!(cmd.contains("tofu -chdir=infra/ output -json"), "tofu: {cmd}");
}

/// F-08: terraform import uses "terraform -chdir=<dir> output -json".
#[test]
fn falsify_f08_terraform_import_command() {
    let cfg = ImportConfig {
        provider: ImportProvider::Terraform,
        reference: "infra/".to_string(),
        version: None,
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    let cmd = import_command(&cfg);
    assert!(
        cmd.contains("terraform -chdir=infra/ output -json"),
        "terraform: {cmd}"
    );
}

/// F-09: Each provider has a capture_method description.
#[test]
fn falsify_f09_capture_methods() {
    for provider in all_providers() {
        let method = capture_method(provider);
        assert!(!method.is_empty(), "{provider:?} must have capture method");
    }
}

/// F-10: DerivationInput has Store and Resource variants.
#[test]
fn falsify_f10_derivation_input_variants() {
    let _store = DerivationInput::Store {
        store: "blake3:abc".to_string(),
    };
    let _resource = DerivationInput::Resource {
        resource: "nginx".to_string(),
    };
}

/// F-11: validate_dag detects cycles.
#[test]
fn falsify_f11_dag_cycle_detection() {
    let mut graph = BTreeMap::new();
    graph.insert("a".to_string(), vec!["b".to_string()]);
    graph.insert("b".to_string(), vec!["a".to_string()]);
    let result = validate_dag(&graph);
    assert!(result.is_err(), "cyclic DAG must be rejected");
}

/// F-12: validate_dag produces topological order for valid DAGs.
#[test]
fn falsify_f12_dag_topological_order() {
    let mut graph = BTreeMap::new();
    graph.insert("a".to_string(), vec!["b".to_string()]);
    graph.insert("b".to_string(), vec![]);
    let order = validate_dag(&graph).expect("valid DAG");
    let a_pos = order.iter().position(|x| x == "a").unwrap();
    let b_pos = order.iter().position(|x| x == "b").unwrap();
    assert!(b_pos < a_pos, "deps must come before dependents in topo order");
}

/// F-13: derivation_purity: Full sandbox → Pure, None → Impure.
#[test]
fn falsify_f13_derivation_purity_levels() {
    use super::sandbox::{SandboxConfig, SandboxLevel};
    let mut deriv = Derivation {
        inputs: BTreeMap::from([("x".to_string(), DerivationInput::Store {
            store: "blake3:abc".to_string(),
        })]),
        script: "echo hello".to_string(),
        sandbox: Some(SandboxConfig {
            level: SandboxLevel::Full,
            memory_mb: 2048,
            cpus: 4.0,
            timeout: 600,
            bind_mounts: vec![],
            env: vec![],
        }),
        arch: "x86_64".to_string(),
        out_var: "$out".to_string(),
    };
    assert_eq!(derivation_purity(&deriv), PurityLevel::Pure);

    deriv.sandbox = None;
    assert_eq!(derivation_purity(&deriv), PurityLevel::Impure);
}

/// F-14: compute_depth(empty) = 1, compute_depth([0,0]) = 1, compute_depth([2]) = 3.
#[test]
fn falsify_f14_derivation_depth() {
    assert_eq!(compute_depth(&[]), 1);
    assert_eq!(compute_depth(&[0, 0]), 1);
    assert_eq!(compute_depth(&[2]), 3);
}

/// F-15: validate_derivation rejects empty script.
#[test]
fn falsify_f15_derivation_validate_empty_script() {
    let deriv = Derivation {
        inputs: BTreeMap::from([("x".to_string(), DerivationInput::Store {
            store: "blake3:abc".to_string(),
        })]),
        script: "   ".to_string(),
        sandbox: None,
        arch: "x86_64".to_string(),
        out_var: "$out".to_string(),
    };
    let errors = validate_derivation(&deriv);
    assert!(!errors.is_empty(), "empty script must be rejected");
}

/// F-16: validate_import rejects empty reference.
#[test]
fn falsify_f16_validate_import_empty_ref() {
    let cfg = ImportConfig {
        provider: ImportProvider::Apt,
        reference: "".to_string(),
        version: None,
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    let errors = validate_import(&cfg);
    assert!(!errors.is_empty(), "empty reference must be rejected");
}

/// F-17: uv import uses "uv pip install --target $STAGING".
#[test]
fn falsify_f17_uv_import_command() {
    let cfg = ImportConfig {
        provider: ImportProvider::Uv,
        reference: "flask".to_string(),
        version: Some("3.0.0".to_string()),
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    let cmd = import_command(&cfg);
    assert!(cmd.contains("uv pip install"), "uv: {cmd}");
    assert!(cmd.contains("$STAGING"), "uv must use $STAGING: {cmd}");
    assert!(cmd.contains("==3.0.0"), "uv must include version: {cmd}");
}

// Phases G-H tests moved to tests_falsify_spec_e.rs (500-line limit)
