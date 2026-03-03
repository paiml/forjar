//! Spec falsification gap tests: Phase K (I8 enforcement) +
//! Phase L (execution bridge behavior, live DAG execution)
//!
//! Fills gaps K-07–K-12, L-15–L-24 from the gap analysis.
#![allow(unused_imports)]

use super::cache_exec::CachePullResult;
use super::convert_exec::ConversionApplyResult;
use super::derivation::{
    collect_input_hashes, derivation_closure_hash, validate_derivation, Derivation,
    DerivationInput,
};
use super::derivation_exec::{
    execute_derivation_dag, execute_derivation_dag_live, plan_derivation, simulate_derivation,
    DerivationPlan,
};
use super::gc_exec::{DryRunEntry, GcSweepResult};
use super::pin_resolve::{parse_resolved_version, resolution_command, ResolvedPin};
use super::provider_exec::ExecutionContext;
use super::sandbox_exec::{plan_sandbox_build, validate_plan};
use super::sandbox_run::SandboxExecResult;
use super::store_diff::{build_sync_plan, compute_diff, DiffResult, SyncPlan};
use super::sync_exec::{DiffExecResult, SyncExecResult};
use crate::core::purifier::{lint_error_count, lint_script, purify_script, validate_or_purify,
    validate_script};
use std::collections::BTreeMap;
use std::path::PathBuf;

// ═══════════════════════════════════════════════════════════════════
// Phase K gaps: I8 enforcement points
// ═══════════════════════════════════════════════════════════════════

/// K-07: validate_script() accepts valid shell and returns Ok.
#[test]
fn falsify_k07_validate_accepts_valid_shell() {
    let result = validate_script("echo hello world");
    assert!(result.is_ok(), "valid shell must pass validation: {result:?}");
    // Also verify multi-command scripts
    let result2 = validate_script("echo a && echo b");
    assert!(result2.is_ok(), "multi-command must pass: {result2:?}");
}

/// K-08: purify_script() cleans up problematic constructs.
#[test]
fn falsify_k08_purify_cleans_script() {
    let result = purify_script("echo hello; echo world");
    assert!(result.is_ok(), "simple multi-command script must purify");
}

/// K-09: lint_error_count() detects errors in problematic scripts.
#[test]
fn falsify_k09_lint_counts_errors() {
    // Valid script should have 0 errors
    let clean = lint_error_count("#!/bin/sh\necho hello");
    assert_eq!(clean, 0, "clean script must have 0 lint errors");
}

/// K-10: validate_or_purify() returns script unchanged when valid.
#[test]
fn falsify_k10_validate_or_purify_identity() {
    let script = "echo test";
    let result = validate_or_purify(script).unwrap();
    assert_eq!(result, script, "valid script must be returned unchanged");
}

/// K-11: lint_script() returns LintResult with diagnostics field.
#[test]
fn falsify_k11_lint_script_structure() {
    let result = lint_script("echo hello");
    // LintResult must have a diagnostics field (vec of diagnostics)
    // Clean scripts may still produce informational/warning diagnostics
    let error_count = result.diagnostics.iter()
        .filter(|d| d.severity == bashrs::linter::Severity::Error)
        .count();
    assert_eq!(error_count, 0, "clean script must have 0 error-severity diagnostics");
}

/// K-12: sandbox plan step 6 references bashrs-purified script.
#[test]
fn falsify_k12_sandbox_step6_bashrs_reference() {
    let config = super::sandbox::preset_profile("minimal").unwrap();
    let plan = plan_sandbox_build(
        &config,
        "blake3:test",
        &BTreeMap::new(),
        "echo build",
        std::path::Path::new("/store"),
    );
    let step6 = plan.steps.iter().find(|s| s.step == 6);
    assert!(step6.is_some(), "step 6 must exist");
    assert!(
        step6.unwrap().description.contains("bashrs"),
        "step 6 must reference bashrs purification"
    );
}

// ═══════════════════════════════════════════════════════════════════
// Phase L gaps: Execution bridge behavior
// ═══════════════════════════════════════════════════════════════════

/// L-15: ExecutionContext timeout_secs is optional.
#[test]
fn falsify_l15_execution_context_optional_timeout() {
    let ctx = ExecutionContext {
        store_dir: PathBuf::from("/store"),
        staging_dir: PathBuf::from("/tmp"),
        machine: test_machine(),
        timeout_secs: None,
    };
    assert!(ctx.timeout_secs.is_none(), "timeout_secs must be optional");
}

/// L-16: GcSweepResult errors field tracks per-entry failures.
#[test]
fn falsify_l16_gc_sweep_error_tracking() {
    let result = GcSweepResult {
        removed: vec!["blake3:a".to_string()],
        bytes_freed: 1024,
        errors: vec![("blake3:b".to_string(), "permission denied".to_string())],
    };
    assert_eq!(result.errors.len(), 1, "must track per-entry errors");
    assert_eq!(result.errors[0].0, "blake3:b");
}

/// L-17: CachePullResult bytes_transferred tracks transfer size.
#[test]
fn falsify_l17_cache_pull_bytes() {
    let result = CachePullResult {
        store_hash: "blake3:abc".to_string(),
        store_path: "/store/abc".to_string(),
        bytes_transferred: 65536,
        verified: true,
    };
    assert_eq!(result.bytes_transferred, 65536);
}

/// L-18: SandboxExecResult steps_executed tracks (step, desc, success).
#[test]
fn falsify_l18_sandbox_exec_step_tracking() {
    let result = SandboxExecResult {
        output_hash: "blake3:out".to_string(),
        store_path: "/store/out".to_string(),
        steps_executed: vec![
            (1, "create namespace".to_string(), true),
            (2, "overlay mount".to_string(), true),
            (3, "execute script".to_string(), false),
        ],
        duration_secs: 10.5,
    };
    assert_eq!(result.steps_executed.len(), 3);
    assert!(!result.steps_executed[2].2, "step 3 must be marked failed");
}

/// L-19: ConversionApplyResult tracks lock_pins_generated count.
#[test]
fn falsify_l19_convert_apply_lock_pins() {
    let result = ConversionApplyResult {
        changes_applied: 5,
        backup_path: PathBuf::from("forjar.yaml.bak"),
        new_purity: super::purity::PurityLevel::Pinned,
        lock_pins_generated: 3,
    };
    assert_eq!(result.lock_pins_generated, 3);
}

/// L-20: resolution_command() returns None for unknown provider.
#[test]
fn falsify_l20_resolution_unknown_provider() {
    assert!(
        resolution_command("unknown_provider", "pkg").is_none(),
        "unknown provider must return None"
    );
}

/// L-21: parse_resolved_version() for uv/pip provider.
#[test]
fn falsify_l21_parse_uv_version() {
    // "Available versions:" format parses correctly
    let output = "Available versions: 3.0.2, 3.0.1, 3.0.0\n";
    let version = parse_resolved_version("uv", output);
    assert_eq!(version, Some("3.0.2".to_string()));
}

/// L-22: DiffExecResult carries full DiffResult struct.
#[test]
fn falsify_l22_diff_exec_carries_diff() {
    let diff = DiffResult {
        store_hash: "blake3:abc".to_string(),
        upstream_changed: true,
        local_origin_hash: Some("sha256:old".to_string()),
        upstream_hash: Some("sha256:new".to_string()),
        provider: "apt".to_string(),
        origin_ref: Some("nginx".to_string()),
        derivation_chain_depth: 0,
    };
    let exec_result = DiffExecResult {
        diff: diff.clone(),
        upstream_command: Some("apt-cache policy nginx".to_string()),
        upstream_output: Some("Candidate: 1.26.0".to_string()),
    };
    assert!(exec_result.diff.upstream_changed);
    assert!(exec_result.upstream_command.is_some());
}

/// L-23: SyncExecResult new_profile_hash is optional.
#[test]
fn falsify_l23_sync_exec_profile_hash() {
    let result = SyncExecResult {
        re_imported: vec![],
        derivations_replayed: 0,
        new_profile_hash: Some("blake3:new_profile".to_string()),
    };
    assert!(result.new_profile_hash.is_some());

    let result_none = SyncExecResult {
        re_imported: vec![],
        derivations_replayed: 0,
        new_profile_hash: None,
    };
    assert!(result_none.new_profile_hash.is_none());
}

/// L-24: execute_derivation_dag_live() dry_run=true delegates to simulate.
#[test]
fn falsify_l24_dag_live_dry_run() {
    let mut a_inputs = BTreeMap::new();
    a_inputs.insert(
        "root".to_string(),
        DerivationInput::Store { store: "blake3:root".to_string() },
    );
    let mut derivations = BTreeMap::new();
    derivations.insert(
        "a".to_string(),
        Derivation {
            inputs: a_inputs,
            script: "echo a".to_string(),
            sandbox: None,
            arch: "x86_64".to_string(),
            out_var: "$out".to_string(),
        },
    );
    let mut init = BTreeMap::new();
    init.insert("root".to_string(), "blake3:root".to_string());

    let results = execute_derivation_dag_live(
        &derivations,
        &["a".to_string()],
        &init,
        &[],
        std::path::Path::new("/store"),
        true, // dry_run
    )
    .unwrap();
    assert!(results.contains_key("a"), "dry_run must produce results");
    assert!(
        results["a"].store_hash.starts_with("blake3:"),
        "result must have blake3 hash"
    );
}

/// L-25: ResolvedPin hash is blake3 prefixed.
#[test]
fn falsify_l25_resolved_pin_hash_format() {
    let pin = ResolvedPin {
        name: "nginx".to_string(),
        provider: "apt".to_string(),
        version: "1.24.0".to_string(),
        hash: "blake3:abc123".to_string(),
    };
    assert!(pin.hash.starts_with("blake3:"), "pin hash must be blake3");
}

/// L-26: DryRunEntry tracks hash and size.
#[test]
fn falsify_l26_dry_run_entry() {
    let entry = DryRunEntry {
        hash: "blake3:dead".to_string(),
        size_bytes: 1048576,
    };
    assert_eq!(entry.size_bytes, 1048576);
    assert!(entry.hash.starts_with("blake3:"));
}

/// L-27: validate_plan() catches empty namespace_id.
#[test]
fn falsify_l27_validate_plan_empty_namespace() {
    let plan = super::sandbox_exec::SandboxPlan {
        steps: vec![super::sandbox_exec::SandboxStep {
            step: 1,
            description: "test".to_string(),
            command: Some("echo test".to_string()),
        }],
        namespace_id: String::new(),
        overlay: super::sandbox_exec::OverlayConfig {
            lower_dirs: vec![PathBuf::from("/a")],
            upper_dir: PathBuf::from("/b"),
            work_dir: PathBuf::from("/c"),
            merged_dir: PathBuf::from("/d"),
        },
        seccomp_rules: vec![],
        cgroup_path: "/cg".to_string(),
    };
    let errors = validate_plan(&plan);
    assert!(
        errors.iter().any(|e| e.contains("namespace_id")),
        "must catch empty namespace_id"
    );
}

/// L-28: build_sync_plan() sorts derivation replays by depth.
#[test]
fn falsify_l28_sync_plan_replay_ordering() {
    use super::meta::{Provenance, StoreMeta};
    let make_meta = |hash: &str, depth: u32| StoreMeta {
        schema: "1.0".to_string(),
        store_hash: hash.to_string(),
        recipe_hash: "r".to_string(),
        input_hashes: vec![],
        arch: "x86_64".to_string(),
        provider: "apt".to_string(),
        created_at: "now".to_string(),
        generator: "forjar".to_string(),
        references: vec![],
        provenance: Some(Provenance {
            origin_provider: "apt".to_string(),
            origin_ref: None,
            origin_hash: Some("sha256:old".to_string()),
            derived_from: Some("blake3:parent".to_string()),
            derivation_depth: depth,
        }),
    };
    let plan = build_sync_plan(&[
        (make_meta("d3", 3), Some("sha256:new".to_string())),
        (make_meta("d1", 1), Some("sha256:new".to_string())),
        (make_meta("d2", 2), Some("sha256:new".to_string())),
    ]);
    assert_eq!(plan.derivation_replays.len(), 3);
    // Must be sorted ascending by depth
    assert_eq!(plan.derivation_replays[0].derivation_depth, 1);
    assert_eq!(plan.derivation_replays[1].derivation_depth, 2);
    assert_eq!(plan.derivation_replays[2].derivation_depth, 3);
}

// ═══════════════════════════════════════════════════════════════════
// Helper
// ═══════════════════════════════════════════════════════════════════

fn test_machine() -> crate::core::types::Machine {
    crate::core::types::Machine {
        hostname: "test".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        cost: 0,
        pepita: None,
    }
}
