#![allow(dead_code)]
#![allow(clippy::field_reassign_with_default)]
//! FJ-3208: format_coverage, coverage_to_json, PolicyCoverage methods
//! (split from falsification_gate_coverage).
//!
//! Usage: cargo test --test falsification_gate_coverage_b

use forjar::core::policy_coverage::{compute_coverage, coverage_to_json, format_coverage};
use forjar::core::types::{
    ComplianceMapping, ForjarConfig, PolicyRule, PolicyRuleType, Resource, ResourceType,
};

// ============================================================================
// Helpers
// ============================================================================

fn make_config_with_resources(resources: &[(&str, ResourceType, Option<&str>)]) -> ForjarConfig {
    let mut config = ForjarConfig::default();
    for (name, rtype, owner) in resources {
        let resource = Resource {
            resource_type: rtype.clone(),
            owner: owner.map(|o| o.to_string()),
            ..Default::default()
        };
        config.resources.insert(name.to_string(), resource);
    }
    config
}

fn require_policy(rtype: &str) -> PolicyRule {
    PolicyRule {
        id: Some(format!("P-{rtype}")),
        rule_type: PolicyRuleType::Require,
        message: "test require".into(),
        resource_type: Some(rtype.into()),
        tag: None,
        field: Some("owner".into()),
        condition_field: None,
        condition_value: None,
        max_count: None,
        min_count: None,
        severity: None,
        remediation: None,
        compliance: vec![],
    }
}

// ============================================================================
// FJ-3208: format_coverage
// ============================================================================

#[test]
fn format_coverage_percentage() {
    let mut config = make_config_with_resources(&[
        ("f1", ResourceType::File, None),
        ("p1", ResourceType::Package, None),
    ]);
    config.policies = vec![require_policy("file")];
    let cov = compute_coverage(&config);
    let text = format_coverage(&cov);
    assert!(text.contains("50.0%"));
    assert!(text.contains("1/2"));
}

#[test]
fn format_coverage_uncovered_listed() {
    let mut config = make_config_with_resources(&[
        ("f1", ResourceType::File, None),
        ("pkg-nginx", ResourceType::Package, None),
    ]);
    config.policies = vec![require_policy("file")];
    let cov = compute_coverage(&config);
    let text = format_coverage(&cov);
    assert!(text.contains("pkg-nginx"));
    assert!(text.contains("Uncovered"));
}

#[test]
fn format_coverage_by_type() {
    let mut config = make_config_with_resources(&[("f1", ResourceType::File, None)]);
    config.policies = vec![require_policy("file")];
    let cov = compute_coverage(&config);
    let text = format_coverage(&cov);
    assert!(text.contains("require:"));
}

#[test]
fn format_coverage_frameworks() {
    let mut policy = require_policy("file");
    policy.compliance = vec![ComplianceMapping {
        framework: "NIST".into(),
        control: "AC-3".into(),
    }];
    let mut config = make_config_with_resources(&[("f1", ResourceType::File, None)]);
    config.policies = vec![policy];
    let cov = compute_coverage(&config);
    let text = format_coverage(&cov);
    assert!(text.contains("NIST"));
}

// ============================================================================
// FJ-3208: coverage_to_json
// ============================================================================

#[test]
fn coverage_json_output() {
    let mut config = make_config_with_resources(&[("f1", ResourceType::File, None)]);
    config.policies = vec![require_policy("file")];
    let cov = compute_coverage(&config);
    let json = coverage_to_json(&cov);
    assert_eq!(json["total_resources"], 1);
    assert_eq!(json["covered_resources"], 1);
    assert_eq!(json["fully_covered"], true);
}

#[test]
fn coverage_json_uncovered() {
    let config = make_config_with_resources(&[("orphan", ResourceType::File, None)]);
    let cov = compute_coverage(&config);
    let json = coverage_to_json(&cov);
    assert_eq!(json["fully_covered"], false);
    let uncovered = json["uncovered"].as_array().unwrap();
    assert_eq!(uncovered.len(), 1);
    assert_eq!(uncovered[0], "orphan");
}

// ============================================================================
// FJ-3208: the derived fields
// ============================================================================
//
// These used to build a `PolicyCoverage` by struct literal and then call
// `coverage_percent()` on it. `coverage_percent` is now a FIELD computed once
// by `compute_coverage` — one place for the answer, so the MCP verb and the
// CLI cannot materialise it differently (paiml/forjar#356) — which makes a
// struct literal a test that the literal it just wrote is the literal it just
// wrote. Drive the real calculation instead.

#[test]
fn coverage_percent_empty() {
    let cov = compute_coverage(&make_config_with_resources(&[]));
    assert!((cov.coverage_percent - 100.0).abs() < f64::EPSILON);
    assert!(cov.fully_covered);
}

#[test]
fn coverage_percent_half() {
    let mut config = make_config_with_resources(&[
        ("f1", ResourceType::File, None),
        ("f2", ResourceType::File, None),
        ("p1", ResourceType::Package, None),
        ("p2", ResourceType::Package, None),
    ]);
    config.policies = vec![require_policy("file")];
    let cov = compute_coverage(&config);
    assert_eq!(cov.total_resources, 4);
    assert_eq!(cov.covered_resources, 2);
    assert!((cov.coverage_percent - 50.0).abs() < f64::EPSILON);
    assert!(!cov.fully_covered);
    assert_eq!(cov.uncovered, vec!["p1", "p2"]);
}
