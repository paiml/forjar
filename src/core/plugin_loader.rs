//! FJ-3404: Plugin resolver for `type: "plugin:<name>"` resources.
//!
//! Resolves plugin manifests from the plugins directory, verifies BLAKE3
//! integrity, checks ABI compatibility, and validates resource schemas.

use crate::core::types::{PluginManifest, PluginStatus};
use std::path::{Path, PathBuf};

/// Default plugin directory relative to the config file.
pub const PLUGIN_DIR: &str = "plugins";

/// Resolve a plugin manifest from a resource type like `plugin:k8s-deployment`.
///
/// Looks for `<plugin_dir>/<name>/plugin.yaml` and parses the manifest.
pub fn resolve_manifest(plugin_dir: &Path, plugin_name: &str) -> Result<PluginManifest, String> {
    let manifest_path = plugin_dir.join(plugin_name).join("plugin.yaml");
    if !manifest_path.exists() {
        return Err(format!(
            "plugin manifest not found: {}",
            manifest_path.display()
        ));
    }
    let content = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("read {}: {e}", manifest_path.display()))?;
    let manifest: PluginManifest = serde_yaml_ng::from_str(&content)
        .map_err(|e| format!("parse {}: {e}", manifest_path.display()))?;

    if manifest.name != plugin_name {
        return Err(format!(
            "plugin name mismatch: manifest says '{}', expected '{plugin_name}'",
            manifest.name
        ));
    }

    Ok(manifest)
}

/// Verify a plugin's WASM module integrity against its BLAKE3 hash.
pub fn verify_plugin(plugin_dir: &Path, manifest: &PluginManifest) -> Result<(), String> {
    if !manifest.is_abi_compatible() {
        return Err(format!(
            "plugin '{}' uses ABI v{}, host supports v{}",
            manifest.name,
            manifest.abi_version,
            crate::core::types::PLUGIN_ABI_VERSION
        ));
    }

    let wasm_path = plugin_dir.join(&manifest.name).join(&manifest.wasm);
    if !wasm_path.exists() {
        return Err(format!("WASM module not found: {}", wasm_path.display()));
    }

    let bytes =
        std::fs::read(&wasm_path).map_err(|e| format!("read {}: {e}", wasm_path.display()))?;

    if !manifest.verify_hash(&bytes) {
        return Err(format!(
            "BLAKE3 hash mismatch for '{}': expected {}, got {}",
            manifest.name,
            manifest.blake3,
            blake3::hash(&bytes).to_hex()
        ));
    }

    Ok(())
}

/// Validate resource properties against a plugin's schema.
pub fn validate_resource_schema(
    manifest: &PluginManifest,
    properties: &indexmap::IndexMap<String, serde_yaml_ng::Value>,
) -> Vec<String> {
    match &manifest.schema {
        Some(schema) => schema.validate(properties),
        None => Vec::new(),
    }
}

/// Extract the plugin name from a resource type like `plugin:k8s-deployment`.
pub fn parse_plugin_type(resource_type: &str) -> Option<&str> {
    resource_type.strip_prefix("plugin:")
}

/// Result of resolving and verifying a plugin.
#[derive(Debug)]
pub struct ResolvedPlugin {
    /// Parsed manifest.
    pub manifest: PluginManifest,
    /// Path to the WASM module.
    pub wasm_path: PathBuf,
    /// Plugin status after verification.
    pub status: PluginStatus,
}

/// Full resolve + verify pipeline for a plugin resource.
pub fn resolve_and_verify(plugin_dir: &Path, plugin_name: &str) -> Result<ResolvedPlugin, String> {
    let manifest = resolve_manifest(plugin_dir, plugin_name)?;
    verify_plugin(plugin_dir, &manifest)?;
    let wasm_path = plugin_dir.join(plugin_name).join(&manifest.wasm);
    Ok(ResolvedPlugin {
        manifest,
        wasm_path,
        status: PluginStatus::Converged,
    })
}

/// List all available plugins in the plugin directory.
pub fn list_plugins(plugin_dir: &Path) -> Vec<String> {
    let mut plugins = Vec::new();
    if let Ok(entries) = std::fs::read_dir(plugin_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("plugin.yaml").exists() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    plugins.push(name.to_string());
                }
            }
        }
    }
    plugins.sort();
    plugins
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_plugin(dir: &Path, name: &str, wasm_content: &[u8]) -> PluginManifest {
        let plugin_dir = dir.join(name);
        std::fs::create_dir_all(&plugin_dir).unwrap();

        let hash = blake3::hash(wasm_content).to_hex().to_string();
        let manifest = format!(
            r#"
name: {name}
version: "0.1.0"
description: "Test plugin"
abi_version: 1
wasm: {name}.wasm
blake3: "{hash}"
"#
        );
        std::fs::write(plugin_dir.join("plugin.yaml"), manifest).unwrap();
        std::fs::write(plugin_dir.join(format!("{name}.wasm")), wasm_content).unwrap();

        resolve_manifest(dir, name).unwrap()
    }

    #[test]
    fn resolve_valid_manifest() {
        let dir = TempDir::new().unwrap();
        let m = setup_plugin(dir.path(), "test-plugin", b"wasm data");
        assert_eq!(m.name, "test-plugin");
        assert_eq!(m.abi_version, 1);
    }

    #[test]
    fn resolve_missing_manifest() {
        let dir = TempDir::new().unwrap();
        let err = resolve_manifest(dir.path(), "nonexistent").unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn resolve_name_mismatch() {
        let dir = TempDir::new().unwrap();
        let plugin_dir = dir.path().join("wrong-name");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::write(
            plugin_dir.join("plugin.yaml"),
            r#"
name: actual-name
version: "0.1.0"
abi_version: 1
wasm: test.wasm
blake3: "abc"
"#,
        )
        .unwrap();
        let err = resolve_manifest(dir.path(), "wrong-name").unwrap_err();
        assert!(err.contains("mismatch"));
    }

    #[test]
    fn verify_valid_plugin() {
        let dir = TempDir::new().unwrap();
        let wasm = b"valid wasm module bytes";
        let m = setup_plugin(dir.path(), "valid", wasm);
        assert!(verify_plugin(dir.path(), &m).is_ok());
    }

    #[test]
    fn verify_tampered_plugin() {
        let dir = TempDir::new().unwrap();
        let wasm = b"original bytes";
        let m = setup_plugin(dir.path(), "tampered", wasm);

        // Tamper with the WASM file
        std::fs::write(
            dir.path().join("tampered").join("tampered.wasm"),
            b"modified bytes",
        )
        .unwrap();

        let err = verify_plugin(dir.path(), &m).unwrap_err();
        assert!(err.contains("hash mismatch"));
    }

    #[test]
    fn verify_missing_wasm() {
        let dir = TempDir::new().unwrap();
        let plugin_dir = dir.path().join("no-wasm");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::write(
            plugin_dir.join("plugin.yaml"),
            r#"
name: no-wasm
version: "0.1.0"
abi_version: 1
wasm: missing.wasm
blake3: "abc"
"#,
        )
        .unwrap();
        let m = resolve_manifest(dir.path(), "no-wasm").unwrap();
        let err = verify_plugin(dir.path(), &m).unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn verify_incompatible_abi() {
        let dir = TempDir::new().unwrap();
        let plugin_dir = dir.path().join("bad-abi");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::write(
            plugin_dir.join("plugin.yaml"),
            r#"
name: bad-abi
version: "0.1.0"
abi_version: 99
wasm: test.wasm
blake3: "abc"
"#,
        )
        .unwrap();
        std::fs::write(plugin_dir.join("test.wasm"), b"data").unwrap();

        let m = resolve_manifest(dir.path(), "bad-abi").unwrap();
        let err = verify_plugin(dir.path(), &m).unwrap_err();
        assert!(err.contains("ABI"));
    }

    #[test]
    fn parse_plugin_type_valid() {
        assert_eq!(parse_plugin_type("plugin:k8s"), Some("k8s"));
        assert_eq!(parse_plugin_type("plugin:my-plugin"), Some("my-plugin"));
    }

    #[test]
    fn parse_plugin_type_invalid() {
        assert_eq!(parse_plugin_type("package"), None);
        assert_eq!(parse_plugin_type("file"), None);
    }

    #[test]
    fn validate_schema_pass() {
        let dir = TempDir::new().unwrap();
        let m = setup_plugin(dir.path(), "schema-test", b"wasm");
        // No schema defined → no errors
        let errors = validate_resource_schema(&m, &indexmap::IndexMap::new());
        assert!(errors.is_empty());
    }

    #[test]
    fn list_plugins_finds_dirs() {
        let dir = TempDir::new().unwrap();
        setup_plugin(dir.path(), "alpha", b"a");
        setup_plugin(dir.path(), "beta", b"b");

        let plugins = list_plugins(dir.path());
        assert_eq!(plugins, vec!["alpha", "beta"]);
    }

    #[test]
    fn list_plugins_empty() {
        let dir = TempDir::new().unwrap();
        assert!(list_plugins(dir.path()).is_empty());
    }

    #[test]
    fn resolve_and_verify_full() {
        let dir = TempDir::new().unwrap();
        let wasm = b"full pipeline test wasm";
        setup_plugin(dir.path(), "full-test", wasm);

        let resolved = resolve_and_verify(dir.path(), "full-test").unwrap();
        assert_eq!(resolved.manifest.name, "full-test");
        assert_eq!(resolved.status, PluginStatus::Converged);
        assert!(resolved.wasm_path.exists());
    }
}
