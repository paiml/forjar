//! Popperian falsification tests for FJ-2000 through FJ-2803.
//!
//! Each test directly implements a rejection criterion from the platform spec.
//! If a test passes, the implementation survives falsification. If it fails,
//! the spec claim is falsified and must be corrected.

#![allow(clippy::field_reassign_with_default)]
#![allow(unused_imports)]

use forjar::core::scoring::{compute, RuntimeData, ScoringInput, SCORE_VERSION};
use forjar::core::types::{
    FailurePolicy, ForjarConfig, Machine, OutputValue, Resource, ResourceType,
};
use tempfile::TempDir;

fn base_config() -> ForjarConfig {
    let mut config = ForjarConfig::default();
    config.name = "test-recipe".into();
    config.version = "1.0".into();
    config
}

fn base_input() -> ScoringInput {
    ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 60_000,
        runtime: None,
        raw_yaml: None,
    }
}

fn full_runtime() -> RuntimeData {
    RuntimeData {
        validate_pass: true,
        plan_pass: true,
        first_apply_pass: true,
        second_apply_pass: true,
        zero_changes_on_reapply: true,
        hash_stable: true,
        all_resources_converged: true,
        state_lock_written: true,
        warning_count: 0,
        changed_on_reapply: 0,
        first_apply_ms: 20_000,
        second_apply_ms: 500,
    }
}

#[test]
fn f_2803_doc_copypaste_vs_distinct() {
    let mut config = base_config();
    config.description = Some("A recipe".into());

    let input_cp = ScoringInput {
        raw_yaml: Some("# Deploy file\n# Deploy file\n# Deploy file\n# Deploy file\nversion: \"1.0\"\nname: test-recipe\n".into()),
        ..base_input()
    };
    let input_distinct = ScoringInput {
        raw_yaml: Some("# Install nginx\n# Configure SSL\n# Set up proxy\n# Enable monitoring\n# Apply firewall\nversion: \"1.0\"\nname: test-recipe\n".into()),
        ..base_input()
    };

    let doc_cp = compute(&config, &input_cp)
        .dimensions
        .iter()
        .find(|d| d.code == "DOC")
        .unwrap()
        .score;
    let doc_distinct = compute(&config, &input_distinct)
        .dimensions
        .iter()
        .find(|d| d.code == "DOC")
        .unwrap()
        .score;

    assert!(
        doc_distinct >= doc_cp,
        "distinct ({doc_distinct}) >= copy-paste ({doc_cp})"
    );
}

// --- RES Dimension ---

/// FJ-2803 RES: Tagged independence must score RES >= 60.
#[test]
fn f_2803_res_tagged_independence_at_least_60() {
    let mut config = base_config();
    config.policy.failure = FailurePolicy::ContinueIndependent;
    config.policy.ssh_retries = 3;
    config.policy.pre_apply = Some("echo pre".into());
    config.policy.post_apply = Some("echo post".into());
    config.policy.deny_paths = vec!["/etc/shadow".into()];

    for i in 0..5 {
        let mut r = Resource::default();
        r.resource_type = ResourceType::File;
        r.tags = vec!["audit".into()];
        r.resource_group = Some("security".into());
        r.mode = Some("0644".into());
        config.resources.insert(format!("res-{i}"), r);
    }

    let result = compute(&config, &base_input());
    let res = result.dimensions.iter().find(|d| d.code == "RES").unwrap();
    assert!(
        res.score >= 60,
        "tagged independence RES >= 60, got: {}",
        res.score
    );
}

/// FJ-2803 RES: Deep DAG (depends_on) must also score RES >= 60.
#[test]
fn f_2803_res_deep_dag_at_least_60() {
    let mut config = base_config();
    config.policy.failure = FailurePolicy::ContinueIndependent;
    config.policy.ssh_retries = 3;
    config.policy.pre_apply = Some("echo pre".into());
    config.policy.post_apply = Some("echo post".into());
    config.policy.deny_paths = vec!["/etc/shadow".into()];

    let mut r0 = Resource::default();
    r0.resource_type = ResourceType::Package;
    config.resources.insert("base".into(), r0);

    for i in 0..4 {
        let mut r = Resource::default();
        r.resource_type = ResourceType::File;
        r.depends_on = vec!["base".into()];
        r.mode = Some("0644".into());
        config.resources.insert(format!("dep-{i}"), r);
    }

    let result = compute(&config, &base_input());
    let res = result.dimensions.iter().find(|d| d.code == "RES").unwrap();
    assert!(res.score >= 60, "deep DAG RES >= 60, got: {}", res.score);
}

// --- CMP Dimension ---

/// FJ-2803 CMP: Fully composable recipe must score CMP >= 85.
#[test]
fn f_2803_cmp_full_composability_at_least_85() {
    let mut config = base_config();
    config
        .params
        .insert("port".into(), serde_yaml_ng::Value::String("8080".into()));
    config.includes = vec!["base.yaml".into()];

    config
        .machines
        .insert("web".into(), Machine::ssh("web-01", "10.0.0.1", "root"));
    config
        .machines
        .insert("db".into(), Machine::ssh("db-01", "10.0.0.2", "root"));

    let mut r = Resource::default();
    r.resource_type = ResourceType::File;
    r.tags = vec!["web".into()];
    r.resource_group = Some("frontend".into());
    r.content = Some("port={{ port }} secret={{ secrets.api_key }}".into());
    config.resources.insert("config".into(), r);

    let mut recipe = Resource::default();
    recipe.resource_type = ResourceType::Recipe;
    config.resources.insert("nested".into(), recipe);

    let result = compute(&config, &base_input());
    let cmp = result.dimensions.iter().find(|d| d.code == "CMP").unwrap();
    assert!(
        cmp.score >= 85,
        "fully composable CMP >= 85, got: {}",
        cmp.score
    );
}

/// FJ-2803 CMP Boundary: No features must score CMP=0.
#[test]
fn f_2803_cmp_empty_scores_zero() {
    let result = compute(&base_config(), &base_input());
    let cmp = result.dimensions.iter().find(|d| d.code == "CMP").unwrap();
    assert_eq!(cmp.score, 0, "empty CMP must be 0, got: {}", cmp.score);
}

// --- COR Dimension ---

/// FJ-2803 COR: Full convergence must score COR >= 90.
#[test]
fn f_2803_cor_full_convergence_at_least_90() {
    let mut input = base_input();
    input.runtime = Some(full_runtime());
    let result = compute(&base_config(), &input);
    let cor = result.dimensions.iter().find(|d| d.code == "COR").unwrap();
    assert!(cor.score >= 90, "full COR >= 90, got: {}", cor.score);
}

/// FJ-2803 COR: Nothing passing must score COR=0.
#[test]
fn f_2803_cor_zero_when_nothing_passes() {
    let rt = RuntimeData {
        validate_pass: false,
        plan_pass: false,
        first_apply_pass: false,
        second_apply_pass: false,
        zero_changes_on_reapply: false,
        hash_stable: false,
        all_resources_converged: false,
        state_lock_written: false,
        warning_count: 0,
        changed_on_reapply: 0,
        first_apply_ms: 0,
        second_apply_ms: 0,
    };
    let mut input = base_input();
    input.runtime = Some(rt);
    let result = compute(&base_config(), &input);
    let cor = result.dimensions.iter().find(|d| d.code == "COR").unwrap();
    assert_eq!(
        cor.score, 0,
        "COR=0 when nothing passes, got: {}",
        cor.score
    );
}

/// FJ-2803 COR: Partial failure + warnings must reduce score.
#[test]
fn f_2803_cor_partial_failure_reduces_score() {
    let mut rt = full_runtime();
    rt.all_resources_converged = false;
    rt.warning_count = 5;
    let mut input = base_input();
    input.runtime = Some(rt);
    let result = compute(&base_config(), &input);
    let cor = result.dimensions.iter().find(|d| d.code == "COR").unwrap();
    assert!(
        cor.score < 80,
        "partial failure COR < 80, got: {}",
        cor.score
    );
}

// --- IDM Dimension ---

/// FJ-2803 IDM: Strong idempotency with zero changes must score IDM >= 90.
#[test]
fn f_2803_idm_strong_zero_changes_at_least_90() {
    let mut input = base_input();
    input.idempotency = "strong".into();
    input.runtime = Some(full_runtime());
    let result = compute(&base_config(), &input);
    let idm = result.dimensions.iter().find(|d| d.code == "IDM").unwrap();
    assert!(idm.score >= 90, "strong IDM >= 90, got: {}", idm.score);
}

/// FJ-2803 IDM: Re-apply modifying 3 resources must score IDM < 50.
#[test]
fn f_2803_idm_reapply_changes_below_50() {
    let mut rt = full_runtime();
    rt.zero_changes_on_reapply = false;
    rt.hash_stable = false;
    rt.changed_on_reapply = 3;
    let mut input = base_input();
    input.idempotency = "eventual".into();
    input.runtime = Some(rt);
    let result = compute(&base_config(), &input);
    let idm = result.dimensions.iter().find(|d| d.code == "IDM").unwrap();
    assert!(idm.score < 50, "3 changes IDM < 50, got: {}", idm.score);
}

// --- PRF Dimension ---

/// FJ-2803 PRF: 33% of budget must score PRF >= 70.
#[test]
fn f_2803_prf_33pct_budget_at_least_70() {
    let mut rt = full_runtime();
    rt.first_apply_ms = 20_000;
    rt.second_apply_ms = 500;
    let mut input = base_input();
    input.budget_ms = 60_000;
    input.runtime = Some(rt);
    let result = compute(&base_config(), &input);
    let prf = result.dimensions.iter().find(|d| d.code == "PRF").unwrap();
    assert!(prf.score >= 70, "33% budget PRF >= 70, got: {}", prf.score);
}

/// FJ-2803 PRF: 200% of budget must score PRF <= 25.
#[test]
fn f_2803_prf_200pct_budget_at_most_25() {
    let mut rt = full_runtime();
    rt.first_apply_ms = 120_000;
    rt.second_apply_ms = 60_000;
    let mut input = base_input();
    input.budget_ms = 60_000;
    input.runtime = Some(rt);
    let result = compute(&base_config(), &input);
    let prf = result.dimensions.iter().find(|d| d.code == "PRF").unwrap();
    assert!(prf.score <= 25, "200% budget PRF <= 25, got: {}", prf.score);
}

// --- Cross-Dimension Falsification ---

/// FJ-2803 Monotonicity: Adding deny_paths must not decrease score.
#[test]
fn f_2803_monotonicity_deny_paths() {
    let result_base = compute(&base_config(), &base_input());

    let mut config_deny = base_config();
    config_deny.policy.deny_paths = vec!["/etc/shadow".into()];
    let result_deny = compute(&config_deny, &base_input());

    assert!(
        result_deny.static_composite >= result_base.static_composite,
        "deny_paths must not decrease: {} -> {}",
        result_base.static_composite,
        result_deny.static_composite
    );
}

/// FJ-2803: Score version is "2.0".
#[test]
fn f_2803_score_version_is_v2() {
    assert_eq!(SCORE_VERSION, "2.0");
}

/// FJ-2803: Blocked status triggers hard fail.
#[test]
fn f_2803_blocked_hard_fail() {
    let mut input = base_input();
    input.status = "blocked".into();
    let result = compute(&base_config(), &input);
    assert!(result.hard_fail);
    assert!(result.grade.contains("blocked"));
}

/// FJ-2803: Pending gets real static grade (v2 fix: not automatic F).
#[test]
fn f_2803_pending_real_static_grade() {
    let mut config = base_config();
    // Max out SAF: files with mode + owner + versioned packages
    let mut file = Resource::default();
    file.resource_type = ResourceType::File;
    file.mode = Some("0644".into());
    file.owner = Some("root".into());
    config.resources.insert("conf".into(), file);
    let mut pkg = Resource::default();
    pkg.resource_type = ResourceType::Package;
    pkg.version = Some("1.0".into());
    config.resources.insert("nginx".into(), pkg);

    // OBS: tripwire + lock_file + notify hooks + outputs
    config.policy.tripwire = true;
    config.policy.lock_file = true;
    config.policy.notify.on_success = Some("echo ok".into());
    config.policy.notify.on_failure = Some("echo fail".into());
    config.policy.notify.on_drift = Some("echo drift".into());
    let out = OutputValue {
        value: "v".into(),
        description: Some("desc".into()),
    };
    config.outputs.insert("r".into(), out);

    // DOC: description + params
    config.description = Some("Well-documented recipe".into());
    config
        .params
        .insert("a".into(), serde_yaml_ng::Value::String("1".into()));
    config
        .params
        .insert("b".into(), serde_yaml_ng::Value::String("2".into()));
    config
        .params
        .insert("c".into(), serde_yaml_ng::Value::String("3".into()));

    // RES: failure policy + retries + deny_paths
    config.policy.failure = FailurePolicy::ContinueIndependent;
    config.policy.ssh_retries = 3;
    config.policy.deny_paths = vec!["/etc/shadow".into()];

    // CMP: params + tags + includes
    config.includes = vec!["base.yaml".into()];
    if let Some(r) = config.resources.get_mut("conf") {
        r.tags = vec!["web".into()];
        r.resource_group = Some("frontend".into());
        r.content = Some("{{ secrets.key }}".into());
    }

    let mut input = ScoringInput {
        raw_yaml: Some("# Recipe: web-server\n# Tier: production\n# Idempotency: strong\n# Budget: 60s\nversion: \"1.0\"\nname: test-recipe\n".into()),
        ..base_input()
    };
    input.status = "pending".into();
    let result = compute(&config, &input);

    assert!(!result.hard_fail, "pending must NOT hard fail");
    assert!(result.grade.contains("pending"));
    assert_ne!(
        result.static_grade, 'F',
        "pending with rich config != F, got static_composite={}",
        result.static_composite
    );
}

/// FJ-2803: Two-tier display format: "X/Y" or "X/pending".
#[test]
fn f_2803_two_tier_format() {
    let mut input_rt = base_input();
    input_rt.runtime = Some(full_runtime());
    let r1 = compute(&base_config(), &input_rt);
    assert!(r1.grade.contains('/'), "with runtime: {}", r1.grade);

    let r2 = compute(&base_config(), &base_input());
    assert!(r2.grade.contains("/pending"), "no runtime: {}", r2.grade);
}

// ============================================================================
// FJ-2900: Score bar rendering
// ============================================================================

/// F-2900-1: Score bar renders correctly at boundaries.
#[test]
fn f_2900_1_score_bar_boundaries() {
    use forjar::core::scoring::score_bar;

    let b0 = score_bar(0);
    assert_eq!(b0.matches('#').count(), 0, "0 = no hashes: {b0}");

    let b100 = score_bar(100);
    assert_eq!(b100.matches('#').count(), 20, "100 = 20 hashes: {b100}");

    let b50 = score_bar(50);
    assert_eq!(b50.matches('#').count(), 10, "50 = 10 hashes: {b50}");
}

// ============================================================================
// FJ-3000: Defect Analysis — bashrs script purification
// ============================================================================

/// F-3000-1: Purifier validates scripts without crashing on edge cases.
#[test]
fn f_3000_1_purifier_handles_edge_cases() {
    use forjar::core::purifier::validate_script;

    // Empty script
    assert!(validate_script("").is_ok());

    // Simple valid script
    assert!(validate_script("echo hello").is_ok());

    // Multiple commands with &&
    assert!(validate_script("apt update && apt install -y nginx").is_ok());
}
