//! Tests for scoring v2 — runtime dimensions, grades, formatting.

use super::scoring::*;
use super::tests_scoring::{full_runtime, minimal_config, minimal_resource, static_input};
use super::types::ResourceType;

// ============================================================================
// Correctness dimension tests (v2: 35% weight)
// ============================================================================

#[test]
fn correctness_no_runtime_is_zero() {
    let config = minimal_config();
    let input = static_input();
    let result = compute(&config, &input);
    let cor = result.dimensions.iter().find(|d| d.code == "COR").unwrap();
    assert_eq!(cor.score, 0);
    assert_eq!(cor.weight, 0.35, "v2 COR weight should be 35%");
}

#[test]
fn correctness_full_runtime_is_95() {
    let config = minimal_config();
    let input = ScoringInput {
        status: "qualified".to_string(),
        idempotency: "strong".to_string(),
        budget_ms: 0,
        runtime: Some(full_runtime()),
        raw_yaml: None,
    };
    let result = compute(&config, &input);
    let cor = result.dimensions.iter().find(|d| d.code == "COR").unwrap();
    // v2: 15+15+40+15+10 = 95
    assert_eq!(cor.score, 95);
}

#[test]
fn correctness_warnings_reduce_score() {
    let config = minimal_config();
    let mut rt = full_runtime();
    rt.warning_count = 5;
    let input = ScoringInput {
        status: "qualified".to_string(),
        idempotency: "strong".to_string(),
        budget_ms: 0,
        runtime: Some(rt),
        raw_yaml: None,
    };
    let result = compute(&config, &input);
    let cor = result.dimensions.iter().find(|d| d.code == "COR").unwrap();
    // 95 - 10 = 85
    assert_eq!(cor.score, 85);
}

// ============================================================================
// Idempotency dimension tests (v2: 35% weight)
// ============================================================================

#[test]
fn idempotency_no_runtime_is_zero() {
    let config = minimal_config();
    let input = static_input();
    let result = compute(&config, &input);
    let idm = result.dimensions.iter().find(|d| d.code == "IDM").unwrap();
    // v2: no static-only idempotency bonus
    assert_eq!(idm.score, 0);
    assert_eq!(idm.weight, 0.35, "v2 IDM weight should be 35%");
}

#[test]
fn idempotency_full_runtime_strong() {
    let config = minimal_config();
    let input = ScoringInput {
        status: "qualified".to_string(),
        idempotency: "strong".to_string(),
        budget_ms: 0,
        runtime: Some(full_runtime()),
        raw_yaml: None,
    };
    let result = compute(&config, &input);
    let idm = result.dimensions.iter().find(|d| d.code == "IDM").unwrap();
    // v2: 25+25+20+20 = 90
    assert_eq!(idm.score, 90);
}

#[test]
fn idempotency_weak_class_runtime() {
    let config = minimal_config();
    let input = ScoringInput {
        status: "qualified".to_string(),
        idempotency: "weak".to_string(),
        budget_ms: 0,
        runtime: Some(full_runtime()),
        raw_yaml: None,
    };
    let result = compute(&config, &input);
    let idm = result.dimensions.iter().find(|d| d.code == "IDM").unwrap();
    // 25+25+20+10 = 80
    assert_eq!(idm.score, 80);
}

#[test]
fn idempotency_changed_on_reapply_penalty() {
    let config = minimal_config();
    let mut rt = full_runtime();
    rt.changed_on_reapply = 3;
    rt.zero_changes_on_reapply = false;
    let input = ScoringInput {
        status: "qualified".to_string(),
        idempotency: "strong".to_string(),
        budget_ms: 0,
        runtime: Some(rt),
        raw_yaml: None,
    };
    let result = compute(&config, &input);
    let idm = result.dimensions.iter().find(|d| d.code == "IDM").unwrap();
    // 25 + 0 (not zero) + 20 + 20 - 30 (3*10) = 35
    assert_eq!(idm.score, 35);
}

// ============================================================================
// Performance dimension tests (v2: 30% weight)
// ============================================================================

#[test]
fn performance_no_budget_is_zero() {
    let config = minimal_config();
    let input = ScoringInput {
        status: "qualified".to_string(),
        idempotency: "strong".to_string(),
        budget_ms: 0,
        runtime: Some(full_runtime()),
        raw_yaml: None,
    };
    let result = compute(&config, &input);
    let prf = result.dimensions.iter().find(|d| d.code == "PRF").unwrap();
    assert_eq!(prf.score, 0);
    assert_eq!(prf.weight, 0.30, "v2 PRF weight should be 30%");
}

#[test]
fn performance_within_budget() {
    let config = minimal_config();
    let mut rt = full_runtime();
    rt.first_apply_ms = 3000;
    rt.second_apply_ms = 100;
    let input = ScoringInput {
        status: "qualified".to_string(),
        idempotency: "strong".to_string(),
        budget_ms: 6000,
        runtime: Some(rt),
        raw_yaml: None,
    };
    let result = compute(&config, &input);
    let prf = result.dimensions.iter().find(|d| d.code == "PRF").unwrap();
    assert_eq!(prf.score, 100);
}

// ============================================================================
// Report formatting tests
// ============================================================================

#[test]
fn format_report_contains_v2_sections() {
    let config = minimal_config();
    let input = static_input();
    let result = compute(&config, &input);
    let report = format_score_report(&result);
    assert!(report.contains("v2"), "report should mention v2");
    assert!(report.contains("Static Grade"));
    assert!(report.contains("Runtime Grade"));
    assert!(report.contains("Overall"));
}

#[test]
fn format_report_hard_fail_shows_reason() {
    let config = minimal_config();
    let input = ScoringInput {
        status: "blocked".to_string(),
        idempotency: "strong".to_string(),
        budget_ms: 0,
        runtime: None,
        raw_yaml: None,
    };
    let result = compute(&config, &input);
    let report = format_score_report(&result);
    assert!(report.contains("HARD FAIL"));
    assert!(report.contains("blocked"));
}

// ============================================================================
// Composite and grade integration
// ============================================================================

#[test]
fn grade_f_for_low_composite() {
    assert_eq!(determine_grade(30, 30), 'F');
}

#[test]
fn grade_d_for_composite_40_plus() {
    assert_eq!(determine_grade(45, 10), 'D');
}

#[test]
fn grade_c_for_composite_60_min_40() {
    assert_eq!(determine_grade(65, 45), 'C');
}

#[test]
fn grade_b_for_composite_75_min_60() {
    assert_eq!(determine_grade(80, 65), 'B');
}

#[test]
fn grade_a_for_composite_90_min_80() {
    assert_eq!(determine_grade(92, 85), 'A');
}

#[test]
fn grade_b_blocked_by_min_dimension() {
    assert_eq!(determine_grade(92, 70), 'B');
}

// ============================================================================
// Score bar rendering
// ============================================================================

#[test]
fn score_bar_full() {
    let bar = score_bar(100);
    assert_eq!(bar, "[####################]");
}

#[test]
fn score_bar_empty() {
    let bar = score_bar(0);
    assert_eq!(bar, "[....................]");
}

#[test]
fn score_bar_half() {
    let bar = score_bar(50);
    assert_eq!(bar, "[##########..........]");
}

// ============================================================================
// File-based scoring
// ============================================================================

#[test]
fn compute_from_file_invalid_path() {
    let input = static_input();
    let result = compute_from_file(std::path::Path::new("/nonexistent.yaml"), &input);
    assert!(result.is_err());
}

#[test]
fn compute_from_file_valid() {
    let yaml = "\
version: '1.0'
name: test-score
description: A test config
resources:
  cfg:
    type: file
    path: /etc/test.conf
    mode: '0644'
    owner: root
    content: hello
";
    let mut f = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(&mut f, yaml.as_bytes()).unwrap();
    std::io::Write::flush(&mut f).unwrap();

    let input = static_input();
    let result = compute_from_file(f.path(), &input).unwrap();
    assert!(result.static_composite > 0);
    assert_eq!(result.dimensions.len(), 8);
    let saf = result.dimensions.iter().find(|d| d.code == "SAF").unwrap();
    assert_eq!(saf.score, 100);
}

// ============================================================================
// Documentation edge cases
// ============================================================================

#[test]
fn documentation_empty_name_no_kebab_bonus() {
    let mut config = minimal_config();
    config.name = "".to_string();
    config.description = Some("A description".to_string());
    let input = static_input();
    let result = compute(&config, &input);
    let doc = result.dimensions.iter().find(|d| d.code == "DOC").unwrap();
    // 15 (description) only — no kebab bonus, no header metadata
    assert_eq!(doc.score, 15);
}

// ============================================================================
// Composability edge cases
// ============================================================================

#[test]
fn composability_with_includes() {
    let mut config = minimal_config();
    config.includes = vec!["base.yaml".to_string()];
    let input = static_input();
    let result = compute(&config, &input);
    let cmp = result.dimensions.iter().find(|d| d.code == "CMP").unwrap();
    assert_eq!(cmp.score, 10);
}

#[test]
fn composability_with_recipe_nesting() {
    let mut config = minimal_config();
    let mut res = minimal_resource(ResourceType::Recipe);
    res.recipe = Some("nested-recipe".to_string());
    config.resources.insert("nested".to_string(), res);
    let input = static_input();
    let result = compute(&config, &input);
    let cmp = result.dimensions.iter().find(|d| d.code == "CMP").unwrap();
    // v2: recipe nesting = 10
    assert_eq!(cmp.score, 10);
}

// ============================================================================
// Resilience edge cases
// ============================================================================

#[test]
fn resilience_dag_ratio_33_pct() {
    let mut config = minimal_config();
    // 3 resources, 1 with deps = 33% → +10 (v2: 30% threshold)
    let r1 = minimal_resource(ResourceType::Package);
    let r2 = minimal_resource(ResourceType::File);
    let mut r3 = minimal_resource(ResourceType::File);
    r3.depends_on = vec!["r1".to_string()];
    config.resources.insert("r1".to_string(), r1);
    config.resources.insert("r2".to_string(), r2);
    config.resources.insert("r3".to_string(), r3);

    let input = static_input();
    let result = compute(&config, &input);
    let res = result.dimensions.iter().find(|d| d.code == "RES").unwrap();
    // 0 (no continue_independent) + 0 (no ssh_retries) + 10 (dag 33%) = 10
    assert_eq!(res.score, 10);
}

#[test]
fn resilience_with_resource_hooks() {
    let mut config = minimal_config();
    let mut res = minimal_resource(ResourceType::File);
    res.pre_apply = Some("echo pre".to_string());
    config.resources.insert("f".to_string(), res);

    let input = static_input();
    let result = compute(&config, &input);
    let dim = result.dimensions.iter().find(|d| d.code == "RES").unwrap();
    // 10 (resource hooks)
    assert_eq!(dim.score, 10);
}

// ============================================================================
// v2 score version
// ============================================================================

#[test]
fn score_version_is_v2() {
    assert_eq!(SCORE_VERSION, "2.0");
}
