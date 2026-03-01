//! Tests for the scoring module — part 2: runtime dimensions, grades, formatting.

use super::scoring::*;
use super::tests_scoring::{minimal_config, full_runtime, static_input};

// ============================================================================
// Correctness dimension tests
// ============================================================================

#[test]
fn correctness_no_runtime_is_zero() {
    let config = minimal_config();
    let input = static_input();
    let result = compute(&config, &input);
    let cor = result.dimensions.iter().find(|d| d.code == "COR").unwrap();
    assert_eq!(cor.score, 0);
}

#[test]
fn correctness_full_runtime_is_100() {
    let config = minimal_config();
    let input = ScoringInput {
        status: "qualified".to_string(),
        idempotency: "strong".to_string(),
        budget_ms: 0,
        runtime: Some(full_runtime()),
    };
    let result = compute(&config, &input);
    let cor = result.dimensions.iter().find(|d| d.code == "COR").unwrap();
    assert_eq!(cor.score, 100);
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
    };
    let result = compute(&config, &input);
    let cor = result.dimensions.iter().find(|d| d.code == "COR").unwrap();
    assert_eq!(cor.score, 90);
}

// ============================================================================
// Idempotency dimension tests
// ============================================================================

#[test]
fn idempotency_strong_class_static_only() {
    let config = minimal_config();
    let input = ScoringInput {
        status: "qualified".to_string(),
        idempotency: "strong".to_string(),
        budget_ms: 0,
        runtime: None,
    };
    let result = compute(&config, &input);
    let idm = result.dimensions.iter().find(|d| d.code == "IDM").unwrap();
    assert_eq!(idm.score, 20);
}

#[test]
fn idempotency_full_runtime_strong() {
    let config = minimal_config();
    let input = ScoringInput {
        status: "qualified".to_string(),
        idempotency: "strong".to_string(),
        budget_ms: 0,
        runtime: Some(full_runtime()),
    };
    let result = compute(&config, &input);
    let idm = result.dimensions.iter().find(|d| d.code == "IDM").unwrap();
    assert_eq!(idm.score, 100);
}

// ============================================================================
// Performance dimension tests
// ============================================================================

#[test]
fn performance_no_budget_is_zero() {
    let config = minimal_config();
    let input = ScoringInput {
        status: "qualified".to_string(),
        idempotency: "strong".to_string(),
        budget_ms: 0,
        runtime: Some(full_runtime()),
    };
    let result = compute(&config, &input);
    let prf = result.dimensions.iter().find(|d| d.code == "PRF").unwrap();
    assert_eq!(prf.score, 0);
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
    };
    let result = compute(&config, &input);
    let prf = result.dimensions.iter().find(|d| d.code == "PRF").unwrap();
    assert_eq!(prf.score, 100);
}

// ============================================================================
// Report formatting tests
// ============================================================================

#[test]
fn format_report_contains_grade() {
    let config = minimal_config();
    let input = static_input();
    let result = compute(&config, &input);
    let report = format_score_report(&result);
    assert!(report.contains("Grade"));
    assert!(report.contains("Forjar Score:"));
}

#[test]
fn format_report_hard_fail_shows_reason() {
    let config = minimal_config();
    let input = ScoringInput {
        status: "blocked".to_string(),
        idempotency: "strong".to_string(),
        budget_ms: 0,
        runtime: None,
    };
    let result = compute(&config, &input);
    let report = format_score_report(&result);
    assert!(report.contains("HARD FAIL"));
    assert!(report.contains("blocked"));
}

// ============================================================================
// Documentation dimension tests
// ============================================================================

#[test]
fn documentation_with_description() {
    let mut config = minimal_config();
    config.description = Some("A great config for web servers".to_string());

    let input = static_input();
    let result = compute(&config, &input);
    let doc = result.dimensions.iter().find(|d| d.code == "DOC").unwrap();
    assert_eq!(doc.score, 45);
}

// ============================================================================
// Composite and grade integration
// ============================================================================

#[test]
fn composite_weighted_sum_correct() {
    let dims = vec![
        DimensionScore { code: "COR", name: "Correctness", score: 100, weight: 0.20 },
        DimensionScore { code: "IDM", name: "Idempotency", score: 100, weight: 0.20 },
        DimensionScore { code: "PRF", name: "Performance", score: 85, weight: 0.15 },
        DimensionScore { code: "SAF", name: "Safety", score: 82, weight: 0.15 },
        DimensionScore { code: "OBS", name: "Observability", score: 60, weight: 0.10 },
        DimensionScore { code: "DOC", name: "Documentation", score: 90, weight: 0.08 },
        DimensionScore { code: "RES", name: "Resilience", score: 50, weight: 0.07 },
        DimensionScore { code: "CMP", name: "Composability", score: 35, weight: 0.05 },
    ];
    let composite = compute_composite(&dims);
    assert_eq!(composite, 84);
}

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
    assert!(result.composite > 0);
    assert_eq!(result.dimensions.len(), 8);
    let saf = result.dimensions.iter().find(|d| d.code == "SAF").unwrap();
    assert_eq!(saf.score, 100);
}
