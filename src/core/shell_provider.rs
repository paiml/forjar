//! FJ-3405: Shell provider bridge for resource plugins.
//!
//! Allows shell scripts to act as resource providers. Scripts define
//! check/apply/destroy functions. All scripts are bashrs-validated
//! before execution and scanned for secret leakage.
//!
//! Shell providers are the on-ramp; WASM providers are the destination.

use crate::core::purifier;
use crate::core::script_secret_lint;
use crate::core::types::PluginStatus;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Shell provider manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellProviderManifest {
    /// Provider name (used in `type: "shell:NAME"`).
    pub name: String,
    /// Provider version.
    pub version: String,
    /// Description.
    #[serde(default)]
    pub description: Option<String>,
    /// Path to the check script (relative to provider dir).
    pub check: String,
    /// Path to the apply script (relative to provider dir).
    pub apply: String,
    /// Path to the destroy script (relative to provider dir).
    pub destroy: String,
}

/// Result of a shell provider operation.
#[derive(Debug, Clone)]
pub struct ShellProviderResult {
    /// Provider name.
    pub name: String,
    /// Operation (check/apply/destroy).
    pub operation: String,
    /// Status after operation.
    pub status: PluginStatus,
    /// Whether the script passed validation.
    pub validated: bool,
    /// Validation errors (if any).
    pub errors: Vec<String>,
}

/// Parse a resource type to extract shell provider name.
///
/// Returns `Some("name")` for `"shell:name"`, `None` otherwise.
pub fn parse_shell_type(resource_type: &str) -> Option<&str> {
    resource_type.strip_prefix("shell:")
}

/// Check if a resource type is a shell provider type.
pub fn is_shell_type(resource_type: &str) -> bool {
    resource_type.starts_with("shell:")
}

/// Load and validate a shell provider manifest.
pub fn load_manifest(provider_dir: &Path) -> Result<ShellProviderManifest, String> {
    let manifest_path = provider_dir.join("provider.yaml");
    let content = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("read {}: {e}", manifest_path.display()))?;
    let manifest: ShellProviderManifest =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("parse manifest: {e}"))?;
    Ok(manifest)
}

/// Validate a shell provider script via bashrs + secret leakage scan.
///
/// Returns Ok(()) if script passes all safety checks.
pub fn validate_provider_script(script: &str) -> Result<(), String> {
    // bashrs lint validation
    purifier::validate_script(script)?;

    // Secret leakage detection (FJ-3307)
    script_secret_lint::validate_no_leaks(script)?;

    Ok(())
}

/// Validate all scripts in a shell provider.
pub fn validate_provider(provider_dir: &Path) -> ShellProviderResult {
    let manifest = match load_manifest(provider_dir) {
        Ok(m) => m,
        Err(e) => {
            return ShellProviderResult {
                name: provider_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
                operation: "validate".into(),
                status: PluginStatus::Error,
                validated: false,
                errors: vec![e],
            };
        }
    };

    let mut errors = Vec::new();

    for (label, script_path) in [
        ("check", &manifest.check),
        ("apply", &manifest.apply),
        ("destroy", &manifest.destroy),
    ] {
        let full_path = provider_dir.join(script_path);
        match std::fs::read_to_string(&full_path) {
            Ok(content) => {
                if let Err(e) = validate_provider_script(&content) {
                    errors.push(format!("{label} ({script_path}): {e}"));
                }
            }
            Err(e) => {
                errors.push(format!("{label} ({script_path}): read error: {e}"));
            }
        }
    }

    ShellProviderResult {
        name: manifest.name,
        operation: "validate".into(),
        status: if errors.is_empty() {
            PluginStatus::Converged
        } else {
            PluginStatus::Error
        },
        validated: errors.is_empty(),
        errors,
    }
}

/// List shell providers in a directory.
pub fn list_shell_providers(provider_dir: &Path) -> Vec<String> {
    let mut providers = Vec::new();
    if let Ok(entries) = std::fs::read_dir(provider_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("provider.yaml").exists() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    providers.push(name.to_string());
                }
            }
        }
    }
    providers.sort();
    providers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_shell_type_valid() {
        assert_eq!(parse_shell_type("shell:nginx"), Some("nginx"));
        assert_eq!(parse_shell_type("shell:my-provider"), Some("my-provider"));
    }

    #[test]
    fn parse_shell_type_invalid() {
        assert_eq!(parse_shell_type("plugin:foo"), None);
        assert_eq!(parse_shell_type("file"), None);
    }

    #[test]
    fn is_shell_type_check() {
        assert!(is_shell_type("shell:nginx"));
        assert!(!is_shell_type("plugin:nginx"));
        assert!(!is_shell_type("package"));
    }

    #[test]
    fn validate_clean_script() {
        let script = "#!/bin/bash\nset -euo pipefail\necho 'checking resource'\n";
        assert!(validate_provider_script(script).is_ok());
    }

    #[test]
    fn validate_script_with_secret_leak() {
        let script = "#!/bin/bash\necho $PASSWORD\n";
        let result = validate_provider_script(script);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("secret leakage"));
    }

    #[test]
    fn load_manifest_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = r#"
name: test-provider
version: "0.1.0"
description: "A test shell provider"
check: check.sh
apply: apply.sh
destroy: destroy.sh
"#;
        std::fs::write(dir.path().join("provider.yaml"), manifest).unwrap();
        let loaded = load_manifest(dir.path()).unwrap();
        assert_eq!(loaded.name, "test-provider");
        assert_eq!(loaded.version, "0.1.0");
        assert_eq!(loaded.check, "check.sh");
    }

    #[test]
    fn load_manifest_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_manifest(dir.path()).is_err());
    }

    #[test]
    fn validate_provider_all_scripts() {
        let dir = tempfile::tempdir().unwrap();
        let pdir = dir.path().join("my-provider");
        std::fs::create_dir_all(&pdir).unwrap();
        std::fs::write(
            pdir.join("provider.yaml"),
            "name: my-provider\nversion: \"0.1.0\"\ncheck: check.sh\napply: apply.sh\ndestroy: destroy.sh\n",
        )
        .unwrap();
        std::fs::write(pdir.join("check.sh"), "#!/bin/bash\nexit 0\n").unwrap();
        std::fs::write(pdir.join("apply.sh"), "#!/bin/bash\nexit 0\n").unwrap();
        std::fs::write(pdir.join("destroy.sh"), "#!/bin/bash\nexit 0\n").unwrap();

        let result = validate_provider(&pdir);
        assert!(result.validated, "errors: {:?}", result.errors);
        assert_eq!(result.status, PluginStatus::Converged);
    }

    #[test]
    fn validate_provider_with_leak() {
        let dir = tempfile::tempdir().unwrap();
        let pdir = dir.path().join("leaky");
        std::fs::create_dir_all(&pdir).unwrap();
        std::fs::write(
            pdir.join("provider.yaml"),
            "name: leaky\nversion: \"0.1.0\"\ncheck: check.sh\napply: apply.sh\ndestroy: destroy.sh\n",
        )
        .unwrap();
        std::fs::write(pdir.join("check.sh"), "#!/bin/bash\nexit 0\n").unwrap();
        std::fs::write(pdir.join("apply.sh"), "#!/bin/bash\necho $PASSWORD\n").unwrap();
        std::fs::write(pdir.join("destroy.sh"), "#!/bin/bash\nexit 0\n").unwrap();

        let result = validate_provider(&pdir);
        assert!(!result.validated);
        assert_eq!(result.status, PluginStatus::Error);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn list_shell_providers_empty() {
        let dir = tempfile::tempdir().unwrap();
        let providers = list_shell_providers(dir.path());
        assert!(providers.is_empty());
    }

    #[test]
    fn list_shell_providers_found() {
        let dir = tempfile::tempdir().unwrap();
        let p1 = dir.path().join("nginx");
        std::fs::create_dir_all(&p1).unwrap();
        std::fs::write(
            p1.join("provider.yaml"),
            "name: nginx\nversion: \"1\"\ncheck: c.sh\napply: a.sh\ndestroy: d.sh\n",
        )
        .unwrap();

        let p2 = dir.path().join("postgres");
        std::fs::create_dir_all(&p2).unwrap();
        std::fs::write(
            p2.join("provider.yaml"),
            "name: postgres\nversion: \"1\"\ncheck: c.sh\napply: a.sh\ndestroy: d.sh\n",
        )
        .unwrap();

        // This dir has no provider.yaml — should be skipped
        std::fs::create_dir_all(dir.path().join("not-a-provider")).unwrap();

        let providers = list_shell_providers(dir.path());
        assert_eq!(providers, vec!["nginx", "postgres"]);
    }
}
