//! FJ-051/114: MC/DC analysis and DO-330 tool qualification falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-051: MC/DC pair generation for AND/OR decisions
//!   - Each condition independently affects decision outcome
//!   - Correct pair counts (n conditions → n pairs + 1 total tests)
//!   - Coverage achievability
//!   - Single/multi-condition decisions
//! - FJ-114: DO-330 Tool Qualification data package
//!   - TQL level display formatting
//!   - Qualification package generation
//!   - Requirements traceability (all verified)
//!   - Coverage evidence (all satisfied)
//!   - Serde serialization
//!
//! Usage: cargo test --test falsification_mcdc_do330

use forjar::core::do330::{generate_qualification_package, ToolQualLevel};
use forjar::core::mcdc::{build_decision, generate_mcdc_and, generate_mcdc_or};

// ============================================================================
// FJ-051: build_decision
// ============================================================================

#[test]
fn build_decision_names() {
    let d = build_decision("a && b", &["a", "b"]);
    assert_eq!(d.name, "a && b");
    assert_eq!(d.conditions.len(), 2);
    assert_eq!(d.conditions[0].name, "a");
    assert_eq!(d.conditions[1].name, "b");
}

#[test]
fn build_decision_indices() {
    let d = build_decision("expr", &["x", "y", "z"]);
    assert_eq!(d.conditions[0].index, 0);
    assert_eq!(d.conditions[1].index, 1);
    assert_eq!(d.conditions[2].index, 2);
}

// ============================================================================
// FJ-051: MC/DC AND — Two Conditions
// ============================================================================

#[test]
fn mcdc_and_two_conditions_pair_count() {
    let d = build_decision("a && b", &["a", "b"]);
    let report = generate_mcdc_and(&d);
    assert_eq!(report.pairs.len(), 2);
}

#[test]
fn mcdc_and_two_conditions_min_tests() {
    let d = build_decision("a && b", &["a", "b"]);
    let report = generate_mcdc_and(&d);
    assert_eq!(report.min_tests_needed, 3); // n + 1
}

#[test]
fn mcdc_and_two_conditions_achievable() {
    let d = build_decision("a && b", &["a", "b"]);
    let report = generate_mcdc_and(&d);
    assert!(report.coverage_achievable);
}

#[test]
fn mcdc_and_two_conditions_true_case_all_true() {
    let d = build_decision("a && b", &["a", "b"]);
    let report = generate_mcdc_and(&d);
    for pair in &report.pairs {
        assert!(pair.true_case.iter().all(|&v| v));
    }
}

#[test]
fn mcdc_and_two_conditions_false_case_one_flipped() {
    let d = build_decision("a && b", &["a", "b"]);
    let report = generate_mcdc_and(&d);
    for pair in &report.pairs {
        let false_count = pair.false_case.iter().filter(|&&v| !v).count();
        assert_eq!(false_count, 1); // Exactly one condition flipped
    }
}

// ============================================================================
// FJ-051: MC/DC AND — Three Conditions
// ============================================================================

#[test]
fn mcdc_and_three_conditions() {
    let d = build_decision("a && b && c", &["a", "b", "c"]);
    let report = generate_mcdc_and(&d);
    assert_eq!(report.pairs.len(), 3);
    assert_eq!(report.min_tests_needed, 4);
    assert!(report.coverage_achievable);
}

#[test]
fn mcdc_and_three_conditions_each_independently_affects() {
    let d = build_decision("a && b && c", &["a", "b", "c"]);
    let report = generate_mcdc_and(&d);
    let condition_names: Vec<&str> = report.pairs.iter().map(|p| p.condition.as_str()).collect();
    assert!(condition_names.contains(&"a"));
    assert!(condition_names.contains(&"b"));
    assert!(condition_names.contains(&"c"));
}

// ============================================================================
// FJ-051: MC/DC AND — Single Condition
// ============================================================================

#[test]
fn mcdc_and_single_condition() {
    let d = build_decision("a", &["a"]);
    let report = generate_mcdc_and(&d);
    assert_eq!(report.pairs.len(), 1);
    assert_eq!(report.min_tests_needed, 2);
    assert!(report.coverage_achievable);
}

// ============================================================================
// FJ-051: MC/DC OR — Two Conditions
// ============================================================================

#[test]
fn mcdc_or_two_conditions_pair_count() {
    let d = build_decision("a || b", &["a", "b"]);
    let report = generate_mcdc_or(&d);
    assert_eq!(report.pairs.len(), 2);
}

#[test]
fn mcdc_or_two_conditions_achievable() {
    let d = build_decision("a || b", &["a", "b"]);
    let report = generate_mcdc_or(&d);
    assert!(report.coverage_achievable);
}

#[test]
fn mcdc_or_two_conditions_false_case_all_false() {
    let d = build_decision("a || b", &["a", "b"]);
    let report = generate_mcdc_or(&d);
    for pair in &report.pairs {
        assert!(pair.false_case.iter().all(|&v| !v));
    }
}

#[test]
fn mcdc_or_two_conditions_true_case_one_true() {
    let d = build_decision("a || b", &["a", "b"]);
    let report = generate_mcdc_or(&d);
    for pair in &report.pairs {
        let true_count = pair.true_case.iter().filter(|&&v| v).count();
        assert_eq!(true_count, 1);
    }
}

// ============================================================================
// FJ-051: MC/DC OR — Three Conditions
// ============================================================================

#[test]
fn mcdc_or_three_conditions() {
    let d = build_decision("a || b || c", &["a", "b", "c"]);
    let report = generate_mcdc_or(&d);
    assert_eq!(report.pairs.len(), 3);
    assert_eq!(report.min_tests_needed, 4);
}

// ============================================================================
// FJ-051: MC/DC Report — Serde
// ============================================================================

#[test]
fn mcdc_report_serde() {
    let d = build_decision("x && y", &["x", "y"]);
    let report = generate_mcdc_and(&d);
    let json = serde_json::to_string(&report).unwrap();
    assert!(json.contains("\"coverage_achievable\":true"));
    assert!(json.contains("\"decision\":\"x && y\""));
}

#[test]
fn mcdc_report_decision_name_preserved() {
    let d = build_decision("complex_expr", &["a", "b", "c"]);
    let report = generate_mcdc_and(&d);
    assert_eq!(report.decision, "complex_expr");
    assert_eq!(report.num_conditions, 3);
}

// ============================================================================
// FJ-114: ToolQualLevel Display
// ============================================================================

#[test]
fn tql5_display() {
    assert_eq!(format!("{}", ToolQualLevel::Tql5), "TQL-5");
}

#[test]
fn tql4_display() {
    assert_eq!(format!("{}", ToolQualLevel::Tql4), "TQL-4");
}

#[test]
fn tql3_display() {
    assert_eq!(format!("{}", ToolQualLevel::Tql3), "TQL-3");
}

#[test]
fn tql2_display() {
    assert_eq!(format!("{}", ToolQualLevel::Tql2), "TQL-2");
}

#[test]
fn tql1_display() {
    assert_eq!(format!("{}", ToolQualLevel::Tql1), "TQL-1");
}

// ============================================================================
// FJ-114: ToolQualLevel Equality
// ============================================================================

#[test]
fn tql_equality() {
    assert_eq!(ToolQualLevel::Tql5, ToolQualLevel::Tql5);
    assert_ne!(ToolQualLevel::Tql5, ToolQualLevel::Tql1);
}

// ============================================================================
// FJ-114: Qualification Package Generation
// ============================================================================

#[test]
fn qualification_package_tql5() {
    let pkg = generate_qualification_package("1.1.1", ToolQualLevel::Tql5);
    assert_eq!(pkg.tool_name, "forjar");
    assert_eq!(pkg.tool_version, "1.1.1");
    assert_eq!(pkg.qualification_level, ToolQualLevel::Tql5);
}

#[test]
fn qualification_package_has_requirements() {
    let pkg = generate_qualification_package("1.0", ToolQualLevel::Tql5);
    assert!(pkg.total_requirements > 0);
    assert!(pkg.verified_requirements > 0);
}

#[test]
fn qualification_package_requirements_all_verified() {
    let pkg = generate_qualification_package("1.0", ToolQualLevel::Tql5);
    assert_eq!(pkg.total_requirements, pkg.verified_requirements);
    for req in &pkg.requirements {
        assert!(req.verified, "req {} not verified", req.id);
    }
}

#[test]
fn qualification_package_has_coverage_evidence() {
    let pkg = generate_qualification_package("1.0", ToolQualLevel::Tql5);
    assert!(!pkg.coverage_evidence.is_empty());
    for cov in &pkg.coverage_evidence {
        assert!(cov.satisfied, "coverage '{}' not satisfied", cov.metric);
    }
}

#[test]
fn qualification_package_complete() {
    let pkg = generate_qualification_package("1.0", ToolQualLevel::Tql5);
    assert!(pkg.qualification_complete);
}

#[test]
fn qualification_package_tql4() {
    let pkg = generate_qualification_package("2.0", ToolQualLevel::Tql4);
    assert_eq!(pkg.qualification_level, ToolQualLevel::Tql4);
    assert_eq!(pkg.tool_version, "2.0");
}

// ============================================================================
// FJ-114: Qualification Package — Serde
// ============================================================================

#[test]
fn qualification_package_serde() {
    let pkg = generate_qualification_package("1.0", ToolQualLevel::Tql5);
    let json = serde_json::to_string(&pkg).unwrap();
    assert!(json.contains("\"tool_name\":\"forjar\""));
    assert!(json.contains("\"qualification_level\":\"Tql5\""));
    assert!(json.contains("\"qualification_complete\":true"));
}

// ============================================================================
// FJ-114: Requirement Structure
// ============================================================================

#[test]
fn requirement_has_test_cases() {
    let pkg = generate_qualification_package("1.0", ToolQualLevel::Tql5);
    for req in &pkg.requirements {
        assert!(
            !req.test_cases.is_empty(),
            "req {} has no test cases",
            req.id
        );
    }
}

#[test]
fn requirement_ids_unique() {
    let pkg = generate_qualification_package("1.0", ToolQualLevel::Tql5);
    let ids: Vec<&str> = pkg.requirements.iter().map(|r| r.id.as_str()).collect();
    let mut sorted = ids.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(ids.len(), sorted.len(), "duplicate requirement IDs");
}

#[test]
fn requirement_has_source_reference() {
    let pkg = generate_qualification_package("1.0", ToolQualLevel::Tql5);
    for req in &pkg.requirements {
        assert!(
            req.source.contains("DO-330"),
            "req {} source doesn't reference DO-330",
            req.id
        );
    }
}

// ============================================================================
// FJ-114: Coverage Evidence
// ============================================================================

#[test]
fn coverage_evidence_meets_threshold() {
    let pkg = generate_qualification_package("1.0", ToolQualLevel::Tql5);
    for cov in &pkg.coverage_evidence {
        assert!(
            cov.achieved >= cov.required,
            "{}: achieved {} < required {}",
            cov.metric,
            cov.achieved,
            cov.required
        );
    }
}

#[test]
fn coverage_evidence_includes_line_coverage() {
    let pkg = generate_qualification_package("1.0", ToolQualLevel::Tql5);
    assert!(
        pkg.coverage_evidence
            .iter()
            .any(|c| c.metric.contains("Line")),
        "no line coverage evidence"
    );
}

#[test]
fn coverage_evidence_includes_mcdc() {
    let pkg = generate_qualification_package("1.0", ToolQualLevel::Tql5);
    assert!(
        pkg.coverage_evidence
            .iter()
            .any(|c| c.metric.contains("MC/DC")),
        "no MC/DC coverage evidence"
    );
}
