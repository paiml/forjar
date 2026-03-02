//! Tests for the scoring module — part 2: runtime dimensions, grades, formatting.

use super::scoring::*;
use super::tests_scoring::{full_runtime, minimal_config, minimal_resource, static_input};
use super::types::ResourceType;

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
        DimensionScore {
            code: "COR",
            name: "Correctness",
            score: 100,
            weight: 0.20,
        },
        DimensionScore {
            code: "IDM",
            name: "Idempotency",
            score: 100,
            weight: 0.20,
        },
        DimensionScore {
            code: "PRF",
            name: "Performance",
            score: 85,
            weight: 0.15,
        },
        DimensionScore {
            code: "SAF",
            name: "Safety",
            score: 82,
            weight: 0.15,
        },
        DimensionScore {
            code: "OBS",
            name: "Observability",
            score: 60,
            weight: 0.10,
        },
        DimensionScore {
            code: "DOC",
            name: "Documentation",
            score: 90,
            weight: 0.08,
        },
        DimensionScore {
            code: "RES",
            name: "Resilience",
            score: 50,
            weight: 0.07,
        },
        DimensionScore {
            code: "CMP",
            name: "Composability",
            score: 35,
            weight: 0.05,
        },
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

// ============================================================================
// Performance dimension edge cases
// ============================================================================

#[test]
fn performance_budget_ranges() {
    let mk = |first_ms, budget_ms, second_ms| {
        let mut rt = full_runtime();
        rt.first_apply_ms = first_ms;
        rt.second_apply_ms = second_ms;
        ScoringInput {
            status: "qualified".to_string(),
            idempotency: "strong".to_string(),
            budget_ms,
            runtime: Some(rt),
        }
    };
    let prf = |input: &ScoringInput| {
        compute(&minimal_config(), input)
            .dimensions
            .into_iter()
            .find(|d| d.code == "PRF")
            .unwrap()
            .score
    };
    // 51-75% budget → 40pts; eff ~2.5% → 20pts; idem <=2s → 30pts = 90
    assert_eq!(prf(&mk(4000, 6000, 100)), 90);
    // 101-150% budget → 15pts; eff ~2.8% → 20pts; idem <=2s → 30pts = 65
    assert_eq!(prf(&mk(7000, 5000, 200)), 65);
    // >150% budget → 0pts; eff 2% → 20pts; idem <=2s → 30pts = 50
    assert_eq!(prf(&mk(10000, 5000, 200)), 50);
    // idem 3000ms → 25pts; budget 66% → 40pts; eff 15% → 10pts = 75
    assert_eq!(prf(&mk(20000, 30000, 3000)), 75);
    // idem >10s → 0pts; budget 66% → 40pts; eff 75% → 0pts = 40
    assert_eq!(prf(&mk(20000, 30000, 15000)), 40);
    // eff 8% → 15pts; budget 50% → 50pts; idem <=2s → 30pts = 95
    assert_eq!(prf(&mk(10000, 20000, 800)), 95);
}

// ============================================================================
// Documentation edge cases
// ============================================================================

#[test]
fn documentation_generic_name_no_bonus() {
    let mut config = minimal_config();
    config.name = "unnamed".to_string();
    config.description = None;
    let input = static_input();
    let result = compute(&config, &input);
    let doc = result.dimensions.iter().find(|d| d.code == "DOC").unwrap();
    assert_eq!(doc.score, 0);
}

#[test]
fn documentation_empty_name_no_bonus() {
    let mut config = minimal_config();
    config.name = "".to_string();
    config.description = Some("A description".to_string());
    let input = static_input();
    let result = compute(&config, &input);
    let doc = result.dimensions.iter().find(|d| d.code == "DOC").unwrap();
    // 15 (description present) + 0 (generic name) + 25 (non-empty description) = 40
    assert_eq!(doc.score, 40);
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
    assert_eq!(cmp.score, 15);
}

// ============================================================================
// Resilience edge cases
// ============================================================================

#[test]
fn resilience_dag_ratio_30_to_49_pct() {
    let mut config = minimal_config();
    // 3 resources, 1 with deps = 33% ratio → 20 pts
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
    // 0 (no continue_independent) + 0 (no ssh_retries) + 20 (dag 33%) + 0 (no hooks) = 20
    assert_eq!(res.score, 20);
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
    // 0 (no continue_independent) + 0 (ssh_retries) + 0 (dag <30%) + 0 (policy hooks) + 10 (resource hooks) = 10
    assert_eq!(dim.score, 10);
}

// ============================================================================
// Idempotency edge cases
// ============================================================================

#[test]
fn idempotency_weak_class_runtime() {
    let config = minimal_config();
    let input = ScoringInput {
        status: "qualified".to_string(),
        idempotency: "weak".to_string(),
        budget_ms: 0,
        runtime: Some(full_runtime()),
    };
    let result = compute(&config, &input);
    let idm = result.dimensions.iter().find(|d| d.code == "IDM").unwrap();
    // 30 + 30 + 20 + 10 (weak) = 90
    assert_eq!(idm.score, 90);
}

#[test]
fn idempotency_eventual_class_static() {
    let config = minimal_config();
    let input = ScoringInput {
        status: "qualified".to_string(),
        idempotency: "eventual".to_string(),
        budget_ms: 0,
        runtime: None,
    };
    let result = compute(&config, &input);
    let idm = result.dimensions.iter().find(|d| d.code == "IDM").unwrap();
    assert_eq!(idm.score, 0);
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
    };
    let result = compute(&config, &input);
    let idm = result.dimensions.iter().find(|d| d.code == "IDM").unwrap();
    // 30 + 0 (not zero) + 20 + 20 - 30 (3*10) = 40
    assert_eq!(idm.score, 40);
}
