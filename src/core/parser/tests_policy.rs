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
    assert_eq!(violations[0].rule_type, PolicyRuleType::Require);
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
    assert_eq!(violations[0].rule_type, PolicyRuleType::Deny);
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
    assert_eq!(violations[0].rule_type, PolicyRuleType::Warn);
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
fn test_fj220_has_field_all_fields() {
    use crate::core::parser::policy::resource_has_field;
    let mut r = Resource::default();

    // Initially nothing set
    assert!(!resource_has_field(&r, "owner"));
    assert!(!resource_has_field(&r, "group"));
    assert!(!resource_has_field(&r, "mode"));
    assert!(!resource_has_field(&r, "tags"));
    assert!(!resource_has_field(&r, "path"));
    assert!(!resource_has_field(&r, "content"));
    assert!(!resource_has_field(&r, "source"));
    assert!(!resource_has_field(&r, "name"));
    assert!(!resource_has_field(&r, "provider"));
    assert!(!resource_has_field(&r, "packages"));
    assert!(!resource_has_field(&r, "depends_on"));
    assert!(!resource_has_field(&r, "shell"));
    assert!(!resource_has_field(&r, "home"));
    assert!(!resource_has_field(&r, "schedule"));
    assert!(!resource_has_field(&r, "command"));
    assert!(!resource_has_field(&r, "image"));
    assert!(!resource_has_field(&r, "state"));
    assert!(!resource_has_field(&r, "when"));
    assert!(!resource_has_field(&r, "nonexistent"));

    // Set each field
    r.owner = Some("root".into());
    assert!(resource_has_field(&r, "owner"));
    r.group = Some("root".into());
    assert!(resource_has_field(&r, "group"));
    r.mode = Some("0644".into());
    assert!(resource_has_field(&r, "mode"));
    r.tags = vec!["web".into()];
    assert!(resource_has_field(&r, "tags"));
    r.path = Some("/etc/app".into());
    assert!(resource_has_field(&r, "path"));
    r.content = Some("data".into());
    assert!(resource_has_field(&r, "content"));
    r.source = Some("/src".into());
    assert!(resource_has_field(&r, "source"));
    r.name = Some("nginx".into());
    assert!(resource_has_field(&r, "name"));
    r.provider = Some("apt".into());
    assert!(resource_has_field(&r, "provider"));
    r.packages = vec!["curl".into()];
    assert!(resource_has_field(&r, "packages"));
    r.depends_on = vec!["dep1".into()];
    assert!(resource_has_field(&r, "depends_on"));
    r.shell = Some("/bin/bash".into());
    assert!(resource_has_field(&r, "shell"));
    r.home = Some("/home/user".into());
    assert!(resource_has_field(&r, "home"));
    r.schedule = Some("0 * * * *".into());
    assert!(resource_has_field(&r, "schedule"));
    r.command = Some("echo hi".into());
    assert!(resource_has_field(&r, "command"));
    r.image = Some("ubuntu:22.04".into());
    assert!(resource_has_field(&r, "image"));
    r.state = Some("running".into());
    assert!(resource_has_field(&r, "state"));
    r.when = Some("always".into());
    assert!(resource_has_field(&r, "when"));
}

#[test]
fn test_fj220_field_value_all_fields() {
    use crate::core::parser::policy::resource_field_value;
    let mut r = Resource {
        resource_type: ResourceType::File,
        ..Default::default()
    };

    // None values
    assert!(resource_field_value(&r, "owner").is_none());
    assert!(resource_field_value(&r, "nonexistent").is_none());

    // type is always available
    assert_eq!(resource_field_value(&r, "type").unwrap(), "file");

    // Set fields and check values
    r.owner = Some("root".into());
    assert_eq!(resource_field_value(&r, "owner").unwrap(), "root");
    r.group = Some("www-data".into());
    assert_eq!(resource_field_value(&r, "group").unwrap(), "www-data");
    r.mode = Some("0755".into());
    assert_eq!(resource_field_value(&r, "mode").unwrap(), "0755");
    r.path = Some("/etc/app.conf".into());
    assert_eq!(resource_field_value(&r, "path").unwrap(), "/etc/app.conf");
    r.content = Some("cfg data".into());
    assert_eq!(resource_field_value(&r, "content").unwrap(), "cfg data");
    r.source = Some("/src/file".into());
    assert_eq!(resource_field_value(&r, "source").unwrap(), "/src/file");
    r.name = Some("nginx".into());
    assert_eq!(resource_field_value(&r, "name").unwrap(), "nginx");
    r.provider = Some("apt".into());
    assert_eq!(resource_field_value(&r, "provider").unwrap(), "apt");
    r.state = Some("stopped".into());
    assert_eq!(resource_field_value(&r, "state").unwrap(), "stopped");
    r.shell = Some("/bin/zsh".into());
    assert_eq!(resource_field_value(&r, "shell").unwrap(), "/bin/zsh");
    r.home = Some("/home/noah".into());
    assert_eq!(resource_field_value(&r, "home").unwrap(), "/home/noah");
    r.schedule = Some("0 5 * * *".into());
    assert_eq!(resource_field_value(&r, "schedule").unwrap(), "0 5 * * *");
    r.command = Some("echo hello".into());
    assert_eq!(resource_field_value(&r, "command").unwrap(), "echo hello");
    r.image = Some("alpine:3.18".into());
    assert_eq!(resource_field_value(&r, "image").unwrap(), "alpine:3.18");
}

#[test]
fn test_fj220_deny_condition_no_match() {
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
policies:
  - type: deny
    message: "no root owner"
    condition_field: owner
    condition_value: root
"#;
    let config = parse_config(yaml).unwrap();
    let violations = evaluate_policies(&config);
    assert!(violations.is_empty());
}

#[test]
fn test_fj220_require_no_field_specified() {
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
    message: "no field specified"
"#;
    let config = parse_config(yaml).unwrap();
    let violations = evaluate_policies(&config);
    assert!(violations.is_empty()); // No field => not violated
}

#[test]
fn test_fj220_deny_no_condition_specified() {
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
  - type: deny
    message: "no condition"
"#;
    let config = parse_config(yaml).unwrap();
    let violations = evaluate_policies(&config);
    assert!(violations.is_empty()); // No condition => not violated
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

// ── FJ-3200 tests ──────────────────────────────────────────────────

#[test]
fn test_fj3200_assert_pass() {
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
policies:
  - type: assert
    id: SEC-010
    message: "files must be owned by noah"
    resource_type: file
    condition_field: owner
    condition_value: noah
"#;
    let config = parse_config(yaml).unwrap();
    let violations = evaluate_policies(&config);
    assert!(violations.is_empty());
}
