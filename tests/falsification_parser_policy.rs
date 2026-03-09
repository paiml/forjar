//! FJ-220/3200/3207: Policy-as-Code evaluation, JSON/SARIF output.
//!
//! Popperian rejection criteria for:
//! - FJ-220: Policy evaluation (require, deny, warn rules)
//! - FJ-3200: Extended policies (assert, limit, severity, compliance)
//! - FJ-3207: SARIF 2.1.0 output for CI integration
//!
//! Usage: cargo test --test falsification_parser_policy

use forjar::core::parser::{
    evaluate_policies, evaluate_policies_full, parse_config, policy_check_to_json,
    policy_check_to_sarif,
};
use forjar::core::types::*;

fn config_with_policy(resource_yaml: &str, policies_yaml: &str) -> ForjarConfig {
    let yaml = format!(
        r#"
version: "1.0"
name: test
resources:
{resource_yaml}
policies:
{policies_yaml}
"#
    );
    parse_config(&yaml).unwrap()
}

// ============================================================================
// FJ-220: evaluate_policies — require rule
// ============================================================================

#[test]
fn policy_require_field_missing() {
    let cfg = config_with_policy(
        r#"  conf:
    type: file
    path: /etc/test.conf"#,
        r#"  - type: require
    message: "files must have owner"
    resource_type: file
    field: owner"#,
    );
    let violations = evaluate_policies(&cfg);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].resource_id, "conf");
    assert!(violations[0].rule_message.contains("owner"));
}

#[test]
fn policy_require_field_present() {
    let cfg = config_with_policy(
        r#"  conf:
    type: file
    path: /etc/test.conf
    owner: root"#,
        r#"  - type: require
    message: "files must have owner"
    resource_type: file
    field: owner"#,
    );
    let violations = evaluate_policies(&cfg);
    assert!(violations.is_empty());
}

// ============================================================================
// FJ-220: evaluate_policies — deny rule
// ============================================================================

#[test]
fn policy_deny_condition_matches() {
    let cfg = config_with_policy(
        r#"  svc:
    type: service
    name: telnetd"#,
        r#"  - type: deny
    message: "telnet is forbidden"
    resource_type: service
    condition_field: name
    condition_value: telnetd"#,
    );
    let violations = evaluate_policies(&cfg);
    assert_eq!(violations.len(), 1);
    assert!(violations[0].rule_message.contains("telnet"));
}

#[test]
fn policy_deny_condition_no_match() {
    let cfg = config_with_policy(
        r#"  svc:
    type: service
    name: sshd"#,
        r#"  - type: deny
    message: "telnet is forbidden"
    resource_type: service
    condition_field: name
    condition_value: telnetd"#,
    );
    let violations = evaluate_policies(&cfg);
    assert!(violations.is_empty());
}

// ============================================================================
// FJ-220: evaluate_policies — warn rule
// ============================================================================

#[test]
fn policy_warn_matches() {
    let cfg = config_with_policy(
        r#"  svc:
    type: service
    name: nginx"#,
        r#"  - type: warn
    message: "nginx should be reviewed"
    condition_field: name
    condition_value: nginx"#,
    );
    let violations = evaluate_policies(&cfg);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].rule_type, PolicyRuleType::Warn);
}

// ============================================================================
// FJ-220: scope filtering — resource_type and tag
// ============================================================================

#[test]
fn policy_scope_filters_by_resource_type() {
    let cfg = config_with_policy(
        r#"  pkg:
    type: package
    packages: [curl]
  conf:
    type: file
    path: /etc/test.conf"#,
        r#"  - type: require
    message: "files must have mode"
    resource_type: file
    field: mode"#,
    );
    let violations = evaluate_policies(&cfg);
    // Only file resource should be checked, not package
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].resource_id, "conf");
}

#[test]
fn policy_scope_filters_by_tag() {
    let cfg = config_with_policy(
        r#"  web:
    type: file
    path: /etc/nginx.conf
    tags: [production]
  dev:
    type: file
    path: /etc/dev.conf
    tags: [development]"#,
        r#"  - type: require
    message: "production files need owner"
    tag: production
    field: owner"#,
    );
    let violations = evaluate_policies(&cfg);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].resource_id, "web");
}

// ============================================================================
// FJ-3200: assert rule
// ============================================================================

#[test]
fn policy_assert_field_matches() {
    let cfg = config_with_policy(
        r#"  conf:
    type: file
    path: /etc/test.conf
    owner: root"#,
        r#"  - type: assert
    message: "owner must be root"
    resource_type: file
    condition_field: owner
    condition_value: root"#,
    );
    let violations = evaluate_policies(&cfg);
    assert!(violations.is_empty());
}

#[test]
fn policy_assert_field_mismatch() {
    let cfg = config_with_policy(
        r#"  conf:
    type: file
    path: /etc/test.conf
    owner: nobody"#,
        r#"  - type: assert
    message: "owner must be root"
    resource_type: file
    condition_field: owner
    condition_value: root"#,
    );
    let violations = evaluate_policies(&cfg);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].rule_type, PolicyRuleType::Assert);
}

// ============================================================================
// FJ-3200: limit rule
// ============================================================================

#[test]
fn policy_limit_under_max() {
    let cfg = config_with_policy(
        r#"  pkg:
    type: package
    packages: [curl, wget]"#,
        r#"  - type: limit
    message: "max 3 packages"
    resource_type: package
    field: packages
    max_count: 3"#,
    );
    let violations = evaluate_policies(&cfg);
    assert!(violations.is_empty());
}

#[test]
fn policy_limit_over_max() {
    let cfg = config_with_policy(
        r#"  pkg:
    type: package
    packages: [curl, wget, jq, vim]"#,
        r#"  - type: limit
    message: "max 3 packages"
    resource_type: package
    field: packages
    max_count: 3"#,
    );
    let violations = evaluate_policies(&cfg);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].rule_type, PolicyRuleType::Limit);
}

#[test]
fn policy_limit_under_min() {
    let cfg = config_with_policy(
        r#"  conf:
    type: file
    path: /etc/test.conf
    tags: [a]"#,
        r#"  - type: limit
    message: "must have >= 2 tags"
    field: tags
    min_count: 2"#,
    );
    let violations = evaluate_policies(&cfg);
    assert_eq!(violations.len(), 1);
}

// ============================================================================
// FJ-3200: evaluate_policies_full — aggregate result
// ============================================================================

#[test]
fn policy_full_result_counts() {
    let cfg = config_with_policy(
        r#"  conf:
    type: file
    path: /etc/test.conf"#,
        r#"  - type: require
    message: "need owner"
    field: owner
  - type: require
    message: "need mode"
    field: mode"#,
    );
    let result = evaluate_policies_full(&cfg);
    assert_eq!(result.rules_evaluated, 2);
    assert_eq!(result.resources_checked, 1);
    assert_eq!(result.violations.len(), 2);
}

#[test]
fn policy_full_no_violations() {
    let cfg = config_with_policy(
        r#"  pkg:
    type: package
    packages: [curl]"#,
        r#"  - type: require
    message: "need packages"
    resource_type: package
    field: packages"#,
    );
    let result = evaluate_policies_full(&cfg);
    assert!(result.violations.is_empty());
    assert!(!result.has_blocking_violations());
}

// ============================================================================
// FJ-3200: policy_check_to_json
// ============================================================================

#[test]
fn policy_json_output() {
    let cfg = config_with_policy(
        r#"  conf:
    type: file
    path: /etc/test.conf"#,
        r#"  - type: require
    message: "need owner"
    field: owner"#,
    );
    let result = evaluate_policies_full(&cfg);
    let json = policy_check_to_json(&result);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["passed"], false);
    assert_eq!(parsed["rules_evaluated"], 1);
    assert_eq!(parsed["violations"].as_array().unwrap().len(), 1);
}

#[test]
fn policy_json_passed() {
    let result = PolicyCheckResult {
        violations: vec![],
        rules_evaluated: 1,
        resources_checked: 2,
    };
    let json = policy_check_to_json(&result);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["passed"], true);
    assert_eq!(parsed["error_count"], 0);
}

// ============================================================================
// FJ-3207: policy_check_to_sarif
// ============================================================================

#[test]
fn policy_sarif_valid_structure() {
    let cfg = config_with_policy(
        r#"  conf:
    type: file
    path: /etc/test.conf"#,
        r#"  - type: require
    message: "need owner"
    field: owner"#,
    );
    let result = evaluate_policies_full(&cfg);
    let sarif = policy_check_to_sarif(&result);
    let parsed: serde_json::Value = serde_json::from_str(&sarif).unwrap();
    assert_eq!(parsed["version"], "2.1.0");
    let runs = parsed["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["tool"]["driver"]["name"], "forjar");
    let results = runs[0]["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["level"], "error");
}

#[test]
fn policy_sarif_empty() {
    let result = PolicyCheckResult {
        violations: vec![],
        rules_evaluated: 0,
        resources_checked: 0,
    };
    let sarif = policy_check_to_sarif(&result);
    let parsed: serde_json::Value = serde_json::from_str(&sarif).unwrap();
    assert_eq!(parsed["version"], "2.1.0");
    let results = parsed["runs"][0]["results"].as_array().unwrap();
    assert!(results.is_empty());
}
