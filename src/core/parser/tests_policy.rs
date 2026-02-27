//! Policy evaluation tests (FJ-220).

use super::*;
use crate::core::types::PolicyRuleType;

#[test]
fn test_fj220_policy_require_field_pass() {
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
    owner: noah
    mode: "0644"
policies:
  - type: require
    message: "files must have owner"
    resource_type: file
    field: owner
"#;
    let config = parse_config(yaml).unwrap();
    let violations = evaluate_policies(&config);
    assert!(violations.is_empty());
}

#[test]
fn test_fj220_policy_require_field_fail() {
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
    message: "files must have owner"
    resource_type: file
    field: owner
"#;
    let config = parse_config(yaml).unwrap();
    let violations = evaluate_policies(&config);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].resource_id, "cfg");
    assert_eq!(violations[0].severity, PolicyRuleType::Require);
}

#[test]
fn test_fj220_policy_deny_condition() {
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
    message: "files must not be owned by root"
    resource_type: file
    condition_field: owner
    condition_value: root
"#;
    let config = parse_config(yaml).unwrap();
    let violations = evaluate_policies(&config);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].severity, PolicyRuleType::Deny);
}

#[test]
fn test_fj220_policy_warn_only() {
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
  - type: warn
    message: "files should not be owned by root"
    resource_type: file
    condition_field: owner
    condition_value: root
"#;
    let config = parse_config(yaml).unwrap();
    let violations = evaluate_policies(&config);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].severity, PolicyRuleType::Warn);
}

#[test]
fn test_fj220_policy_type_filter() {
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
    packages: [curl]
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
policies:
  - type: require
    message: "files must have owner"
    resource_type: file
    field: owner
"#;
    let config = parse_config(yaml).unwrap();
    let violations = evaluate_policies(&config);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].resource_id, "cfg");
}

#[test]
fn test_fj220_policy_tag_filter() {
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
    tags: [critical]
  log:
    type: file
    machine: m1
    path: /var/log/app.log
policies:
  - type: require
    message: "critical files must have owner"
    tag: critical
    field: owner
"#;
    let config = parse_config(yaml).unwrap();
    let violations = evaluate_policies(&config);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].resource_id, "cfg");
}

#[test]
fn test_fj220_policy_multiple_rules() {
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
  - type: require
    message: "files must have mode"
    resource_type: file
    field: mode
  - type: deny
    message: "no root owner"
    resource_type: file
    condition_field: owner
    condition_value: root
"#;
    let config = parse_config(yaml).unwrap();
    let violations = evaluate_policies(&config);
    assert_eq!(violations.len(), 2);
}

#[test]
fn test_fj220_no_policies() {
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
"#;
    let config = parse_config(yaml).unwrap();
    let violations = evaluate_policies(&config);
    assert!(violations.is_empty());
}

#[test]
fn test_fj220_require_tags() {
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
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
    tags: [infra]
policies:
  - type: require
    message: "all resources must have tags"
    field: tags
"#;
    let config = parse_config(yaml).unwrap();
    let violations = evaluate_policies(&config);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].resource_id, "cfg");
}
