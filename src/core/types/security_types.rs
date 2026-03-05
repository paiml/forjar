//! FJ-2300: Security model types — secret management, path policy, authorization.
//!
//! Types for secret resolution, path deny lists, and operator authorization
//! as defined in the platform security model spec.

use serde::{Deserialize, Serialize};
use std::fmt;

/// FJ-2300: Secret provider backend.
///
/// # Examples
///
/// ```
/// use forjar::core::types::SecretProvider;
///
/// let provider = SecretProvider::Env;
/// assert_eq!(provider.to_string(), "env");
///
/// let yaml = "sops";
/// let parsed: SecretProvider = serde_yaml_ng::from_str(yaml).unwrap();
/// assert_eq!(parsed, SecretProvider::Sops);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecretProvider {
    /// Resolve from environment variables (`$FORJAR_SECRET_<name>`).
    #[default]
    Env,
    /// Resolve from files in a secrets directory.
    File,
    /// Resolve via `sops -d` decryption.
    Sops,
    /// Resolve via 1Password CLI (`op read`).
    Op,
}

impl fmt::Display for SecretProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Env => write!(f, "env"),
            Self::File => write!(f, "file"),
            Self::Sops => write!(f, "sops"),
            Self::Op => write!(f, "op"),
        }
    }
}

/// FJ-2300: Secret reference found in resource content.
///
/// Parsed from `{{ secrets.<name> }}` templates in resource content fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretRef {
    /// Secret name (the part after `secrets.`).
    pub name: String,
    /// Full template string (e.g., `{{ secrets.db_password }}`).
    pub template: String,
    /// Resource ID where the reference was found.
    pub resource_id: String,
    /// Field where the reference was found (e.g., "content", "command").
    pub field: String,
}

/// FJ-2300: Secret resolution configuration.
///
/// # Examples
///
/// ```
/// use forjar::core::types::{SecretConfig, SecretProvider};
///
/// let config = SecretConfig {
///     provider: SecretProvider::File,
///     path: Some("/run/secrets/".into()),
///     file: None,
/// };
/// assert_eq!(config.provider.to_string(), "file");
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecretConfig {
    /// Which backend to use for secret resolution.
    #[serde(default)]
    pub provider: SecretProvider,
    /// Path for file-based secret provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Encrypted file for SOPS provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

/// FJ-2300: Path deny policy for resource path restrictions.
///
/// # Examples
///
/// ```
/// use forjar::core::types::PathPolicy;
///
/// let policy = PathPolicy {
///     deny_paths: vec![
///         "/etc/shadow".into(),
///         "/etc/sudoers".into(),
///         "/root/.ssh/authorized_keys".into(),
///     ],
/// };
/// assert!(policy.is_denied("/etc/shadow"));
/// assert!(!policy.is_denied("/etc/nginx/nginx.conf"));
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PathPolicy {
    /// Glob patterns for denied paths.
    #[serde(default)]
    pub deny_paths: Vec<String>,
}

impl PathPolicy {
    /// Check if a path is denied by this policy.
    ///
    /// Supports exact matches and simple glob patterns with `*` suffix.
    pub fn is_denied(&self, path: &str) -> bool {
        for pattern in &self.deny_paths {
            if pattern == path {
                return true;
            }
            if let Some(prefix) = pattern.strip_suffix('*') {
                if path.starts_with(prefix) {
                    return true;
                }
            }
        }
        false
    }

    /// Check if any deny paths are configured.
    pub fn has_restrictions(&self) -> bool {
        !self.deny_paths.is_empty()
    }
}

/// FJ-2300: Authorization check result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthzResult {
    /// Operator is authorized (or no restriction configured).
    Allowed,
    /// Operator is not in the allowed list.
    Denied {
        operator: String,
        machine: String,
    },
}

impl AuthzResult {
    /// Whether the authorization check passed.
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }
}

impl fmt::Display for AuthzResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Allowed => write!(f, "allowed"),
            Self::Denied { operator, machine } => {
                write!(f, "operator '{operator}' not authorized for machine '{machine}'")
            }
        }
    }
}

/// FJ-2300: Secret scan result for hardcoded secret detection.
///
/// Used by `forjar validate --check-secrets` to find inline credentials.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretScanResult {
    /// Findings (potential hardcoded secrets).
    pub findings: Vec<SecretScanFinding>,
    /// Total resource fields scanned.
    pub scanned_fields: usize,
    /// Whether all fields are clean.
    pub clean: bool,
}

impl SecretScanResult {
    /// Build result from findings.
    pub fn from_findings(findings: Vec<SecretScanFinding>, scanned_fields: usize) -> Self {
        let clean = findings.is_empty();
        Self {
            findings,
            scanned_fields,
            clean,
        }
    }
}

/// FJ-2300: A single hardcoded secret finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretScanFinding {
    /// Resource ID where the potential secret was found.
    pub resource_id: String,
    /// Field name (e.g., "content").
    pub field: String,
    /// Pattern that matched (e.g., "password:", "api_key:").
    pub pattern: String,
    /// Redacted preview of the matched text.
    pub preview: String,
}

impl fmt::Display for SecretScanFinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}: potential hardcoded secret (pattern: {}, preview: {})",
            self.resource_id, self.field, self.pattern, self.preview
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_provider_serde_roundtrip() {
        for provider in [
            SecretProvider::Env,
            SecretProvider::File,
            SecretProvider::Sops,
            SecretProvider::Op,
        ] {
            let yaml = serde_yaml_ng::to_string(&provider).unwrap();
            let parsed: SecretProvider = serde_yaml_ng::from_str(&yaml).unwrap();
            assert_eq!(provider, parsed);
        }
    }

    #[test]
    fn secret_provider_default_is_env() {
        assert_eq!(SecretProvider::default(), SecretProvider::Env);
    }

    #[test]
    fn secret_provider_display() {
        assert_eq!(SecretProvider::Env.to_string(), "env");
        assert_eq!(SecretProvider::File.to_string(), "file");
        assert_eq!(SecretProvider::Sops.to_string(), "sops");
        assert_eq!(SecretProvider::Op.to_string(), "op");
    }

    #[test]
    fn secret_ref_serde() {
        let r = SecretRef {
            name: "db_password".into(),
            template: "{{ secrets.db_password }}".into(),
            resource_id: "db-config".into(),
            field: "content".into(),
        };
        let yaml = serde_yaml_ng::to_string(&r).unwrap();
        let parsed: SecretRef = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(parsed.name, "db_password");
        assert_eq!(parsed.resource_id, "db-config");
    }

    #[test]
    fn secret_config_defaults() {
        let config = SecretConfig::default();
        assert_eq!(config.provider, SecretProvider::Env);
        assert!(config.path.is_none());
        assert!(config.file.is_none());
    }

    #[test]
    fn secret_config_serde() {
        let yaml = r#"
provider: sops
file: secrets.enc.yaml
"#;
        let config: SecretConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.provider, SecretProvider::Sops);
        assert_eq!(config.file.as_deref(), Some("secrets.enc.yaml"));
    }

    #[test]
    fn path_policy_exact_match() {
        let policy = PathPolicy {
            deny_paths: vec!["/etc/shadow".into(), "/etc/sudoers".into()],
        };
        assert!(policy.is_denied("/etc/shadow"));
        assert!(policy.is_denied("/etc/sudoers"));
        assert!(!policy.is_denied("/etc/nginx.conf"));
    }

    #[test]
    fn path_policy_glob_match() {
        let policy = PathPolicy {
            deny_paths: vec!["/etc/sudoers.d/*".into()],
        };
        assert!(policy.is_denied("/etc/sudoers.d/custom"));
        assert!(policy.is_denied("/etc/sudoers.d/"));
        assert!(!policy.is_denied("/etc/sudoers"));
    }

    #[test]
    fn path_policy_empty() {
        let policy = PathPolicy::default();
        assert!(!policy.has_restrictions());
        assert!(!policy.is_denied("/etc/shadow"));
    }

    #[test]
    fn authz_result_allowed() {
        let result = AuthzResult::Allowed;
        assert!(result.is_allowed());
        assert_eq!(result.to_string(), "allowed");
    }

    #[test]
    fn authz_result_denied() {
        let result = AuthzResult::Denied {
            operator: "eve".into(),
            machine: "production-db".into(),
        };
        assert!(!result.is_allowed());
        assert!(result.to_string().contains("eve"));
        assert!(result.to_string().contains("production-db"));
    }

    #[test]
    fn secret_scan_result_clean() {
        let result = SecretScanResult::from_findings(vec![], 10);
        assert!(result.clean);
        assert_eq!(result.scanned_fields, 10);
    }

    #[test]
    fn secret_scan_result_with_findings() {
        let findings = vec![SecretScanFinding {
            resource_id: "db-config".into(),
            field: "content".into(),
            pattern: "password:".into(),
            preview: "password: s3cr***".into(),
        }];
        let result = SecretScanResult::from_findings(findings, 5);
        assert!(!result.clean);
        assert_eq!(result.findings.len(), 1);
    }

    #[test]
    fn secret_scan_finding_display() {
        let finding = SecretScanFinding {
            resource_id: "app".into(),
            field: "content".into(),
            pattern: "api_key:".into(),
            preview: "api_key: sk-***".into(),
        };
        let s = finding.to_string();
        assert!(s.contains("app.content"));
        assert!(s.contains("api_key:"));
    }
}
