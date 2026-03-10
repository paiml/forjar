//! FJ-3209: Policy boundary config generation and mutation testing falsification.
//!
//! Popperian rejection criteria for:
//! - Boundary config generation for all compliance check types (assert/deny/require/require_tag)
//! - Golden configs pass, boundary configs fail at decision boundaries
//! - Script checks produce no auto-generated boundaries
//! - Full boundary test evaluation loop
//! - BoundaryTestResult aggregation (all_passed, failure_count)
//! - format_boundary_results human-readable output
//!
//! Usage: cargo test --test falsification_policy_boundary

use forjar::core::compliance_pack::{ComplianceCheck, CompliancePack, ComplianceRule};
use forjar::core::policy_boundary::{
    format_boundary_results, generate_boundary_configs, test_boundaries, BoundaryConfig,
    BoundaryOutcome, BoundaryTestResult,
};
use std::collections::HashMap;

// ============================================================================
// Helpers
// ============================================================================

fn make_pack(name: &str, rules: Vec<ComplianceRule>) -> CompliancePack {
    CompliancePack {
        name: name.into(),
        version: "1.0".into(),
        framework: "test".into(),
        description: None,
        rules,
    }
}

fn assert_rule(id: &str, resource_type: &str, field: &str, expected: &str) -> ComplianceRule {
    ComplianceRule {
        id: id.into(),
        title: format!("Assert {field}={expected}"),
        description: None,
        severity: "error".into(),
        controls: vec![],
        check: ComplianceCheck::Assert {
            resource_type: resource_type.into(),
            field: field.into(),
            expected: expected.into(),
        },
    }
}

fn deny_rule(id: &str, resource_type: &str, field: &str, pattern: &str) -> ComplianceRule {
    ComplianceRule {
        id: id.into(),
        title: format!("Deny {field}~{pattern}"),
        description: None,
        severity: "error".into(),
        controls: vec![],
        check: ComplianceCheck::Deny {
            resource_type: resource_type.into(),
            field: field.into(),
            pattern: pattern.into(),
        },
    }
}

fn require_rule(id: &str, resource_type: &str, field: &str) -> ComplianceRule {
    ComplianceRule {
        id: id.into(),
        title: format!("Require {field}"),
        description: None,
        severity: "warning".into(),
        controls: vec![],
        check: ComplianceCheck::Require {
            resource_type: resource_type.into(),
            field: field.into(),
        },
    }
}

fn require_tag_rule(id: &str, tag: &str) -> ComplianceRule {
    ComplianceRule {
        id: id.into(),
        title: format!("Require tag '{tag}'"),
        description: None,
        severity: "warning".into(),
        controls: vec![],
        check: ComplianceCheck::RequireTag { tag: tag.into() },
    }
}

fn script_rule(id: &str, script: &str) -> ComplianceRule {
    ComplianceRule {
        id: id.into(),
        title: "Script check".into(),
        description: None,
        severity: "info".into(),
        controls: vec![],
        check: ComplianceCheck::Script {
            script: script.into(),
        },
    }
}

// ============================================================================
// FJ-3209: Assert Boundary Generation
// ============================================================================

#[test]
fn assert_generates_two_configs() {
    let pack = make_pack("p", vec![assert_rule("R1", "file", "owner", "root")]);
    let configs = generate_boundary_configs(&pack);
    assert_eq!(configs.len(), 2);
}

#[test]
fn assert_golden_expects_pass() {
    let pack = make_pack("p", vec![assert_rule("R1", "file", "owner", "root")]);
    let configs = generate_boundary_configs(&pack);
    let golden = configs.iter().find(|c| c.expected_pass).unwrap();
    assert_eq!(golden.target_rule_id, "R1");

    // Golden has correct value
    let resource = golden.resources.values().next().unwrap();
    assert_eq!(resource.get("owner").unwrap(), "root");
}

#[test]
fn assert_boundary_expects_fail() {
    let pack = make_pack("p", vec![assert_rule("R1", "file", "owner", "root")]);
    let configs = generate_boundary_configs(&pack);
    let boundary = configs.iter().find(|c| !c.expected_pass).unwrap();

    // Boundary has wrong value (NOT_root)
    let resource = boundary.resources.values().next().unwrap();
    assert_eq!(resource.get("owner").unwrap(), "NOT_root");
}

#[test]
fn assert_boundary_descriptions() {
    let pack = make_pack("p", vec![assert_rule("R1", "file", "owner", "root")]);
    let configs = generate_boundary_configs(&pack);
    let golden = configs.iter().find(|c| c.expected_pass).unwrap();
    assert!(golden.description.contains("golden"));
    assert!(golden.description.contains("owner=root"));

    let boundary = configs.iter().find(|c| !c.expected_pass).unwrap();
    assert!(boundary.description.contains("boundary"));
    assert!(boundary.description.contains("NOT_root"));
}

#[test]
fn assert_boundary_resource_has_type() {
    let pack = make_pack("p", vec![assert_rule("R1", "service", "restart", "always")]);
    let configs = generate_boundary_configs(&pack);
    for config in &configs {
        let resource = config.resources.values().next().unwrap();
        assert_eq!(resource.get("type").unwrap(), "service");
    }
}

// ============================================================================
// FJ-3209: Deny Boundary Generation
// ============================================================================

#[test]
fn deny_generates_two_configs() {
    let pack = make_pack("p", vec![deny_rule("D1", "file", "mode", "777")]);
    let configs = generate_boundary_configs(&pack);
    assert_eq!(configs.len(), 2);
}

#[test]
fn deny_golden_has_safe_value() {
    let pack = make_pack("p", vec![deny_rule("D1", "file", "mode", "777")]);
    let configs = generate_boundary_configs(&pack);
    let golden = configs.iter().find(|c| c.expected_pass).unwrap();
    let resource = golden.resources.values().next().unwrap();
    assert_eq!(resource.get("mode").unwrap(), "safe_value");
}

#[test]
fn deny_boundary_has_denied_pattern() {
    let pack = make_pack("p", vec![deny_rule("D1", "file", "mode", "777")]);
    let configs = generate_boundary_configs(&pack);
    let boundary = configs.iter().find(|c| !c.expected_pass).unwrap();
    let resource = boundary.resources.values().next().unwrap();
    assert_eq!(resource.get("mode").unwrap(), "777");
}

// ============================================================================
// FJ-3209: Require Boundary Generation
// ============================================================================

#[test]
fn require_generates_two_configs() {
    let pack = make_pack("p", vec![require_rule("Q1", "file", "owner")]);
    let configs = generate_boundary_configs(&pack);
    assert_eq!(configs.len(), 2);
}

#[test]
fn require_golden_has_field() {
    let pack = make_pack("p", vec![require_rule("Q1", "file", "owner")]);
    let configs = generate_boundary_configs(&pack);
    let golden = configs.iter().find(|c| c.expected_pass).unwrap();
    let resource = golden.resources.values().next().unwrap();
    assert!(resource.contains_key("owner"));
}

#[test]
fn require_boundary_missing_field() {
    let pack = make_pack("p", vec![require_rule("Q1", "file", "owner")]);
    let configs = generate_boundary_configs(&pack);
    let boundary = configs.iter().find(|c| !c.expected_pass).unwrap();
    let resource = boundary.resources.values().next().unwrap();
    assert!(!resource.contains_key("owner"));
    // But type is still present
    assert!(resource.contains_key("type"));
}

// ============================================================================
// FJ-3209: RequireTag Boundary Generation
// ============================================================================

#[test]
fn require_tag_generates_two_configs() {
    let pack = make_pack("p", vec![require_tag_rule("T1", "production")]);
    let configs = generate_boundary_configs(&pack);
    assert_eq!(configs.len(), 2);
}

#[test]
fn require_tag_golden_has_tag() {
    let pack = make_pack("p", vec![require_tag_rule("T1", "production")]);
    let configs = generate_boundary_configs(&pack);
    let golden = configs.iter().find(|c| c.expected_pass).unwrap();
    let resource = golden.resources.values().next().unwrap();
    assert_eq!(resource.get("tags").unwrap(), "production");
}

#[test]
fn require_tag_boundary_no_tags() {
    let pack = make_pack("p", vec![require_tag_rule("T1", "production")]);
    let configs = generate_boundary_configs(&pack);
    let boundary = configs.iter().find(|c| !c.expected_pass).unwrap();
    let resource = boundary.resources.values().next().unwrap();
    assert!(!resource.contains_key("tags"));
}

// ============================================================================
// FJ-3209: Script Produces No Boundaries
// ============================================================================

#[test]
fn script_check_produces_no_boundaries() {
    let pack = make_pack("p", vec![script_rule("S1", "true")]);
    let configs = generate_boundary_configs(&pack);
    assert!(configs.is_empty());
}

#[test]
fn mixed_pack_script_excluded() {
    let pack = make_pack(
        "p",
        vec![
            assert_rule("R1", "file", "owner", "root"),
            script_rule("S1", "true"),
            deny_rule("D1", "file", "mode", "777"),
        ],
    );
    let configs = generate_boundary_configs(&pack);
    // 2 from assert + 0 from script + 2 from deny = 4
    assert_eq!(configs.len(), 4);
    assert!(configs.iter().all(|c| c.target_rule_id != "S1"));
}

// ============================================================================
// FJ-3209: Empty Pack
// ============================================================================

#[test]
fn empty_pack_produces_no_configs() {
    let pack = make_pack("empty", vec![]);
    let configs = generate_boundary_configs(&pack);
    assert!(configs.is_empty());
}

#[test]
fn empty_pack_boundary_test_result() {
    let pack = make_pack("empty", vec![]);
    let result = test_boundaries(&pack);
    assert!(result.all_passed());
    assert_eq!(result.failure_count(), 0);
    assert_eq!(result.rules_tested, 0);
    assert_eq!(result.outcomes.len(), 0);
}

// ============================================================================
// FJ-3209: Multiple Rules
// ============================================================================

#[test]
fn multiple_rules_generate_pairs() {
    let pack = make_pack(
        "multi",
        vec![
            assert_rule("R1", "file", "owner", "root"),
            deny_rule("D1", "file", "mode", "777"),
            require_rule("Q1", "service", "restart_policy"),
            require_tag_rule("T1", "managed"),
        ],
    );
    let configs = generate_boundary_configs(&pack);
    // 4 rules x 2 configs each = 8
    assert_eq!(configs.len(), 8);
}

#[test]
fn all_rule_ids_represented() {
    let pack = make_pack(
        "multi",
        vec![
            assert_rule("R1", "file", "owner", "root"),
            deny_rule("D1", "file", "mode", "777"),
            require_rule("Q1", "service", "restart_policy"),
        ],
    );
    let configs = generate_boundary_configs(&pack);
    let ids: Vec<&str> = configs.iter().map(|c| c.target_rule_id.as_str()).collect();
    assert!(ids.contains(&"R1"));
    assert!(ids.contains(&"D1"));
    assert!(ids.contains(&"Q1"));
}

// ============================================================================
// FJ-3209: test_boundaries() Integration
// ============================================================================

#[test]
fn test_boundaries_assert_both_pass() {
    let pack = make_pack("p", vec![assert_rule("R1", "file", "owner", "root")]);
    let result = test_boundaries(&pack);
    // Both golden (correct value → passes) and boundary (wrong value → fails)
    // should match their expectations
    assert!(result.all_passed(), "outcomes: {:?}", result.outcomes);
    assert_eq!(result.failure_count(), 0);
    assert_eq!(result.outcomes.len(), 2);
}

#[test]
fn test_boundaries_deny_both_pass() {
    let pack = make_pack("p", vec![deny_rule("D1", "file", "mode", "777")]);
    let result = test_boundaries(&pack);
    assert!(result.all_passed(), "outcomes: {:?}", result.outcomes);
}

#[test]
fn test_boundaries_require_both_pass() {
    let pack = make_pack("p", vec![require_rule("Q1", "file", "owner")]);
    let result = test_boundaries(&pack);
    assert!(result.all_passed(), "outcomes: {:?}", result.outcomes);
}

#[test]
fn test_boundaries_require_tag_both_pass() {
    let pack = make_pack("p", vec![require_tag_rule("T1", "production")]);
    let result = test_boundaries(&pack);
    assert!(result.all_passed(), "outcomes: {:?}", result.outcomes);
}

#[test]
fn test_boundaries_multi_rule_all_pass() {
    let pack = make_pack(
        "full",
        vec![
            assert_rule("R1", "file", "owner", "root"),
            deny_rule("D1", "file", "mode", "777"),
            require_rule("Q1", "service", "restart_policy"),
            require_tag_rule("T1", "managed"),
        ],
    );
    let result = test_boundaries(&pack);
    assert!(
        result.all_passed(),
        "failures: {:?}",
        result
            .outcomes
            .iter()
            .filter(|o| !o.passed)
            .collect::<Vec<_>>()
    );
    assert_eq!(result.outcomes.len(), 8);
    assert_eq!(result.rules_with_boundary, 4);
}

#[test]
fn test_boundaries_pack_name_preserved() {
    let pack = make_pack(
        "my-pack-name",
        vec![assert_rule("R1", "file", "owner", "root")],
    );
    let result = test_boundaries(&pack);
    assert_eq!(result.pack_name, "my-pack-name");
}

// ============================================================================
// FJ-3209: BoundaryTestResult Methods
// ============================================================================

#[test]
fn boundary_test_result_all_passed_true() {
    let result = BoundaryTestResult {
        pack_name: "p".into(),
        rules_tested: 1,
        rules_with_boundary: 1,
        outcomes: vec![BoundaryOutcome {
            rule_id: "R1".into(),
            passed: true,
            expected: "pass".into(),
            actual: "pass".into(),
            description: "golden".into(),
        }],
    };
    assert!(result.all_passed());
    assert_eq!(result.failure_count(), 0);
}

#[test]
fn boundary_test_result_has_failure() {
    let result = BoundaryTestResult {
        pack_name: "p".into(),
        rules_tested: 1,
        rules_with_boundary: 1,
        outcomes: vec![
            BoundaryOutcome {
                rule_id: "R1".into(),
                passed: true,
                expected: "pass".into(),
                actual: "pass".into(),
                description: "golden".into(),
            },
            BoundaryOutcome {
                rule_id: "R1".into(),
                passed: false,
                expected: "fail".into(),
                actual: "pass".into(),
                description: "boundary".into(),
            },
        ],
    };
    assert!(!result.all_passed());
    assert_eq!(result.failure_count(), 1);
}

// ============================================================================
// FJ-3209: format_boundary_results
// ============================================================================

#[test]
fn format_results_header() {
    let pack = make_pack(
        "cis-ubuntu",
        vec![assert_rule("R1", "file", "owner", "root")],
    );
    let result = test_boundaries(&pack);
    let text = format_boundary_results(&result);
    assert!(text.contains("Boundary Testing: cis-ubuntu"));
}

#[test]
fn format_results_pass_status() {
    let pack = make_pack("p", vec![assert_rule("R1", "file", "owner", "root")]);
    let result = test_boundaries(&pack);
    let text = format_boundary_results(&result);
    assert!(text.contains("[PASS]"));
    assert!(text.contains("R1"));
}

#[test]
fn format_results_summary_line() {
    let pack = make_pack("p", vec![deny_rule("D1", "file", "mode", "777")]);
    let result = test_boundaries(&pack);
    let text = format_boundary_results(&result);
    assert!(text.contains("2/2 boundary tests passed"));
}

#[test]
fn format_results_empty_pack() {
    let pack = make_pack("empty", vec![]);
    let result = test_boundaries(&pack);
    let text = format_boundary_results(&result);
    assert!(text.contains("0/0 boundary tests passed"));
}
