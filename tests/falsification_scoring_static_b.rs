#![allow(dead_code)]
#![allow(clippy::field_reassign_with_default)]
//! ForjarScore v2: CMP, grade thresholds, runtime, format_score_report, score_bar
//! (split from falsification_scoring_static).
//!
//! Usage: cargo test --test falsification_scoring_static_b

use forjar::core::scoring::{compute, format_score_report, score_bar, RuntimeData, ScoringInput};
use forjar::core::types::{ForjarConfig, Resource, ResourceType};
use std::collections::HashMap;

// ============================================================================
// Helpers
// ============================================================================

fn empty_config() -> ForjarConfig {
    ForjarConfig {
        version: "1.0".into(),
        name: "test".into(),
        ..Default::default()
    }
}

fn static_input() -> ScoringInput {
    ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 0,
        runtime: None,
        raw_yaml: Some(String::new()),
    }
}

fn perfect_runtime() -> RuntimeData {
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
        first_apply_ms: 100,
        second_apply_ms: 50,
    }
}

// ============================================================================
// CMP — Composability scoring
// ============================================================================

#[test]
fn cmp_empty_config_zero() {
    let config = empty_config();
    let result = compute(&config, &static_input());
    let cmp = result.dimensions.iter().find(|d| d.code == "CMP").unwrap();
    assert_eq!(cmp.score, 0);
}

#[test]
fn cmp_params_score() {
    let mut config = empty_config();
    config
        .params
        .insert("port".into(), serde_yaml_ng::Value::from(8080));

    let result = compute(&config, &static_input());
    let cmp = result.dimensions.iter().find(|d| d.code == "CMP").unwrap();
    assert!(
        cmp.score >= 15,
        "CMP={} should be >=15 with params",
        cmp.score
    );
}

#[test]
fn cmp_templates_score() {
    let mut config = empty_config();
    let mut r = Resource::default();
    r.resource_type = ResourceType::File;
    r.content = Some("port={{params.port}}".into());
    config.resources.insert("cfg".into(), r);

    let result = compute(&config, &static_input());
    let cmp = result.dimensions.iter().find(|d| d.code == "CMP").unwrap();
    assert!(
        cmp.score >= 10,
        "CMP={} should be >=10 with templates",
        cmp.score
    );
}

#[test]
fn cmp_tags_score() {
    let mut config = empty_config();
    let mut r = Resource::default();
    r.resource_type = ResourceType::Package;
    r.tags = vec!["web".into()];
    config.resources.insert("nginx".into(), r);

    let result = compute(&config, &static_input());
    let cmp = result.dimensions.iter().find(|d| d.code == "CMP").unwrap();
    assert!(
        cmp.score >= 15,
        "CMP={} should be >=15 with tags",
        cmp.score
    );
}

#[test]
fn cmp_includes_score() {
    let mut config = empty_config();
    config.includes = vec!["common.yaml".into()];

    let result = compute(&config, &static_input());
    let cmp = result.dimensions.iter().find(|d| d.code == "CMP").unwrap();
    assert!(
        cmp.score >= 10,
        "CMP={} should be >=10 with includes",
        cmp.score
    );
}

#[test]
fn cmp_secrets_template_score() {
    let mut config = empty_config();
    let mut r = Resource::default();
    r.resource_type = ResourceType::File;
    r.content = Some("password={{ secrets.db_pass }}".into());
    config.resources.insert("secret-file".into(), r);

    let result = compute(&config, &static_input());
    let cmp = result.dimensions.iter().find(|d| d.code == "CMP").unwrap();
    // templates + secrets = 10 + 5
    assert!(
        cmp.score >= 5,
        "CMP={} should be >=5 with secrets template",
        cmp.score
    );
}

// ============================================================================
// Grade thresholds
// ============================================================================

#[test]
fn blocked_status_hard_fail() {
    let config = empty_config();
    let input = ScoringInput {
        status: "blocked".into(),
        idempotency: "strong".into(),
        budget_ms: 0,
        runtime: None,
        raw_yaml: Some(String::new()),
    };
    let result = compute(&config, &input);
    assert!(result.hard_fail);
    assert!(result.hard_fail_reason.is_some());
    assert!(result.grade.contains("blocked"));
}

#[test]
fn pending_status_no_hard_fail() {
    let config = empty_config();
    let input = ScoringInput {
        status: "pending".into(),
        idempotency: "strong".into(),
        budget_ms: 0,
        runtime: None,
        raw_yaml: Some(String::new()),
    };
    let result = compute(&config, &input);
    assert!(!result.hard_fail);
    assert!(result.grade.contains("pending"));
}

// ============================================================================
// Runtime grade display
// ============================================================================

#[test]
fn runtime_grade_displayed_when_present() {
    let config = empty_config();
    let input = ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 0,
        runtime: Some(perfect_runtime()),
        raw_yaml: Some(String::new()),
    };
    let result = compute(&config, &input);
    assert!(result.runtime_grade.is_some());
    assert!(result.grade.contains('/'));
    // Grade should be like "F/A" (empty config has low static but perfect runtime)
}

#[test]
fn runtime_composite_present() {
    let config = empty_config();
    let input = ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 5000, // Need budget > 0 for PRF scoring
        runtime: Some(perfect_runtime()),
        raw_yaml: Some(String::new()),
    };
    let result = compute(&config, &input);
    assert!(result.runtime_composite.is_some());
    let rc = result.runtime_composite.unwrap();
    assert!(
        rc >= 80,
        "runtime composite={rc} should be >=80 with perfect runtime and budget"
    );
}

// ============================================================================
// Static weights
// ============================================================================

#[test]
fn static_dimension_weights() {
    let config = empty_config();
    let result = compute(&config, &static_input());

    let saf = result.dimensions.iter().find(|d| d.code == "SAF").unwrap();
    let obs = result.dimensions.iter().find(|d| d.code == "OBS").unwrap();
    let doc = result.dimensions.iter().find(|d| d.code == "DOC").unwrap();
    let res = result.dimensions.iter().find(|d| d.code == "RES").unwrap();
    let cmp = result.dimensions.iter().find(|d| d.code == "CMP").unwrap();

    assert!((saf.weight - 0.25).abs() < f64::EPSILON);
    assert!((obs.weight - 0.20).abs() < f64::EPSILON);
    assert!((doc.weight - 0.15).abs() < f64::EPSILON);
    assert!((res.weight - 0.20).abs() < f64::EPSILON);
    assert!((cmp.weight - 0.20).abs() < f64::EPSILON);
}

// ============================================================================
// format_score_report
// ============================================================================

#[test]
fn format_score_report_has_version() {
    let config = empty_config();
    let result = compute(&config, &static_input());
    let report = format_score_report(&result);
    assert!(report.contains("Forjar Score v2"));
}

#[test]
fn format_score_report_has_static_grade() {
    let config = empty_config();
    let result = compute(&config, &static_input());
    let report = format_score_report(&result);
    assert!(report.contains("Static Grade:"));
}

#[test]
fn format_score_report_has_runtime_pending() {
    let config = empty_config();
    let result = compute(&config, &static_input());
    let report = format_score_report(&result);
    assert!(report.contains("Runtime Grade: pending"));
}

#[test]
fn format_score_report_has_runtime_grade() {
    let config = empty_config();
    let input = ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 0,
        runtime: Some(perfect_runtime()),
        raw_yaml: Some(String::new()),
    };
    let result = compute(&config, &input);
    let report = format_score_report(&result);
    assert!(report.contains("Runtime Grade:"));
    assert!(!report.contains("pending"));
}

#[test]
fn format_score_report_has_dimension_codes() {
    let config = empty_config();
    let result = compute(&config, &static_input());
    let report = format_score_report(&result);
    assert!(report.contains("SAF"));
    assert!(report.contains("OBS"));
    assert!(report.contains("DOC"));
    assert!(report.contains("RES"));
    assert!(report.contains("CMP"));
}

#[test]
fn format_score_report_hard_fail() {
    let config = empty_config();
    let input = ScoringInput {
        status: "blocked".into(),
        idempotency: "strong".into(),
        budget_ms: 0,
        runtime: None,
        raw_yaml: Some(String::new()),
    };
    let result = compute(&config, &input);
    let report = format_score_report(&result);
    assert!(report.contains("HARD FAIL"));
}

// ============================================================================
// score_bar
// ============================================================================

#[test]
fn score_bar_zero() {
    let bar = score_bar(0);
    assert_eq!(bar, "[....................]");
}

#[test]
fn score_bar_hundred() {
    let bar = score_bar(100);
    assert_eq!(bar, "[####################]");
}

#[test]
fn score_bar_fifty() {
    let bar = score_bar(50);
    assert_eq!(bar, "[##########..........]");
}

#[test]
fn score_bar_length_constant() {
    for s in [0, 10, 25, 50, 75, 90, 100] {
        let bar = score_bar(s);
        assert_eq!(bar.len(), 22, "bar length for score={s} should be 22");
    }
}

// ============================================================================
// DimensionScore names
// ============================================================================

#[test]
fn dimension_names_correct() {
    let config = empty_config();
    let result = compute(&config, &static_input());

    let name_map: HashMap<&str, &str> =
        result.dimensions.iter().map(|d| (d.code, d.name)).collect();
    assert_eq!(name_map["SAF"], "Safety");
    assert_eq!(name_map["OBS"], "Observability");
    assert_eq!(name_map["DOC"], "Documentation");
    assert_eq!(name_map["RES"], "Resilience");
    assert_eq!(name_map["CMP"], "Composability");
    assert_eq!(name_map["COR"], "Correctness");
    assert_eq!(name_map["IDM"], "Idempotency");
    assert_eq!(name_map["PRF"], "Performance");
}
