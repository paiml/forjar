use super::policy_boundary::*;
use crate::core::compliance_pack::{ComplianceCheck, CompliancePack, ComplianceRule};

fn make_pack(rules: Vec<ComplianceRule>) -> CompliancePack {
    CompliancePack {
        name: "test-pack".into(),
        version: "1.0".into(),
        framework: "TEST".into(),
        description: None,
        rules,
    }
}

fn assert_rule(id: &str) -> ComplianceRule {
    ComplianceRule {
        id: id.into(),
        title: "Assert test".into(),
        description: None,
        severity: "error".into(),
        controls: vec![],
        check: ComplianceCheck::Assert {
            resource_type: "file".into(),
            field: "owner".into(),
            expected: "root".into(),
        },
    }
}

fn deny_rule(id: &str) -> ComplianceRule {
    ComplianceRule {
        id: id.into(),
        title: "Deny test".into(),
        description: None,
        severity: "error".into(),
        controls: vec![],
        check: ComplianceCheck::Deny {
            resource_type: "file".into(),
            field: "mode".into(),
            pattern: "777".into(),
        },
    }
}

fn require_rule(id: &str) -> ComplianceRule {
    ComplianceRule {
        id: id.into(),
        title: "Require test".into(),
        description: None,
        severity: "warning".into(),
        controls: vec![],
        check: ComplianceCheck::Require {
            resource_type: "file".into(),
            field: "owner".into(),
        },
    }
}

fn require_tag_rule(id: &str) -> ComplianceRule {
    ComplianceRule {
        id: id.into(),
        title: "Tag test".into(),
        description: None,
        severity: "info".into(),
        controls: vec![],
        check: ComplianceCheck::RequireTag {
            tag: "managed".into(),
        },
    }
}

#[test]
fn assert_boundary_generates_two_configs() {
    let pack = make_pack(vec![assert_rule("R1")]);
    let configs = generate_boundary_configs(&pack);
    assert_eq!(configs.len(), 2);
    assert!(configs[0].expected_pass);
    assert!(!configs[1].expected_pass);
}

#[test]
fn deny_boundary_generates_two_configs() {
    let pack = make_pack(vec![deny_rule("R2")]);
    let configs = generate_boundary_configs(&pack);
    assert_eq!(configs.len(), 2);
    assert!(configs[0].expected_pass);
    assert!(!configs[1].expected_pass);
}

#[test]
fn require_boundary_generates_two_configs() {
    let pack = make_pack(vec![require_rule("R3")]);
    let configs = generate_boundary_configs(&pack);
    assert_eq!(configs.len(), 2);
}

#[test]
fn require_tag_boundary_generates_two_configs() {
    let pack = make_pack(vec![require_tag_rule("R4")]);
    let configs = generate_boundary_configs(&pack);
    assert_eq!(configs.len(), 2);
}

#[test]
fn script_rule_no_boundaries() {
    let rule = ComplianceRule {
        id: "S1".into(),
        title: "Script check".into(),
        description: None,
        severity: "error".into(),
        controls: vec![],
        check: ComplianceCheck::Script {
            script: "true".into(),
        },
    };
    let pack = make_pack(vec![rule]);
    let configs = generate_boundary_configs(&pack);
    assert!(configs.is_empty());
}

#[test]
fn assert_boundary_test_passes() {
    let pack = make_pack(vec![assert_rule("R1")]);
    let result = test_boundaries(&pack);
    assert!(result.all_passed());
    assert_eq!(result.failure_count(), 0);
}

#[test]
fn deny_boundary_test_passes() {
    let pack = make_pack(vec![deny_rule("R2")]);
    let result = test_boundaries(&pack);
    assert!(result.all_passed());
}

#[test]
fn require_boundary_test_passes() {
    let pack = make_pack(vec![require_rule("R3")]);
    let result = test_boundaries(&pack);
    assert!(result.all_passed());
}

#[test]
fn require_tag_boundary_test_passes() {
    let pack = make_pack(vec![require_tag_rule("R4")]);
    let result = test_boundaries(&pack);
    assert!(result.all_passed());
}

#[test]
fn mixed_rules_boundary_test() {
    let pack = make_pack(vec![
        assert_rule("R1"),
        deny_rule("R2"),
        require_rule("R3"),
        require_tag_rule("R4"),
    ]);
    let result = test_boundaries(&pack);
    assert_eq!(result.outcomes.len(), 8);
    assert!(result.all_passed());
    assert_eq!(result.rules_with_boundary, 4);
}

#[test]
fn format_results_output() {
    let pack = make_pack(vec![assert_rule("R1")]);
    let result = test_boundaries(&pack);
    let text = format_boundary_results(&result);
    assert!(text.contains("Boundary Testing"));
    assert!(text.contains("[PASS]"));
    assert!(text.contains("R1"));
}

#[test]
fn boundary_config_descriptions() {
    let pack = make_pack(vec![deny_rule("D1")]);
    let configs = generate_boundary_configs(&pack);
    assert!(configs[0].description.contains("golden"));
    assert!(configs[1].description.contains("boundary"));
}

#[test]
fn boundary_result_failure_count() {
    let result = BoundaryTestResult {
        pack_name: "test".into(),
        rules_tested: 2,
        rules_with_boundary: 2,
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

#[test]
fn cis_pack_boundary_test() {
    let pack = crate::core::cis_ubuntu_pack::cis_ubuntu_2204_pack();
    let result = test_boundaries(&pack);
    let non_script_outcomes: Vec<_> = result
        .outcomes
        .iter()
        .filter(|o| !o.description.contains("script"))
        .collect();
    for outcome in &non_script_outcomes {
        assert!(
            outcome.passed,
            "CIS boundary failed: {} - {}",
            outcome.rule_id, outcome.description
        );
    }
}
