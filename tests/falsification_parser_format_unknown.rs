//! FJ-2501: Format validation (mode, port, path, owner/group, cron, deny_paths, addr).
//!
//! Popperian rejection criteria for:
//! - FJ-2501: Octal mode, port range, absolute path, Unix names
//! - FJ-2501: Cron schedule field validation
//! - FJ-2501: Machine addr validation
//! - FJ-2300: deny_paths glob matching
//!
//! Usage: cargo test --test falsification_parser_format_unknown

use forjar::core::parser::{parse_config, validate_config};

// ============================================================================
// FJ-2501: validate_formats — mode validation
// ============================================================================

#[test]
fn format_valid_mode_0644() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  conf:
    type: file
    path: /etc/test.conf
    mode: "0644"
"#;
    let cfg = parse_config(yaml).unwrap();
    let errors = validate_config(&cfg);
    assert!(!errors.iter().any(|e| e.message.contains("mode")));
}

#[test]
fn format_valid_mode_setuid() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  conf:
    type: file
    path: /etc/test.conf
    mode: "1755"
"#;
    let cfg = parse_config(yaml).unwrap();
    let errors = validate_config(&cfg);
    assert!(!errors.iter().any(|e| e.message.contains("mode")));
}

#[test]
fn format_invalid_mode_octal_8() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  conf:
    type: file
    path: /etc/test.conf
    mode: "0888"
"#;
    let cfg = parse_config(yaml).unwrap();
    let errors = validate_config(&cfg);
    assert!(errors.iter().any(|e| e.message.contains("invalid mode")));
}

#[test]
fn format_invalid_mode_too_short() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  conf:
    type: file
    path: /etc/test.conf
    mode: "644"
"#;
    let cfg = parse_config(yaml).unwrap();
    let errors = validate_config(&cfg);
    assert!(errors.iter().any(|e| e.message.contains("invalid mode")));
}

#[test]
fn format_mode_template_skipped() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  conf:
    type: file
    path: /etc/test.conf
    mode: "{{params.mode}}"
"#;
    let cfg = parse_config(yaml).unwrap();
    let errors = validate_config(&cfg);
    assert!(!errors.iter().any(|e| e.message.contains("mode")));
}

// ============================================================================
// FJ-2501: validate_formats — port validation
// ============================================================================

#[test]
fn format_valid_port() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  fw:
    type: network
    port: "443"
    protocol: tcp
    action: allow
"#;
    let cfg = parse_config(yaml).unwrap();
    let errors = validate_config(&cfg);
    assert!(!errors.iter().any(|e| e.message.contains("port")));
}

#[test]
fn format_port_out_of_range() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  fw:
    type: network
    port: "70000"
    protocol: tcp
    action: allow
"#;
    let cfg = parse_config(yaml).unwrap();
    let errors = validate_config(&cfg);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("port") && e.message.contains("range")));
}

#[test]
fn format_port_zero() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  fw:
    type: network
    port: "0"
    protocol: tcp
    action: allow
"#;
    let cfg = parse_config(yaml).unwrap();
    let errors = validate_config(&cfg);
    assert!(errors.iter().any(|e| e.message.contains("port")));
}

// ============================================================================
// FJ-2501: validate_formats — path validation
// ============================================================================

#[test]
fn format_relative_path_rejected() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  conf:
    type: file
    path: relative/path.txt
"#;
    let cfg = parse_config(yaml).unwrap();
    let errors = validate_config(&cfg);
    assert!(errors.iter().any(|e| e.message.contains("absolute")));
}

#[test]
fn format_absolute_path_accepted() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  conf:
    type: file
    path: /etc/test.conf
"#;
    let cfg = parse_config(yaml).unwrap();
    let errors = validate_config(&cfg);
    assert!(!errors.iter().any(|e| e.message.contains("absolute")));
}

// ============================================================================
// FJ-2501: validate_formats — owner/group validation
// ============================================================================

#[test]
fn format_valid_owner_group() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  conf:
    type: file
    path: /etc/test.conf
    owner: www-data
    group: www-data
"#;
    let cfg = parse_config(yaml).unwrap();
    let errors = validate_config(&cfg);
    assert!(!errors
        .iter()
        .any(|e| e.message.contains("owner") || e.message.contains("group")));
}

#[test]
fn format_invalid_owner_starts_with_digit() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  conf:
    type: file
    path: /etc/test.conf
    owner: 0bad
"#;
    let cfg = parse_config(yaml).unwrap();
    let errors = validate_config(&cfg);
    assert!(errors.iter().any(|e| e.message.contains("invalid owner")));
}

#[test]
fn format_invalid_owner_uppercase() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  conf:
    type: file
    path: /etc/test.conf
    owner: Root
"#;
    let cfg = parse_config(yaml).unwrap();
    let errors = validate_config(&cfg);
    assert!(errors.iter().any(|e| e.message.contains("invalid owner")));
}

// ============================================================================
// FJ-2501: validate_formats — cron schedule validation
// ============================================================================

#[test]
fn format_valid_cron_schedule() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  job:
    type: cron
    name: backup
    schedule: "0 2 * * 1"
    command: /usr/bin/backup
"#;
    let cfg = parse_config(yaml).unwrap();
    let errors = validate_config(&cfg);
    assert!(!errors.iter().any(|e| e.message.contains("cron")));
}

#[test]
fn format_cron_keyword_accepted() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  job:
    type: cron
    name: backup
    schedule: "@daily"
    command: /usr/bin/backup
"#;
    let cfg = parse_config(yaml).unwrap();
    let errors = validate_config(&cfg);
    assert!(!errors.iter().any(|e| e.message.contains("cron")));
}

#[test]
fn format_cron_wrong_field_count() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  job:
    type: cron
    name: backup
    schedule: "0 2 *"
    command: /usr/bin/backup
"#;
    let cfg = parse_config(yaml).unwrap();
    let errors = validate_config(&cfg);
    assert!(errors.iter().any(|e| e.message.contains("5 fields")));
}

#[test]
fn format_cron_minute_out_of_range() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  job:
    type: cron
    name: backup
    schedule: "61 2 * * 1"
    command: /usr/bin/backup
"#;
    let cfg = parse_config(yaml).unwrap();
    let errors = validate_config(&cfg);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("minute") && e.message.contains("range")));
}

#[test]
fn format_cron_step_and_range() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  job:
    type: cron
    name: backup
    schedule: "*/5 0-6 1,15 * 1-5"
    command: /usr/bin/backup
"#;
    let cfg = parse_config(yaml).unwrap();
    let errors = validate_config(&cfg);
    assert!(!errors.iter().any(|e| e.message.contains("cron")));
}

// ============================================================================
// FJ-2501: validate_formats — machine addr validation
// ============================================================================

#[test]
fn format_machine_empty_addr() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web-01
    addr: ""
    user: deploy
    arch: x86_64
"#;
    let cfg = parse_config(yaml).unwrap();
    let errors = validate_config(&cfg);
    assert!(errors.iter().any(|e| e.message.contains("addr")));
}

#[test]
fn format_machine_addr_with_spaces() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web-01
    addr: "bad address"
    user: deploy
    arch: x86_64
"#;
    let cfg = parse_config(yaml).unwrap();
    let errors = validate_config(&cfg);
    assert!(errors.iter().any(|e| e.message.contains("addr")));
}

#[test]
fn format_machine_localhost_ok() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: localhost
    user: root
    arch: x86_64
"#;
    let cfg = parse_config(yaml).unwrap();
    let errors = validate_config(&cfg);
    assert!(!errors.iter().any(|e| e.message.contains("addr")));
}

// ============================================================================
// FJ-2501: deny_paths
// ============================================================================

#[test]
fn format_deny_paths_glob_match() {
    let yaml = r#"
version: "1.0"
name: test
policy:
  deny_paths:
    - "/proc/**"
resources:
  bad:
    type: file
    path: /proc/sys/net/ipv4/ip_forward
"#;
    let cfg = parse_config(yaml).unwrap();
    let errors = validate_config(&cfg);
    assert!(errors.iter().any(|e| e.message.contains("denied")));
}

#[test]
fn format_deny_paths_no_match() {
    let yaml = r#"
version: "1.0"
name: test
policy:
  deny_paths:
    - "/proc/**"
resources:
  ok:
    type: file
    path: /etc/safe.conf
"#;
    let cfg = parse_config(yaml).unwrap();
    let errors = validate_config(&cfg);
    assert!(!errors.iter().any(|e| e.message.contains("denied")));
}
