//! FJ-3103/202/1390/3307: Cron parsing, conditional evaluation, security
//! scanning, and script secret lint falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-3103: Cron source
//!   - parse_cron: 5-field parsing, wildcards, steps, ranges, lists, errors
//!   - matches: schedule+time matching
//!   - schedule_summary: display format
//! - FJ-202: Conditional resource evaluation
//!   - evaluate_when: literals, equality, inequality, contains, machine refs
//! - FJ-1390: Security scanner
//!   - scan: hardcoded secrets, HTTP without TLS, world-accessible, weak crypto
//!   - severity_counts: tally by severity level
//! - FJ-3307: Script secret lint
//!   - scan_script: pattern detection, comment skipping
//!   - validate_no_leaks: clean vs dirty scripts
//!
//! Usage: cargo test --test falsification_cron_conditions_scanner

use forjar::core::conditions::evaluate_when;
use forjar::core::cron_source::{matches, parse_cron, schedule_summary, CronTime};
use forjar::core::script_secret_lint::{scan_script, validate_no_leaks};
use forjar::core::security_scanner::{scan, severity_counts, Severity};
use forjar::core::types::*;
use indexmap::IndexMap;
use std::collections::HashMap;

// ============================================================================
// FJ-3103: parse_cron
// ============================================================================

#[test]
fn cron_star_all_fields() {
    let s = parse_cron("* * * * *").unwrap();
    assert_eq!(s.minutes.len(), 60);
    assert_eq!(s.hours.len(), 24);
    assert_eq!(s.days_of_month.len(), 31);
    assert_eq!(s.months.len(), 12);
    assert_eq!(s.days_of_week.len(), 7);
}

#[test]
fn cron_exact_values() {
    let s = parse_cron("0 9 1 1 0").unwrap();
    assert!(s.minutes.contains(&0));
    assert!(s.hours.contains(&9));
    assert!(s.days_of_month.contains(&1));
    assert!(s.months.contains(&1));
    assert!(s.days_of_week.contains(&0));
}

#[test]
fn cron_step_every_15_minutes() {
    let s = parse_cron("*/15 * * * *").unwrap();
    assert_eq!(s.minutes.len(), 4);
    assert!(s.minutes.contains(&0));
    assert!(s.minutes.contains(&15));
    assert!(s.minutes.contains(&30));
    assert!(s.minutes.contains(&45));
}

#[test]
fn cron_range() {
    let s = parse_cron("* 9-17 * * *").unwrap();
    assert_eq!(s.hours.len(), 9);
    assert!(s.hours.contains(&9));
    assert!(s.hours.contains(&17));
    assert!(!s.hours.contains(&8));
}

#[test]
fn cron_list() {
    let s = parse_cron("0 6,12,18 * * *").unwrap();
    assert_eq!(s.hours.len(), 3);
}

#[test]
fn cron_mixed_expression() {
    let s = parse_cron("0,30 */6 1-15 * 1-5").unwrap();
    assert_eq!(s.minutes.len(), 2);
    assert_eq!(s.hours.len(), 4); // 0, 6, 12, 18
    assert!(s.days_of_month.contains(&1) && s.days_of_month.contains(&15));
    assert!(!s.days_of_month.contains(&16));
    assert_eq!(s.days_of_week.len(), 5); // Mon-Fri
}

#[test]
fn cron_parse_errors() {
    assert!(parse_cron("* *").is_err()); // too few fields
    assert!(parse_cron("* * * * * *").is_err()); // too many fields
    assert!(parse_cron("60 * * * *").is_err()); // minute out of range
    assert!(parse_cron("* 25 * * *").is_err()); // hour out of range
    assert!(parse_cron("*/0 * * * *").is_err()); // zero step
    assert!(parse_cron("* 17-9 * * *").is_err()); // reversed range
}

// ============================================================================
// FJ-3103: matches
// ============================================================================

fn ct(minute: u8, hour: u8, day: u8, month: u8, weekday: u8) -> CronTime {
    CronTime {
        minute,
        hour,
        day,
        month,
        weekday,
    }
}

#[test]
fn cron_matches_exact() {
    let s = parse_cron("30 12 15 6 3").unwrap();
    assert!(matches(&s, &ct(30, 12, 15, 6, 3)));
}

#[test]
fn cron_no_match_wrong_minute() {
    let s = parse_cron("30 12 15 6 3").unwrap();
    assert!(!matches(&s, &ct(31, 12, 15, 6, 3)));
}

#[test]
fn cron_wildcard_matches_all() {
    let s = parse_cron("* * * * *").unwrap();
    assert!(matches(&s, &ct(42, 3, 28, 2, 0)));
}

#[test]
fn cron_step_hit_and_miss() {
    let s = parse_cron("*/15 * * * *").unwrap();
    assert!(matches(&s, &ct(45, 0, 1, 1, 0)));
    assert!(!matches(&s, &ct(7, 0, 1, 1, 0)));
}

#[test]
fn cron_weekday_range_match() {
    let s = parse_cron("0 9 * * 1-5").unwrap();
    assert!(matches(&s, &ct(0, 9, 1, 1, 1))); // Monday
    assert!(!matches(&s, &ct(0, 9, 1, 1, 0))); // Sunday
}

// ============================================================================
// FJ-3103: schedule_summary
// ============================================================================

#[test]
fn cron_summary_exact() {
    let s = parse_cron("0 12 * * *").unwrap();
    let summary = schedule_summary(&s);
    assert!(summary.contains("min=0"));
    assert!(summary.contains("hr=12"));
}

#[test]
fn cron_summary_star_condensed() {
    let s = parse_cron("* * * * *").unwrap();
    let summary = schedule_summary(&s);
    assert!(summary.contains("*(60 values)"));
}

// ============================================================================
// FJ-202: evaluate_when — helpers
// ============================================================================

fn test_machine(arch: &str) -> Machine {
    Machine {
        hostname: "test-host".into(),
        addr: "10.0.0.1".into(),
        user: "root".into(),
        arch: arch.into(),
        ssh_key: None,
        roles: vec!["web".into(), "gpu".into()],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    }
}

// ============================================================================
// FJ-202: evaluate_when — literals
// ============================================================================

#[test]
fn when_literal_true() {
    assert!(evaluate_when("true", &HashMap::new(), &test_machine("x86_64")).unwrap());
}

#[test]
fn when_literal_false() {
    assert!(!evaluate_when("false", &HashMap::new(), &test_machine("x86_64")).unwrap());
}

#[test]
fn when_case_insensitive_true() {
    assert!(evaluate_when("TRUE", &HashMap::new(), &test_machine("x86_64")).unwrap());
}

// ============================================================================
// FJ-202: evaluate_when — equality
// ============================================================================

#[test]
fn when_arch_equals() {
    let m = test_machine("x86_64");
    assert!(evaluate_when("{{machine.arch}} == \"x86_64\"", &HashMap::new(), &m).unwrap());
}

#[test]
fn when_arch_not_match() {
    let m = test_machine("aarch64");
    assert!(!evaluate_when("{{machine.arch}} == \"x86_64\"", &HashMap::new(), &m).unwrap());
}

#[test]
fn when_param_equals() {
    let mut p = HashMap::new();
    p.insert("env".into(), serde_yaml_ng::Value::String("prod".into()));
    let m = test_machine("x86_64");
    assert!(evaluate_when("{{params.env}} == \"prod\"", &p, &m).unwrap());
}

#[test]
fn when_not_equals() {
    let mut p = HashMap::new();
    p.insert("env".into(), serde_yaml_ng::Value::String("prod".into()));
    let m = test_machine("x86_64");
    assert!(evaluate_when("{{params.env}} != \"staging\"", &p, &m).unwrap());
}

// ============================================================================
// FJ-202: evaluate_when — contains
// ============================================================================

#[test]
fn when_roles_contains() {
    let m = test_machine("x86_64");
    assert!(evaluate_when("{{machine.roles}} contains \"gpu\"", &HashMap::new(), &m).unwrap());
}

#[test]
fn when_roles_not_contains() {
    let m = test_machine("x86_64");
    assert!(!evaluate_when(
        "{{machine.roles}} contains \"storage\"",
        &HashMap::new(),
        &m
    )
    .unwrap());
}

// ============================================================================
// FJ-202: evaluate_when — errors
// ============================================================================

#[test]
fn when_unknown_param_errors() {
    let m = test_machine("x86_64");
    let err = evaluate_when("{{params.missing}} == \"x\"", &HashMap::new(), &m).unwrap_err();
    assert!(err.contains("unknown"), "err: {err}");
}

#[test]
fn when_invalid_operator_errors() {
    let m = test_machine("x86_64");
    let err = evaluate_when("no operator here", &HashMap::new(), &m).unwrap_err();
    assert!(err.contains("invalid"), "err: {err}");
}

// ============================================================================
// FJ-1390: security scanner — scan
// ============================================================================

fn config_with_resource(id: &str, resource: Resource) -> ForjarConfig {
    let mut resources = IndexMap::new();
    resources.insert(id.into(), resource);
    ForjarConfig {
        name: "test".into(),
        resources,
        ..Default::default()
    }
}

#[test]
fn scanner_detects_hardcoded_secret() {
    let r = Resource {
        resource_type: ResourceType::File,
        content: Some("password=s3cret123".into()),
        ..Default::default()
    };
    let findings = scan(&config_with_resource("cfg", r));
    assert!(!findings.is_empty());
    assert_eq!(findings[0].rule_id, "SS-1");
    assert_eq!(findings[0].severity, Severity::Critical);
}

#[test]
fn scanner_http_without_tls_and_exceptions() {
    // HTTP to external host → flagged
    let r1 = Resource {
        resource_type: ResourceType::File,
        source: Some("http://example.com/data".into()),
        ..Default::default()
    };
    assert!(scan(&config_with_resource("dl", r1))
        .iter()
        .any(|f| f.rule_id == "SS-2"));
    // HTTPS → not flagged
    let r2 = Resource {
        resource_type: ResourceType::File,
        source: Some("https://example.com/data".into()),
        ..Default::default()
    };
    assert!(!scan(&config_with_resource("dl", r2))
        .iter()
        .any(|f| f.rule_id == "SS-2"));
    // HTTP localhost → not flagged
    let r3 = Resource {
        resource_type: ResourceType::File,
        source: Some("http://localhost:8080/health".into()),
        ..Default::default()
    };
    assert!(!scan(&config_with_resource("hc", r3))
        .iter()
        .any(|f| f.rule_id == "SS-2"));
}

#[test]
fn scanner_world_accessible_mode() {
    // 0644 → world-readable only → NOT flagged (SS-3 only flags writable/executable)
    let r1 = Resource {
        resource_type: ResourceType::File,
        mode: Some("0644".into()),
        ..Default::default()
    };
    assert!(!scan(&config_with_resource("f", r1))
        .iter()
        .any(|f| f.rule_id == "SS-3"));
    // 0666 → world-writable → flagged
    let r1w = Resource {
        resource_type: ResourceType::File,
        mode: Some("0666".into()),
        ..Default::default()
    };
    assert!(scan(&config_with_resource("f", r1w))
        .iter()
        .any(|f| f.rule_id == "SS-3"));
    // 0600 → restrictive → not flagged
    let r2 = Resource {
        resource_type: ResourceType::File,
        mode: Some("0600".into()),
        ..Default::default()
    };
    assert!(!scan(&config_with_resource("f", r2))
        .iter()
        .any(|f| f.rule_id == "SS-3"));
}

#[test]
fn scanner_crypto_protocol_network_rules() {
    // SS-7: weak crypto
    let r1 = Resource {
        resource_type: ResourceType::File,
        content: Some("ssl_ciphers DES-CBC3-SHA;".into()),
        ..Default::default()
    };
    assert!(scan(&config_with_resource("ssl", r1))
        .iter()
        .any(|f| f.rule_id == "SS-7"));
    // SS-8: insecure protocol
    let r2 = Resource {
        resource_type: ResourceType::File,
        content: Some("url = ftp://files.example.com/data".into()),
        ..Default::default()
    };
    assert!(scan(&config_with_resource("ftp", r2))
        .iter()
        .any(|f| f.rule_id == "SS-8"));
    // SS-9: unrestricted network
    let r3 = Resource {
        resource_type: ResourceType::File,
        content: Some("bind_address: 0.0.0.0".into()),
        ..Default::default()
    };
    assert!(scan(&config_with_resource("svc", r3))
        .iter()
        .any(|f| f.rule_id == "SS-9"));
}

#[test]
fn scanner_clean_config_no_findings() {
    let r = Resource {
        resource_type: ResourceType::File,
        content: Some("server_name example.com;".into()),
        mode: Some("0600".into()),
        ..Default::default()
    };
    let findings = scan(&config_with_resource("clean", r));
    assert!(findings.is_empty(), "findings: {findings:?}");
}

#[test]
fn scanner_severity_counts_multiple() {
    let mut resources = IndexMap::new();
    resources.insert(
        "s".into(),
        Resource {
            resource_type: ResourceType::File,
            content: Some("password=abc".into()),
            ..Default::default()
        },
    );
    resources.insert(
        "h".into(),
        Resource {
            resource_type: ResourceType::File,
            source: Some("http://remote.com/data".into()),
            ..Default::default()
        },
    );
    let config = ForjarConfig {
        name: "t".into(),
        resources,
        ..Default::default()
    };
    let (c, h, _m, _l) = severity_counts(&scan(&config));
    assert!(c >= 1 && h >= 1);
}

// ============================================================================
// FJ-3307: scan_script
// ============================================================================

#[test]
fn script_lint_clean() {
    let result = scan_script("#!/bin/bash\nset -euo pipefail\napt-get install -y nginx\n");
    assert!(result.clean());
    assert_eq!(result.lines_scanned, 3);
}

#[test]
fn script_lint_echo_password() {
    let result = scan_script("echo $PASSWORD > /tmp/log\n");
    assert!(!result.clean());
    assert_eq!(result.findings[0].pattern_name, "echo_secret_var");
}

#[test]
fn script_lint_curl_inline() {
    let result = scan_script("curl -u admin:hunter2 https://api.example.com\n");
    assert!(!result.clean());
    assert_eq!(result.findings[0].pattern_name, "curl_inline_creds");
}

#[test]
fn script_lint_aws_key() {
    let result = scan_script("export AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE\n");
    assert!(!result.clean());
    let names: Vec<&str> = result
        .findings
        .iter()
        .map(|f| f.pattern_name.as_str())
        .collect();
    assert!(names.contains(&"aws_key_in_script"));
}

#[test]
fn script_lint_private_key() {
    let result = scan_script("cat <<'EOF'\n-----BEGIN RSA PRIVATE KEY-----\nMIIE...\nEOF\n");
    assert!(!result.clean());
    assert_eq!(result.findings[0].pattern_name, "private_key_inline");
}

#[test]
fn script_lint_db_url_password() {
    let result = scan_script("DATABASE_URL=postgres://app:s3cret@db.local:5432/prod\n");
    assert!(!result.clean());
    assert_eq!(result.findings[0].pattern_name, "db_url_embedded_pass");
}

#[test]
fn script_lint_comments_skipped() {
    let result = scan_script("# echo $PASSWORD\n# sshpass -p secret ssh host\n");
    assert!(result.clean());
}

#[test]
fn script_lint_multiple_findings() {
    let script = "echo $TOKEN\ncurl -u admin:pass https://api.com\nsshpass -p pw ssh h\n";
    let result = scan_script(script);
    assert!(result.findings.len() >= 3);
}

// ============================================================================
// FJ-3307: validate_no_leaks
// ============================================================================

#[test]
fn validate_no_leaks_clean_passes() {
    assert!(validate_no_leaks("#!/bin/bash\necho hello\n").is_ok());
}

#[test]
fn validate_no_leaks_dirty_fails() {
    let err = validate_no_leaks("echo $PASSWORD\n").unwrap_err();
    assert!(err.contains("secret leakage detected"));
}
