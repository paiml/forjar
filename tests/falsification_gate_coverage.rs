//! FJ-3203/3208: Compliance gate and policy coverage falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-3203: Pre-apply compliance gate — pack loading, evaluation, error/warning
//!   severity counting, gate pass/fail logic, format_gate_result
//! - FJ-3208: Policy coverage — compute_coverage, coverage_percent, fully_covered,
//!   per_resource counts, by_type counts, framework tracking, format_coverage,
//!   coverage_to_json
//!
//! Usage: cargo test --test falsification_gate_coverage

use forjar::core::compliance_gate::{
    check_compliance_gate, config_to_resource_map, format_gate_result, ComplianceGateResult,
};
use forjar::core::policy_coverage::{
    compute_coverage, coverage_to_json, format_coverage, PolicyCoverage,
};
use forjar::core::types::{
    ComplianceMapping, ForjarConfig, PolicyRule, PolicyRuleType, Resource, ResourceType,
};
use std::collections::HashMap;

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

fn deny_policy(rtype: &str) -> PolicyRule {
    PolicyRule {
        id: Some(format!("D-{rtype}")),
        rule_type: PolicyRuleType::Deny,
        message: "test deny".into(),
        resource_type: Some(rtype.into()),
        tag: None,
        field: Some("mode".into()),
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
// FJ-3203: config_to_resource_map
// ============================================================================

#[test]
fn gate_config_to_map_basic() {
    let config = make_config_with_resources(&[("nginx", ResourceType::File, Some("root"))]);
    let map = config_to_resource_map(&config);
    assert_eq!(map.len(), 1);
    let nginx = map.get("nginx").unwrap();
    assert_eq!(nginx.get("type").unwrap(), "file");
    assert_eq!(nginx.get("owner").unwrap(), "root");
}

#[test]
fn gate_config_to_map_no_owner() {
    let config = make_config_with_resources(&[("pkg", ResourceType::Package, None)]);
    let map = config_to_resource_map(&config);
    let pkg = map.get("pkg").unwrap();
    assert!(pkg.get("owner").is_none());
}

#[test]
fn gate_config_to_map_with_mode() {
    let mut config = ForjarConfig::default();
    let resource = Resource {
        resource_type: ResourceType::File,
        mode: Some("0644".into()),
        ..Default::default()
    };
    config.resources.insert("f1".into(), resource);
    let map = config_to_resource_map(&config);
    assert_eq!(map.get("f1").unwrap().get("mode").unwrap(), "0644");
}

#[test]
fn gate_config_to_map_with_tags() {
    let mut config = ForjarConfig::default();
    let resource = Resource {
        resource_type: ResourceType::File,
        tags: vec!["web".into(), "config".into()],
        ..Default::default()
    };
    config.resources.insert("f1".into(), resource);
    let map = config_to_resource_map(&config);
    assert_eq!(map.get("f1").unwrap().get("tags").unwrap(), "web,config");
}

#[test]
fn gate_config_to_map_multiple_resources() {
    let config = make_config_with_resources(&[
        ("f1", ResourceType::File, Some("root")),
        ("p1", ResourceType::Package, None),
        ("s1", ResourceType::Service, Some("systemd")),
    ]);
    let map = config_to_resource_map(&config);
    assert_eq!(map.len(), 3);
}

// ============================================================================
// FJ-3203: Gate — Empty Dir
// ============================================================================

#[test]
fn gate_empty_policy_dir_passes() {
    let dir = tempfile::tempdir().unwrap();
    let config = make_config_with_resources(&[("f1", ResourceType::File, None)]);
    let result = check_compliance_gate(dir.path(), &config, false).unwrap();
    assert!(result.passed());
    assert_eq!(result.packs_evaluated, 0);
    assert_eq!(result.error_count, 0);
    assert_eq!(result.warning_count, 0);
}

// ============================================================================
// FJ-3203: Gate — Passing Pack
// ============================================================================

#[test]
fn gate_passing_require_pack() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("test.yaml"),
        r#"
name: test
version: "1.0"
framework: TEST
rules:
  - id: T1
    title: Files need owner
    type: require
    resource_type: file
    field: owner
"#,
    )
    .unwrap();

    let config = make_config_with_resources(&[("f1", ResourceType::File, Some("root"))]);
    let result = check_compliance_gate(dir.path(), &config, false).unwrap();
    assert!(result.passed());
    assert_eq!(result.packs_evaluated, 1);
}

// ============================================================================
// FJ-3203: Gate — Failing Pack
// ============================================================================

#[test]
fn gate_failing_error_severity() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("strict.yaml"),
        r#"
name: strict
version: "1.0"
framework: CIS
rules:
  - id: S1
    title: Require owner
    severity: error
    type: require
    resource_type: file
    field: owner
"#,
    )
    .unwrap();

    let config = make_config_with_resources(&[("f1", ResourceType::File, None)]);
    let result = check_compliance_gate(dir.path(), &config, false).unwrap();
    assert!(!result.passed());
    assert_eq!(result.error_count, 1);
}

#[test]
fn gate_warning_severity_still_passes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("soft.yaml"),
        r#"
name: soft
version: "1.0"
framework: TEST
rules:
  - id: W1
    title: Prefer owner
    severity: warning
    type: require
    resource_type: file
    field: owner
"#,
    )
    .unwrap();

    let config = make_config_with_resources(&[("f1", ResourceType::File, None)]);
    let result = check_compliance_gate(dir.path(), &config, false).unwrap();
    // Warnings don't block the gate
    assert!(result.passed());
    assert_eq!(result.warning_count, 1);
}

// ============================================================================
// FJ-3203: ComplianceGateResult Methods
// ============================================================================

#[test]
fn gate_result_passed_no_errors() {
    let result = ComplianceGateResult {
        packs_evaluated: 2,
        results: vec![],
        error_count: 0,
        warning_count: 5,
    };
    assert!(result.passed());
}

#[test]
fn gate_result_failed_with_errors() {
    let result = ComplianceGateResult {
        packs_evaluated: 1,
        results: vec![],
        error_count: 1,
        warning_count: 0,
    };
    assert!(!result.passed());
}

// ============================================================================
// FJ-3203: format_gate_result
// ============================================================================

#[test]
fn format_gate_pass() {
    let result = ComplianceGateResult {
        packs_evaluated: 3,
        results: vec![],
        error_count: 0,
        warning_count: 2,
    };
    let text = format_gate_result(&result);
    assert!(text.contains("PASS"));
    assert!(text.contains("3 packs"));
    assert!(text.contains("0 errors"));
    assert!(text.contains("2 warnings"));
}

#[test]
fn format_gate_fail() {
    let result = ComplianceGateResult {
        packs_evaluated: 1,
        results: vec![],
        error_count: 3,
        warning_count: 1,
    };
    let text = format_gate_result(&result);
    assert!(text.contains("FAIL"));
    assert!(text.contains("3 errors"));
}

// ============================================================================
// FJ-3208: Policy Coverage — Full Coverage
// ============================================================================

#[test]
fn coverage_full() {
    let mut config = make_config_with_resources(&[("f1", ResourceType::File, None)]);
    config.policies = vec![require_policy("file")];
    let cov = compute_coverage(&config);
    assert_eq!(cov.total_resources, 1);
    assert_eq!(cov.covered_resources, 1);
    assert!(cov.fully_covered());
    assert!((cov.coverage_percent() - 100.0).abs() < f64::EPSILON);
}

// ============================================================================
// FJ-3208: Policy Coverage — Partial
// ============================================================================

#[test]
fn coverage_partial() {
    let mut config = make_config_with_resources(&[
        ("f1", ResourceType::File, None),
        ("p1", ResourceType::Package, None),
    ]);
    config.policies = vec![require_policy("file")];
    let cov = compute_coverage(&config);
    assert_eq!(cov.covered_resources, 1);
    assert!(!cov.fully_covered());
    assert!((cov.coverage_percent() - 50.0).abs() < f64::EPSILON);
    assert!(cov.uncovered.contains(&"p1".to_string()));
}

// ============================================================================
// FJ-3208: Policy Coverage — No Policies
// ============================================================================

#[test]
fn coverage_no_policies() {
    let config = make_config_with_resources(&[
        ("f1", ResourceType::File, None),
        ("p1", ResourceType::Package, None),
    ]);
    let cov = compute_coverage(&config);
    assert_eq!(cov.covered_resources, 0);
    assert_eq!(cov.uncovered.len(), 2);
    assert!((cov.coverage_percent() - 0.0).abs() < f64::EPSILON);
}

// ============================================================================
// FJ-3208: Policy Coverage — No Resources
// ============================================================================

#[test]
fn coverage_no_resources() {
    let mut config = ForjarConfig::default();
    config.policies = vec![require_policy("file")];
    let cov = compute_coverage(&config);
    assert_eq!(cov.total_resources, 0);
    assert!(cov.fully_covered());
    assert!((cov.coverage_percent() - 100.0).abs() < f64::EPSILON);
}

// ============================================================================
// FJ-3208: Policy Coverage — by_type Counts
// ============================================================================

#[test]
fn coverage_by_type() {
    let mut config = make_config_with_resources(&[("f1", ResourceType::File, None)]);
    config.policies = vec![require_policy("file"), deny_policy("file")];
    let cov = compute_coverage(&config);
    assert_eq!(cov.by_type.get("require"), Some(&1));
    assert_eq!(cov.by_type.get("deny"), Some(&1));
}

// ============================================================================
// FJ-3208: Policy Coverage — Framework Tracking
// ============================================================================

#[test]
fn coverage_framework_tracking() {
    let mut policy = require_policy("file");
    policy.compliance = vec![ComplianceMapping {
        framework: "CIS".into(),
        control: "1.1".into(),
    }];
    let mut config = make_config_with_resources(&[("f1", ResourceType::File, None)]);
    config.policies = vec![policy];
    let cov = compute_coverage(&config);
    assert!(cov.frameworks.contains("CIS"));
}

#[test]
fn coverage_multiple_frameworks() {
    let mut p1 = require_policy("file");
    p1.compliance = vec![ComplianceMapping {
        framework: "CIS".into(),
        control: "1.1".into(),
    }];
    let mut p2 = deny_policy("file");
    p2.compliance = vec![ComplianceMapping {
        framework: "SOC2".into(),
        control: "CC6.1".into(),
    }];
    let mut config = make_config_with_resources(&[("f1", ResourceType::File, None)]);
    config.policies = vec![p1, p2];
    let cov = compute_coverage(&config);
    assert!(cov.frameworks.contains("CIS"));
    assert!(cov.frameworks.contains("SOC2"));
}

// ============================================================================
// FJ-3208: Policy Coverage — Per-Resource Multiple Policies
// ============================================================================

#[test]
fn coverage_per_resource_multi() {
    let mut config = make_config_with_resources(&[("f1", ResourceType::File, None)]);
    config.policies = vec![require_policy("file"), require_policy("file")];
    let cov = compute_coverage(&config);
    assert_eq!(cov.per_resource.get("f1"), Some(&2));
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
// FJ-3208: PolicyCoverage Methods
// ============================================================================

#[test]
fn coverage_percent_empty() {
    let cov = PolicyCoverage {
        total_resources: 0,
        covered_resources: 0,
        uncovered: vec![],
        per_resource: HashMap::new(),
        by_type: HashMap::new(),
        frameworks: std::collections::HashSet::new(),
    };
    assert!((cov.coverage_percent() - 100.0).abs() < f64::EPSILON);
    assert!(cov.fully_covered());
}

#[test]
fn coverage_percent_half() {
    let cov = PolicyCoverage {
        total_resources: 4,
        covered_resources: 2,
        uncovered: vec!["a".into(), "b".into()],
        per_resource: HashMap::new(),
        by_type: HashMap::new(),
        frameworks: std::collections::HashSet::new(),
    };
    assert!((cov.coverage_percent() - 50.0).abs() < f64::EPSILON);
    assert!(!cov.fully_covered());
}
