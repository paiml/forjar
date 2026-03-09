//! FJ-3403: `forjar plugin list|verify` CLI handler.
//!
//! Lists installed plugins and verifies plugin manifest integrity.

use crate::cli::commands::PluginCmd;
use crate::core::plugin_loader::{list_plugins, resolve_and_verify};
use crate::core::types::PluginStatus;
use std::path::Path;

/// Dispatch plugin subcommands.
pub fn dispatch_plugin(cmd: PluginCmd) -> Result<(), String> {
    match cmd {
        PluginCmd::List { plugin_dir, json } => cmd_plugin_list(&plugin_dir, json),
        PluginCmd::Verify { manifest, json } => cmd_plugin_verify(&manifest, json),
        PluginCmd::Init { name, output, json } => cmd_plugin_init(&name, output.as_deref(), json),
    }
}

/// List installed plugins in a directory.
fn cmd_plugin_list(plugin_dir: &Path, json: bool) -> Result<(), String> {
    let plugin_names = list_plugins(plugin_dir);

    if json {
        let entries: Vec<serde_json::Value> = plugin_names
            .iter()
            .map(|name| {
                // Try to resolve each plugin for full details
                match resolve_and_verify(plugin_dir, name) {
                    Ok(p) => serde_json::json!({
                        "name": p.manifest.name,
                        "version": p.manifest.version,
                        "abi_version": p.manifest.abi_version,
                        "status": format!("{:?}", p.status),
                    }),
                    Err(_) => serde_json::json!({
                        "name": name,
                        "status": "error",
                    }),
                }
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries).unwrap());
    } else {
        if plugin_names.is_empty() {
            println!("No plugins found in {}", plugin_dir.display());
            return Ok(());
        }
        let header = format!("{:<20} {:<10} {:<8} STATUS", "NAME", "VERSION", "ABI");
        println!("{header}");
        println!("{}", "-".repeat(60));
        for name in &plugin_names {
            match resolve_and_verify(plugin_dir, name) {
                Ok(p) => println!(
                    "{:<20} {:<10} {:<8} {:?}",
                    p.manifest.name, p.manifest.version, p.manifest.abi_version, p.status,
                ),
                Err(e) => println!("{:<20} {:<10} {:<8} {}", name, "?", "?", e),
            }
        }
    }

    Ok(())
}

/// Verify a plugin manifest and its WASM binary.
///
/// `manifest_path` should point to a plugin directory containing `plugin.yaml`.
fn cmd_plugin_verify(manifest_path: &Path, json: bool) -> Result<(), String> {
    // manifest_path is the plugin directory or the manifest file
    let (plugin_dir, plugin_name) = if manifest_path.is_dir() {
        let name = manifest_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("invalid plugin directory")?;
        let parent = manifest_path.parent().unwrap_or(manifest_path);
        (parent, name.to_string())
    } else {
        // Assume path is like plugins/myplugin/plugin.yaml
        let plugin_dir_path = manifest_path
            .parent()
            .ok_or("cannot determine plugin directory")?;
        let name = plugin_dir_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("invalid plugin directory name")?;
        let parent = plugin_dir_path.parent().unwrap_or(plugin_dir_path);
        (parent, name.to_string())
    };

    let result = resolve_and_verify(plugin_dir, &plugin_name)?;

    if json {
        let output = serde_json::json!({
            "name": result.manifest.name,
            "version": result.manifest.version,
            "status": format!("{:?}", result.status),
            "wasm_path": result.wasm_path.display().to_string(),
            "permissions": {
                "fs": result.manifest.permissions.fs,
                "net": result.manifest.permissions.net,
                "env": result.manifest.permissions.env,
                "exec": result.manifest.permissions.exec,
            },
        });
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        println!(
            "Plugin: {} v{}",
            result.manifest.name, result.manifest.version
        );
        println!("Status: {:?}", result.status);
        println!("WASM:   {}", result.wasm_path.display());
        let perms = &result.manifest.permissions;
        println!(
            "Perms:  fs={:?}, net={:?}, env={:?}, exec={:?}",
            perms.fs, perms.net, perms.env, perms.exec
        );
    }

    if result.status != PluginStatus::Converged {
        return Err(format!("plugin verification failed: {:?}", result.status));
    }

    Ok(())
}

/// FJ-3407: Scaffold a new plugin project directory.
///
/// Creates `<output>/<name>/` with a `plugin.yaml` manifest template
/// and an empty WASM stub file.
fn cmd_plugin_init(name: &str, output: Option<&Path>, json: bool) -> Result<(), String> {
    let base = output.unwrap_or_else(|| Path::new("plugins"));
    let plugin_dir = base.join(name);

    if plugin_dir.exists() {
        return Err(format!(
            "plugin directory already exists: {}",
            plugin_dir.display()
        ));
    }

    std::fs::create_dir_all(&plugin_dir)
        .map_err(|e| format!("create {}: {e}", plugin_dir.display()))?;

    // Write a stub WASM file (empty — to be replaced by real build)
    let wasm_path = plugin_dir.join("plugin.wasm");
    let wasm_stub = b"(module)";
    std::fs::write(&wasm_path, wasm_stub).map_err(|e| format!("write wasm: {e}"))?;

    let hash = blake3::hash(wasm_stub).to_hex().to_string();

    // Write manifest template
    let manifest = format!(
        r#"name: {name}
version: "0.1.0"
abi_version: 1
wasm: plugin.wasm
blake3: {hash}
description: "{name} plugin for forjar"
permissions:
  fs:
    read: []
    write: []
  net:
    connect: []
  env:
    read: []
  exec:
    allow: []
schema:
  required: []
  properties: {{}}
"#
    );

    let manifest_path = plugin_dir.join("plugin.yaml");
    std::fs::write(&manifest_path, &manifest).map_err(|e| format!("write manifest: {e}"))?;

    if json {
        println!(
            "{}",
            serde_json::json!({
                "name": name,
                "path": plugin_dir.display().to_string(),
                "manifest": manifest_path.display().to_string(),
                "wasm": wasm_path.display().to_string(),
            })
        );
    } else {
        println!("Created plugin scaffold: {}", plugin_dir.display());
        println!("  manifest: {}", manifest_path.display());
        println!("  wasm:     {} (stub)", wasm_path.display());
        println!();
        println!("Next steps:");
        println!("  1. Replace plugin.wasm with your compiled WASM module");
        println!(
            "  2. Update blake3 hash: forjar plugin verify {}",
            plugin_dir.display()
        );
        println!("  3. Configure permissions and schema in plugin.yaml");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_plugin_list(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn list_empty_dir_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_plugin_list(dir.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn verify_missing_manifest() {
        let result = cmd_plugin_verify(Path::new("/nonexistent/plugin"), false);
        assert!(result.is_err());
    }

    #[test]
    fn list_nonexistent_dir() {
        let result = cmd_plugin_list(Path::new("/nonexistent/plugins"), false);
        assert!(result.is_ok()); // list_plugins returns empty vec for missing dirs
    }

    #[test]
    fn dispatch_list() {
        let dir = tempfile::tempdir().unwrap();
        let cmd = PluginCmd::List {
            plugin_dir: dir.path().to_path_buf(),
            json: false,
        };
        let result = dispatch_plugin(cmd);
        assert!(result.is_ok());
    }

    #[test]
    fn init_creates_scaffold() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_plugin_init("my-plugin", Some(dir.path()), false);
        assert!(result.is_ok());
        assert!(dir.path().join("my-plugin/plugin.yaml").exists());
        assert!(dir.path().join("my-plugin/plugin.wasm").exists());

        let manifest = std::fs::read_to_string(dir.path().join("my-plugin/plugin.yaml")).unwrap();
        assert!(manifest.contains("name: my-plugin"));
        assert!(manifest.contains("version: \"0.1.0\""));
        assert!(manifest.contains("blake3:"));
    }

    #[test]
    fn init_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_plugin_init("test-plugin", Some(dir.path()), true);
        assert!(result.is_ok());
    }

    #[test]
    fn init_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("existing")).unwrap();
        let result = cmd_plugin_init("existing", Some(dir.path()), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[test]
    fn init_verify_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        cmd_plugin_init("roundtrip", Some(dir.path()), false).unwrap();

        // Should be verifiable immediately
        let verified = resolve_and_verify(dir.path(), "roundtrip");
        assert!(verified.is_ok(), "verify failed: {:?}", verified.err());
        let resolved = verified.unwrap();
        assert_eq!(resolved.manifest.name, "roundtrip");
        assert_eq!(resolved.status, PluginStatus::Converged);
    }

    #[test]
    fn dispatch_init() {
        let dir = tempfile::tempdir().unwrap();
        let cmd = PluginCmd::Init {
            name: "dispatch-test".into(),
            output: Some(dir.path().to_path_buf()),
            json: false,
        };
        let result = dispatch_plugin(cmd);
        assert!(result.is_ok());
    }
}
