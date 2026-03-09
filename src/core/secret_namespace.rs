//! FJ-3306: Namespace isolation for ephemeral secret injection.
//!
//! Provides process-level isolation for secret values during apply.
//! Secrets are injected into a child process environment, used once,
//! then the process exits and the secret is gone from memory.
//!
//! Full pepita namespace integration uses Linux namespaces (unshare).
//! This module provides the orchestration layer: inject → execute → discard.

use crate::core::ephemeral::ResolvedEphemeral;
use crate::core::secret_audit;
use std::collections::HashMap;

/// Configuration for namespace-isolated secret execution.
#[derive(Debug, Clone)]
pub struct NamespaceConfig {
    /// Unique namespace ID (e.g., "ns-forjar-apply-<uuid>").
    pub namespace_id: String,
    /// Whether to log audit events.
    pub audit_enabled: bool,
    /// State directory for audit logs.
    pub state_dir: Option<std::path::PathBuf>,
    /// Environment variables to inherit (allowlist).
    pub inherit_env: Vec<String>,
}

impl Default for NamespaceConfig {
    fn default() -> Self {
        Self {
            namespace_id: format!("ns-forjar-{}", std::process::id()),
            audit_enabled: true,
            state_dir: None,
            inherit_env: vec!["PATH".into(), "HOME".into(), "USER".into(), "LANG".into()],
        }
    }
}

/// Result of a namespace-isolated execution.
#[derive(Debug, Clone)]
pub struct NamespaceResult {
    /// Namespace ID used.
    pub namespace_id: String,
    /// Whether the command succeeded.
    pub success: bool,
    /// Exit code.
    pub exit_code: Option<i32>,
    /// Standard output.
    pub stdout: String,
    /// Standard error.
    pub stderr: String,
    /// Number of secrets injected.
    pub secrets_injected: usize,
    /// Number of secrets discarded.
    pub secrets_discarded: usize,
}

/// Build an isolated environment from resolved ephemerals.
///
/// Creates a minimal environment with only allowlisted variables
/// plus the secret values. No parent environment leaks.
pub fn build_isolated_env(
    config: &NamespaceConfig,
    secrets: &[ResolvedEphemeral],
) -> HashMap<String, String> {
    let mut env = HashMap::new();

    // Inherit only allowlisted variables
    for key in &config.inherit_env {
        if let Ok(val) = std::env::var(key) {
            env.insert(key.clone(), val);
        }
    }

    // Inject secrets
    for secret in secrets {
        env.insert(secret.key.clone(), secret.value.clone());
    }

    // Add namespace marker
    env.insert("FORJAR_NAMESPACE".into(), config.namespace_id.clone());

    env
}

/// Execute a command in a namespace-isolated environment.
///
/// Secrets are injected into the child process environment only.
/// After execution, audit events are logged and secret references discarded.
pub fn execute_isolated(
    config: &NamespaceConfig,
    secrets: &[ResolvedEphemeral],
    command: &str,
    args: &[&str],
) -> Result<NamespaceResult, String> {
    let env = build_isolated_env(config, secrets);

    // Audit: log inject events
    if config.audit_enabled {
        if let Some(ref state_dir) = config.state_dir {
            for secret in secrets {
                let event = secret_audit::make_inject_event(
                    &secret.key,
                    "namespace",
                    &secret.hash,
                    &config.namespace_id,
                );
                let _ = secret_audit::append_audit(state_dir, &event);
            }
        }
    }

    // Execute command with isolated environment
    let output = std::process::Command::new(command)
        .args(args)
        .env_clear() // Critical: clear parent environment
        .envs(&env)
        .output()
        .map_err(|e| format!("execute in namespace {}: {e}", config.namespace_id))?;

    let secrets_injected = secrets.len();

    // Audit: log discard events
    if config.audit_enabled {
        if let Some(ref state_dir) = config.state_dir {
            for secret in secrets {
                let event = secret_audit::make_discard_event(&secret.key, &secret.hash);
                let _ = secret_audit::append_audit(state_dir, &event);
            }
        }
    }

    Ok(NamespaceResult {
        namespace_id: config.namespace_id.clone(),
        success: output.status.success(),
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        secrets_injected,
        secrets_discarded: secrets_injected,
    })
}

/// Verify that a secret is NOT visible in the current process environment.
///
/// After namespace teardown, confirm the secret didn't leak to the parent.
pub fn verify_no_leak(key: &str) -> bool {
    std::env::var(key).is_err()
}

/// Verify that /proc/<pid>/environ does NOT contain the secret key.
///
/// Linux-specific check for /proc leak prevention.
pub fn verify_no_proc_leak(pid: u32, key: &str) -> bool {
    let environ_path = format!("/proc/{pid}/environ");
    match std::fs::read_to_string(&environ_path) {
        Ok(content) => !content.contains(key),
        Err(_) => true, // Process gone = no leak
    }
}

/// Format namespace result for display.
pub fn format_result(result: &NamespaceResult) -> String {
    let status = if result.success { "SUCCESS" } else { "FAILED" };
    format!(
        "Namespace {}: {} (exit={}) secrets={}/{}",
        result.namespace_id,
        status,
        result.exit_code.map_or("-".into(), |c| c.to_string()),
        result.secrets_injected,
        result.secrets_discarded
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_secret(key: &str, value: &str) -> ResolvedEphemeral {
        ResolvedEphemeral {
            key: key.into(),
            value: value.into(),
            hash: blake3::hash(value.as_bytes()).to_hex().to_string(),
        }
    }

    #[test]
    fn build_env_includes_secrets() {
        let config = NamespaceConfig::default();
        let secrets = vec![make_secret("DB_PASS", "s3cret")];
        let env = build_isolated_env(&config, &secrets);
        assert_eq!(env.get("DB_PASS").unwrap(), "s3cret");
        assert!(env.contains_key("FORJAR_NAMESPACE"));
    }

    #[test]
    fn build_env_inherits_path() {
        let config = NamespaceConfig::default();
        let env = build_isolated_env(&config, &[]);
        // PATH should be inherited if set
        if std::env::var("PATH").is_ok() {
            assert!(env.contains_key("PATH"));
        }
    }

    #[test]
    fn build_env_no_extra_vars() {
        let config = NamespaceConfig {
            inherit_env: vec![], // inherit nothing
            ..Default::default()
        };
        let secrets = vec![make_secret("K", "V")];
        let env = build_isolated_env(&config, &secrets);
        // Only K and FORJAR_NAMESPACE
        assert_eq!(env.len(), 2);
        assert!(env.contains_key("K"));
        assert!(env.contains_key("FORJAR_NAMESPACE"));
    }

    #[test]
    fn execute_echo_secret() {
        let config = NamespaceConfig {
            audit_enabled: false,
            ..Default::default()
        };
        let secrets = vec![make_secret("MY_SECRET", "hidden-value")];
        let result = execute_isolated(&config, &secrets, "sh", &["-c", "echo $MY_SECRET"]).unwrap();
        assert!(result.success);
        assert_eq!(result.stdout.trim(), "hidden-value");
        assert_eq!(result.secrets_injected, 1);
        assert_eq!(result.secrets_discarded, 1);
    }

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
    fn execute_with_audit() {
        let dir = tempfile::tempdir().unwrap();
        let config = NamespaceConfig {
            audit_enabled: true,
            state_dir: Some(dir.path().to_path_buf()),
            ..Default::default()
        };
        let secrets = vec![make_secret("K", "V")];
        execute_isolated(&config, &secrets, "true", &[]).unwrap();

        // Should have inject + discard events
        let events = secret_audit::read_audit(dir.path()).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, secret_audit::SecretEventType::Inject);
        assert_eq!(events[1].event_type, secret_audit::SecretEventType::Discard);
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
    fn verify_no_leak_current_env() {
        // A random key should not be in our environment
        assert!(verify_no_leak("FORJAR_TEST_NONEXISTENT_KEY_12345"));
    }

    #[test]
    fn verify_no_proc_leak_nonexistent_pid() {
        // PID 1 environ is often readable, but a high PID won't exist
        assert!(verify_no_proc_leak(999_999_999, "SECRET_KEY"));
    }

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
            namespace_id: "ns-test-2".into(),
            success: false,
            exit_code: Some(1),
            stdout: String::new(),
            stderr: "error".into(),
            secrets_injected: 1,
            secrets_discarded: 1,
        };
        let text = format_result(&result);
        assert!(text.contains("FAILED"));
    }

    #[test]
    fn default_namespace_config() {
        let config = NamespaceConfig::default();
        assert!(config.namespace_id.starts_with("ns-forjar-"));
        assert!(config.audit_enabled);
        assert!(config.state_dir.is_none());
        assert!(config.inherit_env.contains(&"PATH".to_string()));
    }

    #[test]
    fn multiple_secrets_injected() {
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
    }

    #[test]
    fn namespace_id_in_child_env() {
        let config = NamespaceConfig {
            namespace_id: "ns-test-custom".into(),
            audit_enabled: false,
            ..Default::default()
        };
        let result =
            execute_isolated(&config, &[], "sh", &["-c", "echo $FORJAR_NAMESPACE"]).unwrap();
        assert_eq!(result.stdout.trim(), "ns-test-custom");
    }
}
