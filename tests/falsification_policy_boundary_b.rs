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
#![allow(dead_code)]

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
// FJ-3209: BoundaryConfig Struct Verification
// ============================================================================

#[test]
fn boundary_config_clone() {
    let config = BoundaryConfig {
        target_rule_id: "R1".into(),
        expected_pass: true,
        resources: HashMap::new(),
        description: "test".into(),
    };
    let cloned = config.clone();
    assert_eq!(cloned.target_rule_id, "R1");
    assert!(cloned.expected_pass);
}

#[test]
fn boundary_config_debug() {
    let config = BoundaryConfig {
        target_rule_id: "R1".into(),
        expected_pass: false,
        resources: HashMap::new(),
        description: "test".into(),
    };
    let debug = format!("{config:?}");
    assert!(debug.contains("BoundaryConfig"));
    assert!(debug.contains("R1"));
}

#[test]
fn boundary_outcome_debug() {
    let outcome = BoundaryOutcome {
        rule_id: "X1".into(),
        passed: true,
        expected: "pass".into(),
        actual: "pass".into(),
        description: "golden".into(),
    };
    let debug = format!("{outcome:?}");
    assert!(debug.contains("BoundaryOutcome"));
}

#[test]
fn boundary_test_result_debug() {
    let result = BoundaryTestResult {
        pack_name: "p".into(),
        rules_tested: 0,
        rules_with_boundary: 0,
        outcomes: vec![],
    };
    let debug = format!("{result:?}");
    assert!(debug.contains("BoundaryTestResult"));
}

// ============================================================================
// FJ-3209: Resource Name Patterns
// ============================================================================

#[test]
fn assert_boundary_resource_name_includes_type() {
    let pack = make_pack("p", vec![assert_rule("R1", "service", "restart", "always")]);
    let configs = generate_boundary_configs(&pack);
    for config in &configs {
        assert!(config.resources.contains_key("boundary-service"));
    }
}

#[test]
fn deny_boundary_resource_name_includes_type() {
    let pack = make_pack("p", vec![deny_rule("D1", "package", "version", "0.0.0")]);
    let configs = generate_boundary_configs(&pack);
    for config in &configs {
        assert!(config.resources.contains_key("boundary-package"));
    }
}

#[test]
fn require_tag_uses_generic_resource_name() {
    let pack = make_pack("p", vec![require_tag_rule("T1", "env:prod")]);
    let configs = generate_boundary_configs(&pack);
    for config in &configs {
        assert!(config.resources.contains_key("boundary-resource"));
    }
}
