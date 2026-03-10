//! Policy evaluation tests (FJ-220).

use super::*;
use crate::core::types::{PolicyRuleType, PolicySeverity};

#[test]
fn test_fj3200_assert_fail() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
    owner: root
policies:
  - type: assert
    id: SEC-010
    message: "files must be owned by noah"
    resource_type: file
    condition_field: owner
    condition_value: noah
    remediation: "Change owner to noah"
"#;
    let config = parse_config(yaml).unwrap();
    let violations = evaluate_policies(&config);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].rule_type, PolicyRuleType::Assert);
    assert_eq!(violations[0].severity, PolicySeverity::Error);
    assert_eq!(violations[0].policy_id.as_deref(), Some("SEC-010"));
    assert_eq!(
        violations[0].remediation.as_deref(),
        Some("Change owner to noah")
    );
}

#[test]
fn test_fj3200_limit_max_packages() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  big-pkg:
    type: package
    machine: m1
    provider: apt
    packages: [a, b, c, d, e]
policies:
  - type: limit
    id: PERF-001
    message: "package lists under 3 items"
    resource_type: package
    field: packages
    max_count: 3
"#;
    let config = parse_config(yaml).unwrap();
    let violations = evaluate_policies(&config);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].rule_type, PolicyRuleType::Limit);
    assert_eq!(violations[0].severity, PolicySeverity::Warning);
    assert_eq!(violations[0].policy_id.as_deref(), Some("PERF-001"));
}

#[test]
fn test_fj3200_limit_pass() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl, wget]
policies:
  - type: limit
    message: "packages under 5"
    field: packages
    max_count: 5
"#;
    let config = parse_config(yaml).unwrap();
    let violations = evaluate_policies(&config);
    assert!(violations.is_empty());
}

#[test]
fn test_fj3200_limit_min_tags() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
policies:
  - type: limit
    message: "must have at least 1 tag"
    field: tags
    min_count: 1
"#;
    let config = parse_config(yaml).unwrap();
    let violations = evaluate_policies(&config);
    assert_eq!(violations.len(), 1);
}

#[test]
fn test_fj3200_severity_override() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
    owner: root
policies:
  - type: deny
    message: "root owner is info-only"
    condition_field: owner
    condition_value: root
    severity: info
"#;
    let config = parse_config(yaml).unwrap();
    let violations = evaluate_policies(&config);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].severity, PolicySeverity::Info);
    assert!(!violations[0].is_blocking());
}

#[test]
fn test_fj3200_compliance_mapping() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
policies:
  - type: require
    id: SEC-001
    message: "files must have owner"
    resource_type: file
    field: owner
    compliance:
      - framework: cis
        control: "6.1.2"
      - framework: stig
        control: "V-238196"
"#;
    let config = parse_config(yaml).unwrap();
    let violations = evaluate_policies(&config);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].compliance.len(), 2);
    assert_eq!(violations[0].compliance[0].framework, "cis");
    assert_eq!(violations[0].compliance[0].control, "6.1.2");
    assert_eq!(violations[0].compliance[1].framework, "stig");
}

#[test]
fn test_fj3200_full_result() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
    owner: root
policies:
  - type: deny
    message: "no root"
    condition_field: owner
    condition_value: root
  - type: warn
    message: "should have tags"
    condition_field: owner
    condition_value: root
"#;
    let config = parse_config(yaml).unwrap();
    let result = evaluate_policies_full(&config);
    assert_eq!(result.rules_evaluated, 2);
    assert_eq!(result.resources_checked, 1);
    assert!(result.has_blocking_violations());
    assert_eq!(result.error_count(), 1);
    assert_eq!(result.warning_count(), 1);
}

#[test]
fn test_fj3200_json_output() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
    owner: root
policies:
  - type: deny
    id: SEC-001
    message: "no root"
    condition_field: owner
    condition_value: root
    remediation: "change owner"
    compliance:
      - framework: cis
        control: "6.1.2"
"#;
    let config = parse_config(yaml).unwrap();
    let result = evaluate_policies_full(&config);
    let json = policy_check_to_json(&result);
    assert!(json.contains("SEC-001"));
    assert!(json.contains("no root"));
    assert!(json.contains("change owner"));
    assert!(json.contains("cis"));
    assert!(json.contains("6.1.2"));
    assert!(json.contains("\"passed\": false"));
}
