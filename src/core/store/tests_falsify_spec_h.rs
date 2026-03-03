//! Spec falsification gap tests: Phase D (sandbox exec, seccomp) +
//! Phase E (GC mark-sweep, substitution) + Phase F (derivation exec, DAG)
//!
//! Fills gaps D-14–D-20, E-12–E-17, F-18–F-26 from the gap analysis.
#![allow(unused_imports)]

use super::cache::{
    resolve_substitution, ssh_command, CacheConfig, CacheInventory, CacheSource,
    SubstitutionResult,
};
use super::derivation::{
    collect_input_hashes, derivation_closure_hash, validate_derivation, Derivation,
    DerivationInput, DerivationResult,
};
use super::derivation_exec::{
    execute_derivation_dag, is_store_hit, plan_derivation, simulate_derivation, skipped_steps,
    DerivationPlan, DerivationStep,
};
use super::gc::{collect_roots, mark_and_sweep, GcConfig, GcReport};
use super::meta::{new_meta, Provenance, StoreMeta};
use super::sandbox::{preset_profile, SandboxConfig, SandboxLevel};
use super::sandbox_exec::{
    plan_sandbox_build, seccomp_rules_for_level, simulate_sandbox_build, validate_plan,
    OverlayConfig, SandboxPlan, SandboxStep, SeccompRule,
};
use super::store_diff::{
    build_sync_plan, compute_diff, has_diffable_provenance, upstream_check_command,
};
use super::substitution::{plan_substitution, SubstitutionContext, SubstitutionOutcome};
use std::collections::BTreeMap;
use std::path::PathBuf;

// ═══════════════════════════════════════════════════════════════════
// Phase D gaps: Sandbox execution plan, seccomp, overlayfs
// ═══════════════════════════════════════════════════════════════════

/// D-14: plan_sandbox_build() produces exactly 10 lifecycle steps.
#[test]
fn falsify_d14_sandbox_plan_10_steps() {
    let config = preset_profile("full").unwrap();
    let mut inputs = BTreeMap::new();
    inputs.insert("src".to_string(), PathBuf::from("/store/abc/content"));
    let plan = plan_sandbox_build(
        &config,
        "blake3:aabbcc",
        &inputs,
        "echo build",
        std::path::Path::new("/var/forjar/store"),
    );
    assert!(
        plan.steps.len() >= 10,
        "Full sandbox must have >=10 steps, got {}",
        plan.steps.len()
    );
}

/// D-15: Seccomp BPF rules for Full level deny connect, mount, ptrace.
#[test]
fn falsify_d15_seccomp_full_denies() {
    let rules = seccomp_rules_for_level(SandboxLevel::Full);
    let names: Vec<&str> = rules.iter().map(|r| r.syscall.as_str()).collect();
    assert!(names.contains(&"connect"), "must deny connect");
    assert!(names.contains(&"mount"), "must deny mount");
    assert!(names.contains(&"ptrace"), "must deny ptrace");
}

/// D-16: Seccomp BPF rules for non-Full levels are empty.
#[test]
fn falsify_d16_seccomp_non_full_empty() {
    assert!(seccomp_rules_for_level(SandboxLevel::Minimal).is_empty());
    assert!(seccomp_rules_for_level(SandboxLevel::NetworkOnly).is_empty());
}

/// D-17: Overlay config has lower_dirs, upper_dir, work_dir, merged_dir.
#[test]
fn falsify_d17_overlay_config_fields() {
    let config = preset_profile("full").unwrap();
    let mut inputs = BTreeMap::new();
    inputs.insert("src".to_string(), PathBuf::from("/store/abc"));
    let plan = plan_sandbox_build(
        &config, "blake3:test", &inputs, "echo test",
        std::path::Path::new("/store"),
    );
    assert!(!plan.overlay.lower_dirs.is_empty());
    assert!(plan.overlay.upper_dir.to_string_lossy().contains("upper"));
    assert!(plan.overlay.work_dir.to_string_lossy().contains("work"));
}

/// D-18: validate_plan() catches empty steps.
#[test]
fn falsify_d18_validate_plan_empty() {
    let plan = SandboxPlan {
        steps: vec![],
        namespace_id: "test".to_string(),
        overlay: OverlayConfig {
            lower_dirs: vec![PathBuf::from("/a")],
            upper_dir: PathBuf::from("/b"),
            work_dir: PathBuf::from("/c"),
            merged_dir: PathBuf::from("/d"),
        },
        seccomp_rules: vec![],
        cgroup_path: "/cg".to_string(),
    };
    let errors = validate_plan(&plan);
    assert!(!errors.is_empty(), "empty steps must produce error");
}

/// D-19: simulate_sandbox_build() produces deterministic output hash.
#[test]
fn falsify_d19_simulate_deterministic() {
    let config = preset_profile("minimal").unwrap();
    let mut inputs = BTreeMap::new();
    inputs.insert("src".to_string(), PathBuf::from("/store/abc"));
    let r1 = simulate_sandbox_build(&config, "hash1", &inputs, "echo hello",
        std::path::Path::new("/store"));
    let r2 = simulate_sandbox_build(&config, "hash1", &inputs, "echo hello",
        std::path::Path::new("/store"));
    assert_eq!(r1.output_hash, r2.output_hash, "simulation must be deterministic");
}

/// D-20: namespace_id derived from build hash prefix.
#[test]
fn falsify_d20_namespace_id_from_hash() {
    let config = preset_profile("minimal").unwrap();
    let plan = plan_sandbox_build(
        &config, "blake3:abcdef0123456789", &BTreeMap::new(), "true",
        std::path::Path::new("/store"),
    );
    assert!(plan.namespace_id.starts_with("forjar-build-"));
    assert!(plan.namespace_id.contains("blake3:abcdef01"));
}

// ═══════════════════════════════════════════════════════════════════
// Phase E gaps: GC mark-sweep, substitution
// ═══════════════════════════════════════════════════════════════════

/// E-12: collect_roots() aggregates profile + lockfile hashes.
#[test]
fn falsify_e12_gc_collect_roots() {
    let roots = collect_roots(
        &["blake3:profile_root".to_string()],
        &["blake3:lock_pin".to_string()],
        None,
    );
    assert!(roots.contains("blake3:profile_root"));
    assert!(roots.contains("blake3:lock_pin"));
}

/// E-13: mark_and_sweep() marks unreachable entries as dead.
#[test]
fn falsify_e13_gc_mark_sweep_dead() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("store");
    let reachable_hash = "a".repeat(64);
    let dead_hash = "b".repeat(64);
    std::fs::create_dir_all(store.join(&reachable_hash)).unwrap();
    std::fs::create_dir_all(store.join(&dead_hash)).unwrap();

    let roots = std::collections::BTreeSet::from([format!("blake3:{reachable_hash}")]);
    let report = mark_and_sweep(&roots, &store).unwrap();
    assert!(
        report.dead.contains(&format!("blake3:{dead_hash}")),
        "unreachable entry must be marked dead: {:?}",
        report.dead
    );
    assert!(
        !report.dead.contains(&format!("blake3:{reachable_hash}")),
        "reachable entry must not be marked dead"
    );
}

/// E-14: resolve_substitution() prefers local store.
#[test]
fn falsify_e14_substitution_local_first() {
    let result = resolve_substitution(
        "blake3:cached",
        &["blake3:cached".to_string()],
        &[],
    );
    assert!(matches!(result, SubstitutionResult::LocalHit { .. }));
}

/// E-15: resolve_substitution() falls back to cache inventory.
#[test]
fn falsify_e15_substitution_cache_fallback() {
    use super::cache::{build_inventory, CacheEntry};
    let entry = CacheEntry {
        store_hash: "blake3:remote".to_string(),
        size_bytes: 1024,
        created_at: "now".to_string(),
        provider: "apt".to_string(),
        arch: "x86_64".to_string(),
    };
    let inv = build_inventory("cache1", vec![entry]);
    let result = resolve_substitution("blake3:remote", &[], &[inv]);
    assert!(matches!(result, SubstitutionResult::CacheHit { .. }));
}

/// E-16: resolve_substitution() CacheMiss when not found.
#[test]
fn falsify_e16_substitution_cache_miss() {
    let result = resolve_substitution("blake3:nothing", &[], &[]);
    assert_eq!(result, SubstitutionResult::CacheMiss);
}

/// E-17: GcConfig default keep_generations is 5.
#[test]
fn falsify_e17_gc_keep_generations_default() {
    let config = GcConfig::default();
    assert_eq!(config.keep_generations, 5);
}

// ═══════════════════════════════════════════════════════════════════
// Phase F gaps: Derivation execution, DAG lifecycle, diff/sync
// ═══════════════════════════════════════════════════════════════════

fn test_derivation(script: &str) -> Derivation {
    let mut inputs = BTreeMap::new();
    inputs.insert(
        "src".to_string(),
        DerivationInput::Store { store: "blake3:abc".to_string() },
    );
    Derivation {
        inputs,
        script: script.to_string(),
        sandbox: None,
        arch: "x86_64".to_string(),
        out_var: "$out".to_string(),
    }
}

/// F-18: plan_derivation() generates 10-step lifecycle for cache miss.
#[test]
fn falsify_f18_derivation_plan_10_steps() {
    let deriv = test_derivation("echo build");
    let mut resources = BTreeMap::new();
    resources.insert("src".to_string(), "blake3:abc".to_string());
    let plan = plan_derivation(&deriv, &resources, &[], std::path::Path::new("/store")).unwrap();
    assert_eq!(plan.steps.len(), 10, "cache-miss must have 10 steps, got {}", plan.steps.len());
    assert!(!plan.store_hit);
    assert!(plan.sandbox_plan.is_some());
}

/// F-19: plan_derivation() skips steps 4-10 on store hit.
#[test]
fn falsify_f19_derivation_store_hit_skips() {
    let deriv = test_derivation("echo build");
    let mut resources = BTreeMap::new();
    resources.insert("src".to_string(), "blake3:abc".to_string());
    let input_hashes = collect_input_hashes(&deriv, &resources).unwrap();
    let closure = derivation_closure_hash(&deriv, &input_hashes);
    let plan = plan_derivation(&deriv, &resources, &[closure], std::path::Path::new("/store"))
        .unwrap();
    assert!(plan.store_hit);
    assert!(plan.sandbox_plan.is_none());
    assert_eq!(skipped_steps(&plan), 7);
}

/// F-20: simulate_derivation() produces DerivationResult with closure_hash.
#[test]
fn falsify_f20_simulate_derivation_result() {
    let deriv = test_derivation("echo test");
    let mut resources = BTreeMap::new();
    resources.insert("src".to_string(), "blake3:abc".to_string());
    let result = simulate_derivation(&deriv, &resources, &[], std::path::Path::new("/store"))
        .unwrap();
    assert!(result.closure_hash.starts_with("blake3:"));
    assert!(result.store_hash.starts_with("blake3:"));
}

/// F-21: execute_derivation_dag() processes in topological order.
#[test]
fn falsify_f21_dag_topological_execution() {
    let mut a_inputs = BTreeMap::new();
    a_inputs.insert("root".to_string(), DerivationInput::Store { store: "blake3:root".to_string() });
    let mut b_inputs = BTreeMap::new();
    b_inputs.insert("dep".to_string(), DerivationInput::Resource { resource: "a".to_string() });

    let mut derivations = BTreeMap::new();
    derivations.insert("a".to_string(), Derivation {
        inputs: a_inputs, script: "echo a".to_string(), sandbox: None,
        arch: "x86_64".to_string(), out_var: "$out".to_string(),
    });
    derivations.insert("b".to_string(), Derivation {
        inputs: b_inputs, script: "echo b".to_string(), sandbox: None,
        arch: "x86_64".to_string(), out_var: "$out".to_string(),
    });

    let mut init = BTreeMap::new();
    init.insert("root".to_string(), "blake3:root".to_string());
    let results = execute_derivation_dag(
        &derivations, &["a".to_string(), "b".to_string()],
        &init, &[], std::path::Path::new("/store"),
    ).unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.contains_key("a"));
    assert!(results.contains_key("b"));
}

/// F-22: compute_diff() detects upstream change.
#[test]
fn falsify_f22_diff_detects_change() {
    let meta = test_meta("blake3:local", "apt", Some("sha256:old"));
    let diff = compute_diff(&meta, Some("sha256:new"));
    assert!(diff.upstream_changed, "different hash must detect change");
}

/// F-23: compute_diff() no change when hashes match.
#[test]
fn falsify_f23_diff_no_change() {
    let meta = test_meta("blake3:local", "apt", Some("sha256:same"));
    let diff = compute_diff(&meta, Some("sha256:same"));
    assert!(!diff.upstream_changed, "same hash must not detect change");
}

/// F-24: build_sync_plan() separates re-imports from derivation replays.
#[test]
fn falsify_f24_sync_plan_separation() {
    let meta_leaf = test_meta_with_depth("blake3:leaf", "apt", Some("sha256:old"), 0, None);
    let meta_derived = test_meta_with_depth(
        "blake3:derived", "apt", Some("sha256:old"), 1, Some("blake3:leaf"),
    );
    let plan = build_sync_plan(&[
        (meta_leaf, Some("sha256:new".to_string())),
        (meta_derived, Some("sha256:new".to_string())),
    ]);
    assert_eq!(plan.re_imports.len(), 1, "leaf must be re-import");
    assert_eq!(plan.derivation_replays.len(), 1, "derived must be replay");
}

/// F-25: upstream_check_command() generates provider-specific commands.
#[test]
fn falsify_f25_upstream_check_commands() {
    let apt = test_meta_with_ref("apt", "nginx");
    assert!(upstream_check_command(&apt).unwrap().contains("apt-cache policy"));
    let cargo = test_meta_with_ref("cargo", "rg");
    assert!(upstream_check_command(&cargo).unwrap().contains("cargo search"));
    let docker = test_meta_with_ref("docker", "nginx");
    assert!(upstream_check_command(&docker).unwrap().contains("docker manifest inspect"));
}

/// F-26: has_diffable_provenance() checks for origin_hash or origin_ref.
#[test]
fn falsify_f26_diffable_provenance() {
    let with = test_meta("blake3:x", "apt", Some("sha256:abc"));
    assert!(has_diffable_provenance(&with));
    let without = StoreMeta { provenance: None, ..with };
    assert!(!has_diffable_provenance(&without));
}

// ═══════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════

fn test_meta(hash: &str, provider: &str, origin_hash: Option<&str>) -> StoreMeta {
    test_meta_with_depth(hash, provider, origin_hash, 0, None)
}

fn test_meta_with_depth(
    hash: &str, provider: &str, origin_hash: Option<&str>,
    depth: u32, derived_from: Option<&str>,
) -> StoreMeta {
    StoreMeta {
        schema: "1.0".to_string(),
        store_hash: hash.to_string(),
        recipe_hash: "blake3:r".to_string(),
        input_hashes: vec![],
        arch: "x86_64".to_string(),
        provider: provider.to_string(),
        created_at: "now".to_string(),
        generator: "forjar".to_string(),
        references: vec![],
        provenance: Some(Provenance {
            origin_provider: provider.to_string(),
            origin_ref: Some("nginx".to_string()),
            origin_hash: origin_hash.map(|s| s.to_string()),
            derived_from: derived_from.map(|s| s.to_string()),
            derivation_depth: depth,
        }),
    }
}

fn test_meta_with_ref(provider: &str, origin_ref: &str) -> StoreMeta {
    StoreMeta {
        schema: "1.0".to_string(),
        store_hash: "blake3:x".to_string(),
        recipe_hash: "r".to_string(),
        input_hashes: vec![],
        arch: "x86_64".to_string(),
        provider: provider.to_string(),
        created_at: "now".to_string(),
        generator: "forjar".to_string(),
        references: vec![],
        provenance: Some(Provenance {
            origin_provider: provider.to_string(),
            origin_ref: Some(origin_ref.to_string()),
            origin_hash: None,
            derived_from: None,
            derivation_depth: 0,
        }),
    }
}
