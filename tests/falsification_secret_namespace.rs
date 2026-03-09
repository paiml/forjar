//! FJ-3306: Namespace isolation for ephemeral secret injection falsification.
//!
//! Popperian rejection criteria for:
//! - build_isolated_env: allowlisted inheritance, secret injection, namespace marker
//! - execute_isolated: child process receives secrets, parent doesn't
//! - env_clear: parent environment fully cleared in child
//! - Audit trail: inject + discard events logged during execution
//! - verify_no_leak: secret not visible in current process after execution
//! - verify_no_proc_leak: /proc/<pid>/environ check for nonexistent process
//! - Multiple secrets injected simultaneously
//! - NamespaceConfig defaults and custom configuration
//! - NamespaceResult fields populated correctly
//! - format_result output for success and failure
//!
//! Usage: cargo test --test falsification_secret_namespace

use forjar::core::ephemeral::ResolvedEphemeral;
use forjar::core::secret_audit::{read_audit, SecretEventType};
use forjar::core::secret_namespace::{
    build_isolated_env, execute_isolated, format_result, verify_no_leak, verify_no_proc_leak,
    NamespaceConfig, NamespaceResult,
};

// ============================================================================
// Helpers
// ============================================================================

fn make_secret(key: &str, value: &str) -> ResolvedEphemeral {
    ResolvedEphemeral {
        key: key.into(),
        value: value.into(),
        hash: blake3::hash(value.as_bytes()).to_hex().to_string(),
    }
}

// ============================================================================
// FJ-3306: NamespaceConfig Defaults
// ============================================================================

#[test]
fn default_config_namespace_id_prefix() {
    let config = NamespaceConfig::default();
    assert!(config.namespace_id.starts_with("ns-forjar-"));
}

#[test]
fn default_config_audit_enabled() {
    let config = NamespaceConfig::default();
    assert!(config.audit_enabled);
}

#[test]
fn default_config_no_state_dir() {
    let config = NamespaceConfig::default();
    assert!(config.state_dir.is_none());
}

#[test]
fn default_config_inherits_standard_vars() {
    let config = NamespaceConfig::default();
    assert!(config.inherit_env.contains(&"PATH".to_string()));
    assert!(config.inherit_env.contains(&"HOME".to_string()));
    assert!(config.inherit_env.contains(&"USER".to_string()));
    assert!(config.inherit_env.contains(&"LANG".to_string()));
}

// ============================================================================
// FJ-3306: build_isolated_env
// ============================================================================

#[test]
fn build_env_injects_secrets() {
    let config = NamespaceConfig::default();
    let secrets = vec![
        make_secret("DB_PASS", "s3cret"),
        make_secret("API_KEY", "abc123"),
    ];
    let env = build_isolated_env(&config, &secrets);
    assert_eq!(env.get("DB_PASS").unwrap(), "s3cret");
    assert_eq!(env.get("API_KEY").unwrap(), "abc123");
}

#[test]
fn build_env_includes_namespace_marker() {
    let config = NamespaceConfig {
        namespace_id: "ns-test-42".into(),
        ..Default::default()
    };
    let env = build_isolated_env(&config, &[]);
    assert_eq!(env.get("FORJAR_NAMESPACE").unwrap(), "ns-test-42");
}

#[test]
fn build_env_inherits_path_if_set() {
    let config = NamespaceConfig::default();
    let env = build_isolated_env(&config, &[]);
    if std::env::var("PATH").is_ok() {
        assert!(env.contains_key("PATH"));
    }
}

#[test]
fn build_env_no_inheritance_minimal() {
    let config = NamespaceConfig {
        inherit_env: vec![], // nothing inherited
        ..Default::default()
    };
    let secrets = vec![make_secret("ONLY_KEY", "val")];
    let env = build_isolated_env(&config, &secrets);
    // Only ONLY_KEY + FORJAR_NAMESPACE
    assert_eq!(env.len(), 2);
    assert!(env.contains_key("ONLY_KEY"));
    assert!(env.contains_key("FORJAR_NAMESPACE"));
}

#[test]
fn build_env_empty_secrets() {
    let config = NamespaceConfig {
        inherit_env: vec![],
        ..Default::default()
    };
    let env = build_isolated_env(&config, &[]);
    // Only FORJAR_NAMESPACE
    assert_eq!(env.len(), 1);
    assert!(env.contains_key("FORJAR_NAMESPACE"));
}

#[test]
fn build_env_secret_overrides_inherited_key() {
    // If a secret has the same name as an inherited var, secret wins
    let config = NamespaceConfig {
        inherit_env: vec!["PATH".into()],
        ..Default::default()
    };
    let secrets = vec![make_secret("PATH", "/custom/bin")];
    let env = build_isolated_env(&config, &secrets);
    assert_eq!(env.get("PATH").unwrap(), "/custom/bin");
}

// ============================================================================
// FJ-3306: execute_isolated — Secret in Child
// ============================================================================

#[test]
fn execute_echo_secret_value() {
    let config = NamespaceConfig {
        audit_enabled: false,
        ..Default::default()
    };
    let secrets = vec![make_secret("MY_SECRET", "hidden-value")];
    let result = execute_isolated(&config, &secrets, "sh", &["-c", "echo $MY_SECRET"]).unwrap();
    assert!(result.success);
    assert_eq!(result.stdout.trim(), "hidden-value");
}

#[test]
fn execute_multiple_secrets() {
    let config = NamespaceConfig {
        audit_enabled: false,
        ..Default::default()
    };
    let secrets = vec![
        make_secret("A", "val_a"),
        make_secret("B", "val_b"),
        make_secret("C", "val_c"),
    ];
    let result = execute_isolated(&config, &secrets, "sh", &["-c", "echo $A $B $C"]).unwrap();
    assert!(result.success);
    assert_eq!(result.stdout.trim(), "val_a val_b val_c");
    assert_eq!(result.secrets_injected, 3);
    assert_eq!(result.secrets_discarded, 3);
}

#[test]
fn execute_namespace_id_visible_in_child() {
    let config = NamespaceConfig {
        namespace_id: "ns-custom-id".into(),
        audit_enabled: false,
        ..Default::default()
    };
    let result = execute_isolated(&config, &[], "sh", &["-c", "echo $FORJAR_NAMESPACE"]).unwrap();
    assert_eq!(result.stdout.trim(), "ns-custom-id");
}

// ============================================================================
// FJ-3306: execute_isolated — Parent Env Isolation
// ============================================================================

#[test]
fn execute_no_parent_env_leak() {
    let config = NamespaceConfig {
        inherit_env: vec![], // No inheritance
        audit_enabled: false,
        ..Default::default()
    };
    // HOME should not be visible in child
    let result = execute_isolated(&config, &[], "sh", &["-c", "echo ${HOME:-UNSET}"]).unwrap();
    assert_eq!(result.stdout.trim(), "UNSET");
}

#[test]
fn execute_inherited_vars_visible() {
    let config = NamespaceConfig {
        inherit_env: vec!["PATH".into()],
        audit_enabled: false,
        ..Default::default()
    };
    // PATH should be visible since we inherited it
    let result = execute_isolated(&config, &[], "sh", &["-c", "echo ${PATH:-UNSET}"]).unwrap();
    assert_ne!(result.stdout.trim(), "UNSET");
}

// ============================================================================
// FJ-3306: execute_isolated — Audit Trail
// ============================================================================

#[test]
fn execute_with_audit_logs_inject_and_discard() {
    let dir = tempfile::tempdir().unwrap();
    let config = NamespaceConfig {
        audit_enabled: true,
        state_dir: Some(dir.path().to_path_buf()),
        ..Default::default()
    };
    let secrets = vec![make_secret("K", "V")];
    execute_isolated(&config, &secrets, "true", &[]).unwrap();

    let events = read_audit(dir.path()).unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, SecretEventType::Inject);
    assert_eq!(events[1].event_type, SecretEventType::Discard);
}

#[test]
fn execute_audit_disabled_no_events() {
    let dir = tempfile::tempdir().unwrap();
    let config = NamespaceConfig {
        audit_enabled: false,
        state_dir: Some(dir.path().to_path_buf()),
        ..Default::default()
    };
    let secrets = vec![make_secret("K", "V")];
    execute_isolated(&config, &secrets, "true", &[]).unwrap();

    let events = read_audit(dir.path()).unwrap();
    assert!(events.is_empty());
}

#[test]
fn execute_audit_multiple_secrets_logged() {
    let dir = tempfile::tempdir().unwrap();
    let config = NamespaceConfig {
        audit_enabled: true,
        state_dir: Some(dir.path().to_path_buf()),
        ..Default::default()
    };
    let secrets = vec![make_secret("K1", "V1"), make_secret("K2", "V2")];
    execute_isolated(&config, &secrets, "true", &[]).unwrap();

    let events = read_audit(dir.path()).unwrap();
    // 2 inject + 2 discard = 4
    assert_eq!(events.len(), 4);
    let inject_count = events
        .iter()
        .filter(|e| e.event_type == SecretEventType::Inject)
        .count();
    let discard_count = events
        .iter()
        .filter(|e| e.event_type == SecretEventType::Discard)
        .count();
    assert_eq!(inject_count, 2);
    assert_eq!(discard_count, 2);
}

#[test]
fn execute_audit_inject_has_namespace_id() {
    let dir = tempfile::tempdir().unwrap();
    let config = NamespaceConfig {
        namespace_id: "ns-audit-test".into(),
        audit_enabled: true,
        state_dir: Some(dir.path().to_path_buf()),
        ..Default::default()
    };
    let secrets = vec![make_secret("K", "V")];
    execute_isolated(&config, &secrets, "true", &[]).unwrap();

    let events = read_audit(dir.path()).unwrap();
    let inject = events
        .iter()
        .find(|e| e.event_type == SecretEventType::Inject)
        .unwrap();
    assert_eq!(inject.namespace.as_deref(), Some("ns-audit-test"));
}

// ============================================================================
// FJ-3306: execute_isolated — Failure Handling
// ============================================================================

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
fn execute_nonexistent_command_errors() {
    let config = NamespaceConfig {
        audit_enabled: false,
        ..Default::default()
    };
    let result = execute_isolated(&config, &[], "nonexistent-cmd-99999", &[]);
    assert!(result.is_err());
}

// ============================================================================
// FJ-3306: NamespaceResult Fields
// ============================================================================

#[test]
fn result_fields_on_success() {
    let config = NamespaceConfig {
        namespace_id: "ns-fields-test".into(),
        audit_enabled: false,
        ..Default::default()
    };
    let secrets = vec![make_secret("K", "V")];
    let result = execute_isolated(&config, &secrets, "true", &[]).unwrap();
    assert_eq!(result.namespace_id, "ns-fields-test");
    assert!(result.success);
    assert_eq!(result.exit_code, Some(0));
    assert_eq!(result.secrets_injected, 1);
    assert_eq!(result.secrets_discarded, 1);
}

#[test]
fn result_stdout_captured() {
    let config = NamespaceConfig {
        audit_enabled: false,
        ..Default::default()
    };
    let result = execute_isolated(&config, &[], "echo", &["hello world"]).unwrap();
    assert!(result.stdout.contains("hello world"));
}

#[test]
fn result_stderr_captured() {
    let config = NamespaceConfig {
        audit_enabled: false,
        ..Default::default()
    };
    let result = execute_isolated(&config, &[], "sh", &["-c", "echo 'err msg' >&2"]).unwrap();
    assert!(result.stderr.contains("err msg"));
}

// ============================================================================
// FJ-3306: verify_no_leak
// ============================================================================

#[test]
fn verify_no_leak_random_key() {
    assert!(verify_no_leak("FORJAR_TEST_NONEXISTENT_KEY_XYZ"));
}

#[test]
fn verify_no_leak_after_execute() {
    let config = NamespaceConfig {
        audit_enabled: false,
        ..Default::default()
    };
    let secrets = vec![make_secret("FORJAR_TEMP_SECRET_7777", "ephemeral")];
    execute_isolated(&config, &secrets, "true", &[]).unwrap();
    // Secret should NOT be visible in parent process
    assert!(verify_no_leak("FORJAR_TEMP_SECRET_7777"));
}

// ============================================================================
// FJ-3306: verify_no_proc_leak
// ============================================================================

#[test]
fn verify_no_proc_leak_nonexistent_pid() {
    // High PID won't exist → no leak
    assert!(verify_no_proc_leak(999_999_999, "SECRET_KEY"));
}

// ============================================================================
// FJ-3306: format_result
// ============================================================================

#[test]
fn format_result_success() {
    let result = NamespaceResult {
        namespace_id: "ns-fmt-1".into(),
        success: true,
        exit_code: Some(0),
        stdout: String::new(),
        stderr: String::new(),
        secrets_injected: 3,
        secrets_discarded: 3,
    };
    let text = format_result(&result);
    assert!(text.contains("ns-fmt-1"));
    assert!(text.contains("SUCCESS"));
    assert!(text.contains("3/3"));
}

#[test]
fn format_result_failure() {
    let result = NamespaceResult {
        namespace_id: "ns-fmt-2".into(),
        success: false,
        exit_code: Some(1),
        stdout: String::new(),
        stderr: String::new(),
        secrets_injected: 1,
        secrets_discarded: 1,
    };
    let text = format_result(&result);
    assert!(text.contains("FAILED"));
    assert!(text.contains("exit=1"));
}

#[test]
fn format_result_no_exit_code() {
    let result = NamespaceResult {
        namespace_id: "ns-fmt-3".into(),
        success: false,
        exit_code: None,
        stdout: String::new(),
        stderr: String::new(),
        secrets_injected: 0,
        secrets_discarded: 0,
    };
    let text = format_result(&result);
    assert!(text.contains("exit=-"));
}

// ============================================================================
// FJ-3306: End-to-End Integration
// ============================================================================

#[test]
fn full_namespace_lifecycle() {
    let dir = tempfile::tempdir().unwrap();
    let config = NamespaceConfig {
        namespace_id: "ns-e2e-test".into(),
        audit_enabled: true,
        state_dir: Some(dir.path().to_path_buf()),
        inherit_env: vec!["PATH".into()],
    };
    let secrets = vec![
        make_secret("DB_PASS", "s3cret"),
        make_secret("API_KEY", "abc123"),
    ];

    // Execute with secrets
    let result = execute_isolated(
        &config,
        &secrets,
        "sh",
        &["-c", "echo $DB_PASS:$API_KEY:$FORJAR_NAMESPACE"],
    )
    .unwrap();

    assert!(result.success);
    assert_eq!(result.stdout.trim(), "s3cret:abc123:ns-e2e-test");
    assert_eq!(result.secrets_injected, 2);
    assert_eq!(result.secrets_discarded, 2);

    // Verify audit trail
    let events = read_audit(dir.path()).unwrap();
    assert_eq!(events.len(), 4); // 2 inject + 2 discard

    // Verify no leak
    assert!(verify_no_leak("DB_PASS"));
    assert!(verify_no_leak("API_KEY"));
}
