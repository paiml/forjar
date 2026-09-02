//! Unit tests for the policy rule types.
//!
//! Extracted from `policy_rule_types.rs` so that file stays under the 500-line
//! cap after `display_id_at` landed (paiml/forjar#369). Same module, same
//! `use super::*` scope — only the file boundary moved.

use super::*;

#[test]
fn test_effective_severity_defaults() {
    let deny = PolicyRule {
        rule_type: PolicyRuleType::Deny,
        message: "test".into(),
        id: None,
        resource_type: None,
        tag: None,
        field: None,
        condition_field: None,
        condition_value: None,
        max_count: None,
        min_count: None,
        severity: None,
        remediation: None,
        compliance: vec![],
    };
    assert_eq!(deny.effective_severity(), PolicySeverity::Error);

    let warn = PolicyRule {
        rule_type: PolicyRuleType::Warn,
        severity: None,
        ..deny.clone()
    };
    assert_eq!(warn.effective_severity(), PolicySeverity::Warning);

    let assert_r = PolicyRule {
        rule_type: PolicyRuleType::Assert,
        severity: None,
        ..deny.clone()
    };
    assert_eq!(assert_r.effective_severity(), PolicySeverity::Error);

    let limit = PolicyRule {
        rule_type: PolicyRuleType::Limit,
        severity: None,
        ..deny.clone()
    };
    assert_eq!(limit.effective_severity(), PolicySeverity::Warning);

    let require = PolicyRule {
        rule_type: PolicyRuleType::Require,
        severity: None,
        ..deny.clone()
    };
    assert_eq!(require.effective_severity(), PolicySeverity::Error);
}

#[test]
fn test_effective_severity_override() {
    let rule = PolicyRule {
        rule_type: PolicyRuleType::Deny,
        message: "test".into(),
        id: None,
        resource_type: None,
        tag: None,
        field: None,
        condition_field: None,
        condition_value: None,
        max_count: None,
        min_count: None,
        severity: Some(PolicySeverity::Info),
        remediation: None,
        compliance: vec![],
    };
    assert_eq!(rule.effective_severity(), PolicySeverity::Info);
}

#[test]
fn test_display_id_explicit() {
    let rule = PolicyRule {
        rule_type: PolicyRuleType::Deny,
        message: "no root".into(),
        id: Some("SEC-001".into()),
        resource_type: None,
        tag: None,
        field: None,
        condition_field: None,
        condition_value: None,
        max_count: None,
        min_count: None,
        severity: None,
        remediation: None,
        compliance: vec![],
    };
    assert_eq!(rule.display_id(), "SEC-001");
}

#[test]
fn test_display_id_generated() {
    let rule = PolicyRule {
        rule_type: PolicyRuleType::Warn,
        message: "files should have owner".into(),
        id: None,
        resource_type: None,
        tag: None,
        field: None,
        condition_field: None,
        condition_value: None,
        max_count: None,
        min_count: None,
        severity: None,
        remediation: None,
        compliance: vec![],
    };
    assert_eq!(rule.display_id(), "RULE-files-should-have-owner");
}

#[test]
fn test_violation_is_blocking() {
    let v = PolicyViolation {
        rule_message: "test".into(),
        resource_id: "r1".into(),
        rule_type: PolicyRuleType::Deny,
        severity: PolicySeverity::Error,
        policy_id: None,
        remediation: None,
        compliance: vec![],
    };
    assert!(v.is_blocking());

    let v2 = PolicyViolation {
        severity: PolicySeverity::Warning,
        ..v.clone()
    };
    assert!(!v2.is_blocking());
}

#[test]
fn test_policy_check_result_counts() {
    let result = PolicyCheckResult {
        violations: vec![
            PolicyViolation {
                rule_message: "e1".into(),
                resource_id: "r1".into(),
                rule_type: PolicyRuleType::Deny,
                severity: PolicySeverity::Error,
                policy_id: None,
                remediation: None,
                compliance: vec![],
            },
            PolicyViolation {
                rule_message: "w1".into(),
                resource_id: "r2".into(),
                rule_type: PolicyRuleType::Warn,
                severity: PolicySeverity::Warning,
                policy_id: None,
                remediation: None,
                compliance: vec![],
            },
            PolicyViolation {
                rule_message: "i1".into(),
                resource_id: "r3".into(),
                rule_type: PolicyRuleType::Warn,
                severity: PolicySeverity::Info,
                policy_id: None,
                remediation: None,
                compliance: vec![],
            },
        ],
        rules_evaluated: 5,
        resources_checked: 10,
    };
    assert!(result.has_blocking_violations());
    assert_eq!(result.error_count(), 1);
    assert_eq!(result.warning_count(), 1);
    assert_eq!(result.info_count(), 1);
}

#[test]
fn test_policy_check_result_no_blocking() {
    let result = PolicyCheckResult {
        violations: vec![PolicyViolation {
            rule_message: "w1".into(),
            resource_id: "r1".into(),
            rule_type: PolicyRuleType::Warn,
            severity: PolicySeverity::Warning,
            policy_id: None,
            remediation: None,
            compliance: vec![],
        }],
        rules_evaluated: 1,
        resources_checked: 1,
    };
    assert!(!result.has_blocking_violations());
}

#[test]
fn test_compliance_mapping_serde() {
    let m = ComplianceMapping {
        framework: "cis".into(),
        control: "6.1.2".into(),
    };
    let json = serde_json::to_string(&m).unwrap();
    assert!(json.contains("cis"));
    assert!(json.contains("6.1.2"));
}

#[test]
fn test_policy_severity_serde() {
    let s = PolicySeverity::Error;
    let json = serde_json::to_string(&s).unwrap();
    assert_eq!(json, "\"error\"");
    let w: PolicySeverity = serde_json::from_str("\"warning\"").unwrap();
    assert_eq!(w, PolicySeverity::Warning);
}

#[test]
fn test_policy_rule_type_serde_new_variants() {
    let a: PolicyRuleType = serde_json::from_str("\"assert\"").unwrap();
    assert_eq!(a, PolicyRuleType::Assert);
    let l: PolicyRuleType = serde_json::from_str("\"limit\"").unwrap();
    assert_eq!(l, PolicyRuleType::Limit);
}
