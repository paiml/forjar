//! FJ-3404: Plugin type dispatch for `type: "plugin:NAME"` resources.
//!
//! When a resource has `type: "plugin:foo"`, this module resolves the
//! plugin manifest, verifies its BLAKE3 hash, validates the resource
//! schema, and dispatches to the plugin's check/apply/destroy methods.

use crate::core::plugin_loader::{resolve_and_verify, ResolvedPlugin};
use crate::core::plugin_runtime;
use crate::core::types::PluginStatus;
use std::path::Path;

/// Parse a resource type string to extract plugin name.
///
/// Returns `Some("name")` for `"plugin:name"`, `None` otherwise.
pub fn parse_plugin_type(resource_type: &str) -> Option<&str> {
    resource_type.strip_prefix("plugin:")
}

/// Result of dispatching a plugin operation.
#[derive(Debug, Clone)]
pub struct PluginDispatchResult {
    /// Plugin name.
    pub plugin_name: String,
    /// Operation performed.
    pub operation: String,
    /// Whether the operation succeeded.
    pub success: bool,
    /// Status message.
    pub message: String,
    /// Plugin status after operation.
    pub status: PluginStatus,
}

/// Resolve and verify a plugin for dispatch.
pub fn resolve_plugin(plugin_dir: &Path, plugin_name: &str) -> Result<ResolvedPlugin, String> {
    let resolved = resolve_and_verify(plugin_dir, plugin_name)?;
    if resolved.status != PluginStatus::Converged {
        return Err(format!(
            "plugin '{}' verification failed: {:?}",
            plugin_name, resolved.status
        ));
    }
    Ok(resolved)
}

/// Dispatch a check operation to a plugin.
///
/// Resolves the plugin, verifies BLAKE3 integrity, then executes the
/// `check` export via the WASM runtime (wasmi). Falls back to stub
/// when `wasm-runtime` feature is disabled.
pub fn dispatch_check(
    plugin_dir: &Path,
    plugin_name: &str,
    resource_config: &serde_json::Value,
) -> PluginDispatchResult {
    dispatch_operation(plugin_dir, plugin_name, "check", resource_config)
}

/// Dispatch an apply operation to a plugin.
pub fn dispatch_apply(
    plugin_dir: &Path,
    plugin_name: &str,
    resource_config: &serde_json::Value,
) -> PluginDispatchResult {
    dispatch_operation(plugin_dir, plugin_name, "apply", resource_config)
}

/// Dispatch a destroy operation to a plugin.
pub fn dispatch_destroy(
    plugin_dir: &Path,
    plugin_name: &str,
    resource_config: &serde_json::Value,
) -> PluginDispatchResult {
    dispatch_operation(plugin_dir, plugin_name, "destroy", resource_config)
}

/// Common dispatch logic for all plugin operations.
fn dispatch_operation(
    plugin_dir: &Path,
    plugin_name: &str,
    operation: &str,
    resource_config: &serde_json::Value,
) -> PluginDispatchResult {
    let resolved = match resolve_plugin(plugin_dir, plugin_name) {
        Ok(r) => r,
        Err(e) => {
            return PluginDispatchResult {
                plugin_name: plugin_name.into(),
                operation: operation.into(),
                success: false,
                message: e,
                status: PluginStatus::Error,
            };
        }
    };

    let input_json = serde_json::to_vec(resource_config).unwrap_or_default();
    match plugin_runtime::execute_wasm(&resolved.wasm_path, operation, &input_json) {
        Ok(result) => PluginDispatchResult {
            plugin_name: resolved.manifest.name,
            operation: operation.into(),
            success: result.success,
            message: if result.output.is_empty() {
                format!(
                    "plugin '{}' v{} {operation}",
                    plugin_name, resolved.manifest.version
                )
            } else {
                result.output
            },
            status: result.status,
        },
        Err(e) => PluginDispatchResult {
            plugin_name: resolved.manifest.name,
            operation: operation.into(),
            success: false,
            message: format!("wasm exec: {e}"),
            status: PluginStatus::Error,
        },
    }
}

/// Check if a resource type is a plugin type.
pub fn is_plugin_type(resource_type: &str) -> bool {
    resource_type.starts_with("plugin:")
}

/// List all plugin types available in a directory.
pub fn available_plugin_types(plugin_dir: &Path) -> Vec<String> {
    crate::core::plugin_loader::list_plugins(plugin_dir)
        .into_iter()
        .map(|name| format!("plugin:{name}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plugin_type_valid() {
        assert_eq!(parse_plugin_type("plugin:nginx"), Some("nginx"));
        assert_eq!(parse_plugin_type("plugin:my-custom"), Some("my-custom"));
    }

    #[test]
    fn parse_plugin_type_invalid() {
        assert_eq!(parse_plugin_type("package"), None);
        assert_eq!(parse_plugin_type("file"), None);
        assert_eq!(parse_plugin_type("plugin"), None);
    }

    #[test]
    fn is_plugin_type_check() {
        assert!(is_plugin_type("plugin:foo"));
        assert!(!is_plugin_type("package"));
        assert!(!is_plugin_type("file"));
    }

    #[test]
    fn available_plugin_types_empty() {
        let dir = tempfile::tempdir().unwrap();
        let types = available_plugin_types(dir.path());
        assert!(types.is_empty());
    }

    #[test]
    fn dispatch_check_missing_plugin() {
        let dir = tempfile::tempdir().unwrap();
        let config = serde_json::json!({"key": "value"});
        let result = dispatch_check(dir.path(), "nonexistent", &config);
        assert!(!result.success);
        assert_eq!(result.operation, "check");
    }

    #[test]
    fn dispatch_apply_missing_plugin() {
        let dir = tempfile::tempdir().unwrap();
        let config = serde_json::json!({});
        let result = dispatch_apply(dir.path(), "nonexistent", &config);
        assert!(!result.success);
        assert_eq!(result.operation, "apply");
    }

    #[test]
    fn dispatch_destroy_missing_plugin() {
        let dir = tempfile::tempdir().unwrap();
        let config = serde_json::json!({});
        let result = dispatch_destroy(dir.path(), "nonexistent", &config);
        assert!(!result.success);
        assert_eq!(result.operation, "destroy");
    }

    #[test]
    fn dispatch_check_with_real_plugin() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_dir = dir.path().join("test-plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();

        let wasm_bytes = b"fake wasm module content";
        let hash = blake3::hash(wasm_bytes).to_hex().to_string();
        std::fs::write(plugin_dir.join("plugin.wasm"), wasm_bytes).unwrap();
        std::fs::write(
            plugin_dir.join("plugin.yaml"),
            format!(
                r#"
name: test-plugin
version: "0.1.0"
abi_version: 1
wasm: plugin.wasm
blake3: {hash}
permissions:
  fs: {{}}
  net: {{}}
  env: {{}}
  exec: {{}}
"#
            ),
        )
        .unwrap();

        let config = serde_json::json!({"key": "value"});
        let result = dispatch_check(dir.path(), "test-plugin", &config);
        assert_eq!(result.operation, "check");
        if plugin_runtime::is_runtime_available() {
            // With real runtime, fake WASM bytes fail to compile
            assert!(!result.success);
            assert!(result.message.contains("wasm"));
        } else {
            assert!(result.success, "dispatch failed: {}", result.message);
        }
    }

    #[test]
    fn resolve_plugin_verified() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_dir = dir.path().join("verified");
        std::fs::create_dir_all(&plugin_dir).unwrap();

        let wasm_bytes = b"valid wasm bytes";
        let hash = blake3::hash(wasm_bytes).to_hex().to_string();
        std::fs::write(plugin_dir.join("plugin.wasm"), wasm_bytes).unwrap();
        std::fs::write(
            plugin_dir.join("plugin.yaml"),
            format!(
                r#"
name: verified
version: "1.0.0"
abi_version: 1
wasm: plugin.wasm
blake3: {hash}
permissions:
  fs: {{}}
  net: {{}}
  env: {{}}
  exec: {{}}
"#
            ),
        )
        .unwrap();

        let resolved = resolve_plugin(dir.path(), "verified");
        assert!(resolved.is_ok());
    }
}
