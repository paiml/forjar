//! FJ-2501/2504/2500: Popperian falsification for format validation,
//! LSP diagnostics, and unknown field detection.
//!
//! Each test states conditions under which the validation or LSP
//! subsystem would be rejected as invalid.

use forjar::cli::lsp::{CompletionItem, Diagnostic, DiagnosticSeverity, LspServer};
use forjar::core::parser::{check_unknown_fields, parse_config, validate_config};

// ── FJ-2501: Format Validation — Mode ──────────────────────────────

#[test]
fn f_2501_1_valid_mode_accepted() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  conf:
    type: file
    path: /etc/app.conf
    mode: "0644"
    content: "hello"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let mode_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("mode"))
        .collect();
    assert!(
        mode_errors.is_empty(),
        "valid mode '0644' must be accepted: {mode_errors:?}"
    );
}

#[test]
fn f_2501_2_invalid_mode_rejected() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  conf:
    type: file
    path: /etc/app.conf
    mode: "999"
    content: "hello"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let mode_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("mode"))
        .collect();
    assert!(
        !mode_errors.is_empty(),
        "invalid mode '999' (3 digits, digit 9 out of range) must be rejected"
    );
}

#[test]
fn f_2501_3_octal_mode_rejects_non_octal_digit() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  conf:
    type: file
    path: /etc/app.conf
    mode: "0894"
    content: "hello"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.message.contains("mode")),
        "mode '0894' contains digit 8/9 — must be rejected"
    );
}

#[test]
fn f_2501_4_setuid_mode_accepted() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  bin:
    type: file
    path: /usr/local/bin/tool
    mode: "4755"
    content: "binary"
    sudo: true
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let mode_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("mode"))
        .collect();
    assert!(
        mode_errors.is_empty(),
        "setuid mode '4755' must be accepted"
    );
}

// ── FJ-2501: Format Validation — Port ──────────────────────────────

#[test]
fn f_2501_5_valid_port_accepted() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  svc:
    type: service
    port: "8080"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let port_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("port"))
        .collect();
    assert!(port_errors.is_empty(), "port 8080 must be accepted");
}

#[test]
fn f_2501_6_port_zero_rejected() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  svc:
    type: service
    port: "0"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.message.contains("port")),
        "port 0 must be rejected (range 1-65535)"
    );
}

#[test]
fn f_2501_7_port_too_high_rejected() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  svc:
    type: service
    port: "70000"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.message.contains("port")),
        "port 70000 must be rejected (> 65535)"
    );
}

// ── FJ-2501: Format Validation — Path ──────────────────────────────

#[test]
fn f_2501_8_absolute_path_accepted() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  conf:
    type: file
    path: /etc/app.conf
    content: "hello"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let path_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("must be absolute"))
        .collect();
    assert!(
        path_errors.is_empty(),
        "absolute path /etc/app.conf must be accepted"
    );
}

#[test]
fn f_2501_9_relative_path_rejected() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  conf:
    type: file
    path: relative/path.conf
    content: "hello"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("must be absolute")),
        "relative path must be rejected"
    );
}

// ── FJ-2501: Format Validation — Owner/Group ───────────────────────

#[test]
fn f_2501_10_valid_unix_owner_accepted() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  conf:
    type: file
    path: /etc/app.conf
    owner: www-data
    content: "hello"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let owner_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("owner"))
        .collect();
    assert!(
        owner_errors.is_empty(),
        "valid unix owner 'www-data' must be accepted"
    );
}

#[test]
fn f_2501_11_invalid_owner_rejected() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  conf:
    type: file
    path: /etc/app.conf
    owner: "Invalid User!"
    content: "hello"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.message.contains("owner")),
        "owner 'Invalid User!' (uppercase, space, !) must be rejected"
    );
}

// ── FJ-2501: Format Validation — Cron Schedule ─────────────────────

#[test]
fn f_2501_12_valid_cron_schedule_accepted() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  backup:
    type: cron
    machine: m
    name: backup-job
    schedule: "0 2 * * 0"
    command: "backup.sh"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let sched_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("cron"))
        .collect();
    assert!(
        sched_errors.is_empty(),
        "valid cron schedule '0 2 * * 0' must be accepted: {sched_errors:?}"
    );
}

#[test]
fn f_2501_13_cron_schedule_wrong_field_count_rejected() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  backup:
    type: cron
    machine: m
    name: backup-job
    schedule: "0 2 *"
    command: "backup.sh"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.message.contains("5 fields")),
        "cron schedule with 3 fields must be rejected"
    );
}

#[test]
fn f_2501_14_cron_minute_out_of_range_rejected() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  backup:
    type: cron
    machine: m
    name: backup-job
    schedule: "65 2 * * 0"
    command: "backup.sh"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.message.contains("minute")),
        "cron minute 65 (> 59) must be rejected"
    );
}

#[test]
fn f_2501_15_cron_keyword_accepted() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  backup:
    type: cron
    machine: m
    name: backup-job
    schedule: "@daily"
    command: "backup.sh"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let sched_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("cron"))
        .collect();
    assert!(
        sched_errors.is_empty(),
        "cron keyword '@daily' must be accepted: {sched_errors:?}"
    );
}

#[test]
fn f_2501_16_cron_step_and_range_accepted() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  backup:
    type: cron
    machine: m
    name: backup-job
    schedule: "*/15 9-17 * * 1-5"
    command: "backup.sh"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let sched_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("cron"))
        .collect();
    assert!(
        sched_errors.is_empty(),
        "cron with step (*/15) and ranges (9-17, 1-5) must be accepted: {sched_errors:?}"
    );
}

// ── FJ-2501: Template Expressions Skip Validation ──────────────────

#[test]
fn f_2501_17_template_mode_skips_validation() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  conf:
    type: file
    path: /etc/app.conf
    mode: "{{params.mode}}"
    content: "hello"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let mode_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("mode"))
        .collect();
    assert!(
        mode_errors.is_empty(),
        "template expression '{{{{params.mode}}}}' must skip validation"
    );
}

// ── FJ-2500: Unknown Field Detection ───────────────────────────────

#[test]
fn f_2500_1_known_fields_no_warnings() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  pkg:
    type: package
    packages: [curl]
"#;
    let warnings = check_unknown_fields(yaml);
    assert!(
        warnings.is_empty(),
        "valid config with known fields must produce no warnings"
    );
}

#[test]
fn f_2500_2_unknown_resource_field_flagged() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  pkg:
    type: package
    packages: [curl]
    bogus_field: true
"#;
    let warnings = check_unknown_fields(yaml);
    assert!(
        !warnings.is_empty(),
        "unknown field 'bogus_field' must be flagged"
    );
}

#[test]
fn f_2500_3_unknown_top_level_key_flagged() {
    let yaml = r#"
version: "1.0"
name: test
frobnicate: true
resources:
  pkg:
    type: package
    packages: [curl]
"#;
    let warnings = check_unknown_fields(yaml);
    assert!(
        !warnings.is_empty(),
        "unknown top-level key 'frobnicate' must be flagged"
    );
}

// ── FJ-2504: LSP — Validate YAML ──────────────────────────────────
