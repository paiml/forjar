//! Tests for FJ-2501 format validation (mode, port, path, owner, cron).

#![cfg(test)]

use super::format_validation::*;
use crate::core::types::ForjarConfig;

#[test]
fn valid_modes() {
    assert!(is_valid_mode("0644"));
    assert!(is_valid_mode("0755"));
    assert!(is_valid_mode("0600"));
    assert!(is_valid_mode("0777"));
    assert!(is_valid_mode("0000"));
    assert!(is_valid_mode("1755"));
}

#[test]
fn invalid_modes() {
    assert!(!is_valid_mode("644"));
    assert!(!is_valid_mode("0888"));
    assert!(!is_valid_mode("abcd"));
    assert!(!is_valid_mode(""));
    assert!(!is_valid_mode("07777"));
}

#[test]
fn valid_unix_names() {
    assert!(is_valid_unix_name("root"));
    assert!(is_valid_unix_name("www-data"));
    assert!(is_valid_unix_name("_apt"));
    assert!(is_valid_unix_name("nobody"));
    assert!(is_valid_unix_name("user123"));
}

#[test]
fn invalid_unix_names() {
    assert!(!is_valid_unix_name(""));
    assert!(!is_valid_unix_name("123user"));
    assert!(!is_valid_unix_name("Root"));
    assert!(!is_valid_unix_name("user.name"));
    assert!(!is_valid_unix_name("a".repeat(33).as_str()));
}

#[test]
fn format_validation_on_config() {
    let yaml = r#"
version: "1.0"
name: format-test
machines:
  web:
    hostname: web-01
    addr: 10.0.0.1
resources:
  cfg:
    type: file
    machine: web
    path: /etc/nginx.conf
    mode: "0644"
    owner: www-data
    group: www-data
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let errors = validate_formats(&config);
    assert!(errors.is_empty(), "expected no errors: {errors:?}");
}

#[test]
fn format_bad_mode_detected() {
    let yaml = r#"
version: "1.0"
name: bad-mode
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m
    path: /etc/test
    mode: "0999"
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let errors = validate_formats(&config);
    assert!(!errors.is_empty());
    assert!(errors[0].message.contains("invalid mode"));
}

#[test]
fn format_bad_owner_detected() {
    let yaml = r#"
version: "1.0"
name: bad-owner
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m
    path: /etc/test
    owner: "Bad User"
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let errors = validate_formats(&config);
    assert!(!errors.is_empty());
    assert!(errors[0].message.contains("invalid owner"));
}

#[test]
fn format_relative_path_detected() {
    let yaml = r#"
version: "1.0"
name: rel-path
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m
    path: relative/path.txt
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let errors = validate_formats(&config);
    assert!(!errors.is_empty());
    assert!(errors[0].message.contains("must be absolute"));
}

#[test]
fn format_bad_machine_addr() {
    let yaml = r#"
version: "1.0"
name: bad-addr
machines:
  m:
    hostname: m
    addr: "has spaces"
resources: {}
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let errors = validate_formats(&config);
    assert!(!errors.is_empty());
    assert!(errors[0].message.contains("invalid addr"));
}

#[test]
fn format_template_expressions_skipped() {
    let yaml = r#"
version: "1.0"
name: template
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m
    path: "{{params.config_path}}"
    mode: "{{params.file_mode}}"
    owner: "{{params.owner}}"
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let errors = validate_formats(&config);
    assert!(errors.is_empty(), "templates should be skipped: {errors:?}");
}

#[test]
fn valid_cron_fields() {
    assert!(validate_cron_field("*", 0, 59).is_ok());
    assert!(validate_cron_field("0", 0, 59).is_ok());
    assert!(validate_cron_field("59", 0, 59).is_ok());
    assert!(validate_cron_field("*/5", 0, 59).is_ok());
    assert!(validate_cron_field("1-31", 1, 31).is_ok());
    assert!(validate_cron_field("0,15,30,45", 0, 59).is_ok());
    assert!(validate_cron_field("1-5", 0, 7).is_ok());
}

#[test]
fn invalid_cron_fields() {
    assert!(validate_cron_field("60", 0, 59).is_err());
    assert!(validate_cron_field("*/0", 0, 59).is_err());
    assert!(validate_cron_field("32", 1, 31).is_err());
    assert!(validate_cron_field("0", 1, 31).is_err());
    assert!(validate_cron_field("5-2", 0, 7).is_err());
    assert!(validate_cron_field("abc", 0, 59).is_err());
    assert!(validate_cron_field("8", 0, 7).is_err());
}

#[test]
fn cron_schedule_valid_config() {
    let yaml = r#"
version: "1.0"
name: cron-test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  job:
    type: cron
    machine: m
    command: /usr/bin/backup
    schedule: "0 2 * * 1-5"
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let errors = validate_formats(&config);
    assert!(errors.is_empty(), "expected no errors: {errors:?}");
}

#[test]
fn cron_schedule_bad_range_detected() {
    let yaml = r#"
version: "1.0"
name: cron-bad
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  job:
    type: cron
    machine: m
    command: /usr/bin/backup
    schedule: "99 25 32 13 8"
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let errors = validate_formats(&config);
    assert!(errors.len() >= 4, "expected errors for all 5 fields: {errors:?}");
}

#[test]
fn cron_schedule_wrong_field_count() {
    let yaml = r#"
version: "1.0"
name: cron-fields
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  job:
    type: cron
    machine: m
    command: /usr/bin/backup
    schedule: "0 * *"
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let errors = validate_formats(&config);
    assert!(errors.iter().any(|e| e.message.contains("5 fields")));
}

#[test]
fn cron_schedule_keywords_accepted() {
    let yaml = r#"
version: "1.0"
name: cron-keyword
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  job:
    type: cron
    machine: m
    command: /usr/bin/backup
    schedule: "@daily"
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let errors = validate_formats(&config);
    assert!(errors.is_empty(), "keywords should pass: {errors:?}");
}

#[test]
fn cron_schedule_template_skipped() {
    let yaml = r#"
version: "1.0"
name: cron-tmpl
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  job:
    type: cron
    machine: m
    command: /usr/bin/backup
    schedule: "{{params.schedule}}"
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let errors = validate_formats(&config);
    assert!(errors.is_empty(), "templates should be skipped: {errors:?}");
}

#[test]
fn deny_paths_blocks_forbidden_path() {
    let yaml = r#"
version: "1.0"
name: deny-test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
policy:
  deny_paths:
    - /etc/shadow
    - /root/**
resources:
  shadow:
    type: file
    machine: m
    path: /etc/shadow
    content: "x"
  root-cfg:
    type: file
    machine: m
    path: /root/.bashrc
    content: "x"
  allowed:
    type: file
    machine: m
    path: /opt/app/config.yaml
    content: "x"
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let errors = validate_formats(&config);
    let deny_errors: Vec<_> = errors.iter().filter(|e| e.message.contains("denied")).collect();
    assert_eq!(deny_errors.len(), 2, "should deny /etc/shadow and /root/.bashrc: {deny_errors:?}");
}

#[test]
fn deny_paths_empty_no_errors() {
    let yaml = r#"
version: "1.0"
name: no-deny
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m
    path: /etc/shadow
    content: "x"
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let errors = validate_formats(&config);
    assert!(errors.iter().all(|e| !e.message.contains("denied")));
}

#[test]
fn path_glob_matching() {
    assert!(path_matches_glob("/etc/shadow", "/etc/shadow"));
    assert!(path_matches_glob("/root/.bashrc", "/root/**"));
    assert!(path_matches_glob("/root/deep/path", "/root/**"));
    assert!(!path_matches_glob("/home/user/.bashrc", "/root/**"));
    assert!(path_matches_glob("/etc/passwd", "/etc/*"));
    assert!(!path_matches_glob("/var/etc/file", "/etc/*"));
}

#[test]
fn format_port_out_of_range() {
    let yaml = r#"
version: "1.0"
name: port-test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  fw:
    type: network
    machine: m
    port: 99999
    protocol: tcp
    action: allow
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let errors = validate_formats(&config);
    assert!(errors.iter().any(|e| e.message.contains("port")));
}
