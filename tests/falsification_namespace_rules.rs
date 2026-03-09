//! FJ-3306/3108: Secret namespace isolation and rulebook validation.
//! Usage: cargo test --test falsification_namespace_rules

use forjar::core::ephemeral::ResolvedEphemeral;
use forjar::core::rules_engine::{
    event_type_coverage, validate_rulebook_config, validate_rulebook_file, validate_rulebook_yaml,
    IssueSeverity, RuleIssue, ValidationSummary,
};
use forjar::core::secret_namespace::*;
use forjar::core::types::*;

// ============================================================================
// FJ-3306: build_isolated_env
// ============================================================================

fn make_secret(key: &str, value: &str) -> ResolvedEphemeral {
    ResolvedEphemeral {
        key: key.into(),
        value: value.into(),
        hash: blake3::hash(value.as_bytes()).to_hex().to_string(),
    }
}

#[test]
fn build_env_includes_secrets() {
    let config = NamespaceConfig {
        inherit_env: vec![],
        ..Default::default()
    };
    let secrets = vec![make_secret("DB_PASS", "s3cret")];
    let env = build_isolated_env(&config, &secrets);
    assert_eq!(env.get("DB_PASS").unwrap(), "s3cret");
}

#[test]
fn build_env_namespace_marker() {
    let config = NamespaceConfig {
        namespace_id: "ns-test-42".into(),
        inherit_env: vec![],
        ..Default::default()
    };
    let env = build_isolated_env(&config, &[]);
    assert_eq!(env.get("FORJAR_NAMESPACE").unwrap(), "ns-test-42");
}

#[test]
fn build_env_no_extra_vars() {
    let config = NamespaceConfig {
        inherit_env: vec![],
        ..Default::default()
    };
    let secrets = vec![make_secret("K", "V")];
    let env = build_isolated_env(&config, &secrets);
    assert_eq!(env.len(), 2); // K + FORJAR_NAMESPACE
}

#[test]
fn build_env_multiple_secrets() {
    let config = NamespaceConfig {
        inherit_env: vec![],
        ..Default::default()
    };
    let secrets = vec![
        make_secret("A", "1"),
        make_secret("B", "2"),
        make_secret("C", "3"),
    ];
    let env = build_isolated_env(&config, &secrets);
    assert_eq!(env.get("A").unwrap(), "1");
    assert_eq!(env.get("B").unwrap(), "2");
    assert_eq!(env.get("C").unwrap(), "3");
}

#[test]
fn build_env_inherits_path() {
    let config = NamespaceConfig::default();
    let env = build_isolated_env(&config, &[]);
    if std::env::var("PATH").is_ok() {
        assert!(env.contains_key("PATH"));
    }
}

#[test]
fn build_env_empty_secrets() {
    let config = NamespaceConfig {
        inherit_env: vec![],
        ..Default::default()
    };
    let env = build_isolated_env(&config, &[]);
    assert_eq!(env.len(), 1); // Only FORJAR_NAMESPACE
}

// ============================================================================
// FJ-3306: verify_no_leak
// ============================================================================

#[test]
fn verify_no_leak_nonexistent_key() {
    assert!(verify_no_leak("FORJAR_TEST_NONEXISTENT_KEY_999"));
}

// ============================================================================
// FJ-3306: execute_isolated
// ============================================================================

#[test]
fn execute_echo_secret() {
    let config = NamespaceConfig {
        audit_enabled: false,
        ..Default::default()
    };
    let secrets = vec![make_secret("MY_SECRET", "hidden")];
    let result = execute_isolated(&config, &secrets, "sh", &["-c", "echo $MY_SECRET"]).unwrap();
    assert!(result.success);
    assert_eq!(result.stdout.trim(), "hidden");
    assert_eq!(result.secrets_injected, 1);
    assert_eq!(result.secrets_discarded, 1);
}

#[test]
fn execute_failing_command() {
    let config = NamespaceConfig {
        audit_enabled: false,
        ..Default::default()
    };
    let result = execute_isolated(&config, &[], "false", &[]).unwrap();
    assert!(!result.success);
    assert_eq!(result.exit_code, Some(1));
}

#[test]
fn execute_no_parent_env_leak() {
    let config = NamespaceConfig {
        inherit_env: vec![],
        audit_enabled: false,
        ..Default::default()
    };
    let result = execute_isolated(&config, &[], "sh", &["-c", "echo ${HOME:-UNSET}"]).unwrap();
    assert_eq!(result.stdout.trim(), "UNSET");
}

#[test]
fn execute_namespace_id_in_child() {
    let config = NamespaceConfig {
        namespace_id: "ns-custom-id".into(),
        audit_enabled: false,
        ..Default::default()
    };
    let result = execute_isolated(&config, &[], "sh", &["-c", "echo $FORJAR_NAMESPACE"]).unwrap();
    assert_eq!(result.stdout.trim(), "ns-custom-id");
}

// ============================================================================
// FJ-3306: format_result
// ============================================================================

#[test]
fn format_result_success() {
    let result = NamespaceResult {
        namespace_id: "ns-test-1".into(),
        success: true,
        exit_code: Some(0),
        stdout: String::new(),
        stderr: String::new(),
        secrets_injected: 2,
        secrets_discarded: 2,
    };
    let text = format_result(&result);
    assert!(text.contains("SUCCESS"));
    assert!(text.contains("ns-test-1"));
    assert!(text.contains("2/2"));
}

#[test]
fn format_result_failure() {
    let result = NamespaceResult {
        namespace_id: "ns-fail".into(),
        success: false,
        exit_code: Some(1),
        stdout: String::new(),
        stderr: "error".into(),
        secrets_injected: 1,
        secrets_discarded: 1,
    };
    let text = format_result(&result);
    assert!(text.contains("FAILED"));
    assert!(text.contains("ns-fail"));
}

#[test]
fn format_result_no_exit_code() {
    let result = NamespaceResult {
        namespace_id: "ns-sig".into(),
        success: false,
        exit_code: None,
        stdout: String::new(),
        stderr: String::new(),
        secrets_injected: 0,
        secrets_discarded: 0,
    };
    let text = format_result(&result);
    assert!(text.contains("-")); // no exit code → dash
}

// ============================================================================
// FJ-3306: NamespaceConfig defaults
// ============================================================================

#[test]
fn namespace_config_defaults() {
    let config = NamespaceConfig::default();
    assert!(config.namespace_id.starts_with("ns-forjar-"));
    assert!(config.audit_enabled);
    assert!(config.state_dir.is_none());
    assert!(config.inherit_env.contains(&"PATH".to_string()));
    assert!(config.inherit_env.contains(&"HOME".to_string()));
}

// ============================================================================
// FJ-3108: validate_rulebook_yaml — valid
// ============================================================================

fn valid_yaml() -> &'static str {
    r#"
rulebooks:
  - name: config-repair
    events:
      - type: file_changed
        match:
          path: /etc/nginx/nginx.conf
    actions:
      - apply:
          file: forjar.yaml
          tags: [config]
    cooldown_secs: 60
"#
}

#[test]
fn validate_valid_rulebook() {
    let issues = validate_rulebook_yaml(valid_yaml()).unwrap();
    assert!(issues.is_empty());
}

#[test]
fn validate_parse_error() {
    assert!(validate_rulebook_yaml("not: valid: [yaml").is_err());
}

#[test]
fn validate_no_events() {
    let yaml = r#"
rulebooks:
  - name: bad
    events: []
    actions:
      - script: "echo ok"
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    assert!(issues.iter().any(|i| i.message.contains("no event")));
}

#[test]
fn validate_no_actions() {
    let yaml = r#"
rulebooks:
  - name: bad
    events:
      - type: manual
    actions: []
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    assert!(issues.iter().any(|i| i.message.contains("no actions")));
}

#[test]
fn validate_duplicate_names() {
    let yaml = r#"
rulebooks:
  - name: dupe
    events: [{type: manual}]
    actions: [{script: "echo 1"}]
  - name: dupe
    events: [{type: manual}]
    actions: [{script: "echo 2"}]
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    assert!(issues.iter().any(|i| i.message.contains("duplicate")));
}

#[test]
fn validate_empty_apply_file() {
    let yaml = r#"
rulebooks:
  - name: bad-apply
    events: [{type: manual}]
    actions:
      - apply:
          file: ""
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    assert!(issues
        .iter()
        .any(|i| i.message.contains("apply.file is empty")));
}

#[test]
fn validate_zero_cooldown_warning() {
    let yaml = r#"
rulebooks:
  - name: rapid
    events: [{type: manual}]
    actions: [{script: "echo ok"}]
    cooldown_secs: 0
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    assert!(issues.iter().any(|i| {
        i.severity == IssueSeverity::Warning && i.message.contains("cooldown_secs=0")
    }));
}

#[test]
fn validate_high_retries_warning() {
    let yaml = r#"
rulebooks:
  - name: retry
    events: [{type: manual}]
    actions: [{script: "echo ok"}]
    max_retries: 50
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    assert!(issues.iter().any(|i| i.message.contains("unusually high")));
}

#[test]
fn validate_empty_notify_channel() {
    let yaml = r#"
rulebooks:
  - name: bad-notify
    events: [{type: manual}]
    actions:
      - notify:
          channel: ""
          message: "test"
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    assert!(issues
        .iter()
        .any(|i| i.message.contains("notify.channel is empty")));
}

// ============================================================================
// FJ-3108: validate_rulebook_config (direct struct)
// ============================================================================

#[test]
fn validate_config_empty_rulebooks() {
    let config = RulebookConfig { rulebooks: vec![] };
    let issues = validate_rulebook_config(&config);
    assert!(issues.is_empty());
}

// ============================================================================
// FJ-3108: ValidationSummary
// ============================================================================

#[test]
fn validation_summary_counts() {
    let issues = vec![
        RuleIssue {
            rulebook: "a".into(),
            severity: IssueSeverity::Error,
            message: "err".into(),
        },
        RuleIssue {
            rulebook: "b".into(),
            severity: IssueSeverity::Warning,
            message: "warn".into(),
        },
        RuleIssue {
            rulebook: "c".into(),
            severity: IssueSeverity::Error,
            message: "err2".into(),
        },
    ];
    let summary = ValidationSummary::new(3, issues);
    assert_eq!(summary.error_count(), 2);
    assert_eq!(summary.warning_count(), 1);
    assert!(!summary.passed());
}

#[test]
fn validation_summary_passed() {
    let summary = ValidationSummary::new(2, vec![]);
    assert!(summary.passed());
    assert_eq!(summary.error_count(), 0);
    assert_eq!(summary.warning_count(), 0);
}

#[test]
fn validation_summary_warnings_only_passes() {
    let issues = vec![RuleIssue {
        rulebook: "a".into(),
        severity: IssueSeverity::Warning,
        message: "warn".into(),
    }];
    let summary = ValidationSummary::new(1, issues);
    assert!(summary.passed()); // warnings don't block
    assert_eq!(summary.warning_count(), 1);
}

// ============================================================================
// FJ-3108: event_type_coverage
// ============================================================================

#[test]
fn event_type_coverage_counts() {
    let yaml = r#"
rulebooks:
  - name: r1
    events:
      - {type: file_changed}
      - {type: manual}
    actions: [{script: "echo 1"}]
  - name: r2
    events:
      - {type: file_changed}
    actions: [{script: "echo 2"}]
"#;
    let config: RulebookConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let coverage = event_type_coverage(&config);
    let fc = coverage
        .iter()
        .find(|(et, _)| *et == EventType::FileChanged);
    assert_eq!(fc.unwrap().1, 2);
    let m = coverage.iter().find(|(et, _)| *et == EventType::Manual);
    assert_eq!(m.unwrap().1, 1);
    let cr = coverage.iter().find(|(et, _)| *et == EventType::CronFired);
    assert_eq!(cr.unwrap().1, 0);
}

#[test]
fn event_type_coverage_empty() {
    let config = RulebookConfig { rulebooks: vec![] };
    let coverage = event_type_coverage(&config);
    assert_eq!(coverage.len(), 6); // all 6 event types
    assert!(coverage.iter().all(|(_, count)| *count == 0));
}

// ============================================================================
// FJ-3108: IssueSeverity Display
// ============================================================================

#[test]
fn issue_severity_display() {
    assert_eq!(IssueSeverity::Error.to_string(), "error");
    assert_eq!(IssueSeverity::Warning.to_string(), "warning");
}

// ============================================================================
// FJ-3108: validate_rulebook_file
// ============================================================================

#[test]
fn validate_file_not_found() {
    assert!(validate_rulebook_file(std::path::Path::new("/nonexistent/file.yaml")).is_err());
}

#[test]
fn validate_file_valid() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rules.yaml");
    std::fs::write(&path, valid_yaml()).unwrap();
    let issues = validate_rulebook_file(&path).unwrap();
    assert!(issues.is_empty());
}
