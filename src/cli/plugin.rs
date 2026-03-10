//! FJ-3403: `forjar plugin` CLI handler — list, verify, init, install, build, remove.

use crate::cli::commands::PluginCmd;
use crate::core::plugin_loader::{list_plugins, resolve_and_verify, resolve_manifest};
use crate::core::types::PluginStatus;
use std::path::Path;

/// Dispatch plugin subcommands.
pub fn dispatch_plugin(cmd: PluginCmd) -> Result<(), String> {
    match cmd {
        PluginCmd::List { plugin_dir, json } => cmd_plugin_list(&plugin_dir, json),
        PluginCmd::Verify { manifest, json } => cmd_plugin_verify(&manifest, json),
        PluginCmd::Init { name, output, json } => cmd_plugin_init(&name, output.as_deref(), json),
        PluginCmd::Install {
            source,
            plugin_dir,
            json,
        } => cmd_plugin_install(&source, &plugin_dir, json),
        PluginCmd::Build { path, output, json } => cmd_plugin_build(&path, output.as_deref(), json),
        PluginCmd::Remove {
            name,
            plugin_dir,
            yes,
            json,
        } => cmd_plugin_remove(&name, &plugin_dir, yes, json),
    }
}

/// List installed plugins in a directory.
fn cmd_plugin_list(plugin_dir: &Path, json: bool) -> Result<(), String> {
    let names = list_plugins(plugin_dir);
    if json {
        let entries: Vec<serde_json::Value> = names
            .iter()
            .map(|name| match resolve_and_verify(plugin_dir, name) {
                Ok(p) => serde_json::json!({
                    "name": p.manifest.name, "version": p.manifest.version,
                    "abi_version": p.manifest.abi_version, "status": format!("{:?}", p.status),
                }),
                Err(_) => serde_json::json!({"name": name, "status": "error"}),
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries).unwrap());
    } else {
        if names.is_empty() {
            println!("No plugins found in {}", plugin_dir.display());
            return Ok(());
        }
        println!("{:<20} {:<10} {:<8} STATUS", "NAME", "VERSION", "ABI");
        println!("{}", "-".repeat(60));
        for name in &names {
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
fn cmd_plugin_verify(manifest_path: &Path, json: bool) -> Result<(), String> {
    let (plugin_dir, plugin_name) = if manifest_path.is_dir() {
        let name = manifest_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("invalid plugin directory")?;
        (
            manifest_path.parent().unwrap_or(manifest_path),
            name.to_string(),
        )
    } else {
        let dir = manifest_path
            .parent()
            .ok_or("cannot determine plugin directory")?;
        let name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("invalid plugin directory name")?;
        (dir.parent().unwrap_or(dir), name.to_string())
    };
    let result = resolve_and_verify(plugin_dir, &plugin_name)?;
    if json {
        let output = serde_json::json!({
            "name": result.manifest.name, "version": result.manifest.version,
            "status": format!("{:?}", result.status),
            "wasm_path": result.wasm_path.display().to_string(),
            "permissions": { "fs": result.manifest.permissions.fs,
                "net": result.manifest.permissions.net,
                "env": result.manifest.permissions.env,
                "exec": result.manifest.permissions.exec },
        });
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        println!(
            "Plugin: {} v{}",
            result.manifest.name, result.manifest.version
        );
        println!("Status: {:?}", result.status);
        println!("WASM:   {}", result.wasm_path.display());
        let p = &result.manifest.permissions;
        println!(
            "Perms:  fs={:?}, net={:?}, env={:?}, exec={:?}",
            p.fs, p.net, p.env, p.exec
        );
    }
    if result.status != PluginStatus::Converged {
        return Err(format!("plugin verification failed: {:?}", result.status));
    }
    Ok(())
}

/// FJ-3407: Scaffold a new plugin project directory.
fn cmd_plugin_init(name: &str, output: Option<&Path>, json: bool) -> Result<(), String> {
    let base = output.unwrap_or_else(|| Path::new("plugins"));
    let dir = base.join(name);
    if dir.exists() {
        return Err(format!(
            "plugin directory already exists: {}",
            dir.display()
        ));
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    let wasm_path = dir.join("plugin.wasm");
    let wasm_stub = b"(module)";
    std::fs::write(&wasm_path, wasm_stub).map_err(|e| format!("write wasm: {e}"))?;
    let hash = blake3::hash(wasm_stub).to_hex().to_string();
    let manifest = [
        &format!("name: {name}"),
        "version: \"0.1.0\"",
        "abi_version: 1",
        "wasm: plugin.wasm",
        &format!("blake3: {hash}"),
        &format!("description: \"{name} plugin for forjar\""),
        "permissions:",
        "  fs:",
        "    read: []",
        "    write: []",
        "  net:",
        "    connect: []",
        "  env:",
        "    read: []",
        "  exec:",
        "    allow: []",
        "schema:",
        "  required: []",
        "  properties: {}",
        "",
    ]
    .join("\n");
    let manifest_path = dir.join("plugin.yaml");
    std::fs::write(&manifest_path, &manifest).map_err(|e| format!("write manifest: {e}"))?;
    if json {
        println!(
            "{}",
            serde_json::json!({
                "name": name, "path": dir.display().to_string(),
                "manifest": manifest_path.display().to_string(),
                "wasm": wasm_path.display().to_string(),
            })
        );
    } else {
        println!("Created plugin scaffold: {}", dir.display());
        println!("  manifest: {}", manifest_path.display());
        println!("  wasm:     {} (stub)", wasm_path.display());
        println!("\nNext steps:");
        println!("  1. Replace plugin.wasm with your compiled WASM module");
        println!(
            "  2. Update blake3 hash: forjar plugin verify {}",
            dir.display()
        );
        println!("  3. Configure permissions and schema in plugin.yaml");
    }
    Ok(())
}

/// Install a plugin from a local source directory, verifying BLAKE3 after copy.
fn cmd_plugin_install(source: &str, plugin_dir: &Path, json: bool) -> Result<(), String> {
    let src = Path::new(source);
    if !src.is_dir() {
        return Err(format!("source is not a directory: {source}"));
    }
    let manifest_src = src.join("plugin.yaml");
    if !manifest_src.exists() {
        return Err(format!("no plugin.yaml found in {}", src.display()));
    }
    let content = std::fs::read_to_string(&manifest_src)
        .map_err(|e| format!("read {}: {e}", manifest_src.display()))?;
    let manifest: crate::core::types::PluginManifest = serde_yaml_ng::from_str(&content)
        .map_err(|e| format!("parse {}: {e}", manifest_src.display()))?;
    let name = &manifest.name;
    let dest = plugin_dir.join(name);
    if dest.exists() {
        return Err(format!(
            "plugin '{}' already installed at {}",
            name,
            dest.display()
        ));
    }
    std::fs::create_dir_all(&dest).map_err(|e| format!("create {}: {e}", dest.display()))?;
    std::fs::copy(&manifest_src, dest.join("plugin.yaml"))
        .map_err(|e| format!("copy plugin.yaml: {e}"))?;
    let wasm_src = src.join(&manifest.wasm);
    if !wasm_src.exists() {
        return Err(format!("WASM file not found: {}", wasm_src.display()));
    }
    std::fs::copy(&wasm_src, dest.join(&manifest.wasm))
        .map_err(|e| format!("copy {}: {e}", manifest.wasm))?;
    resolve_and_verify(plugin_dir, name).map_err(|e| format!("post-install verify failed: {e}"))?;
    if json {
        println!(
            "{}",
            serde_json::json!({
                "installed": name, "path": dest.display().to_string(), "version": manifest.version,
            })
        );
    } else {
        println!(
            "Installed plugin '{}' v{} to {}",
            name,
            manifest.version,
            dest.display()
        );
    }
    Ok(())
}

/// Build a WASM plugin from a Rust source directory.
fn cmd_plugin_build(path: &Path, output: Option<&Path>, json: bool) -> Result<(), String> {
    if !path.join("Cargo.toml").exists() {
        return Err(format!("no Cargo.toml found in {}", path.display()));
    }
    let cargo_content = std::fs::read_to_string(path.join("Cargo.toml"))
        .map_err(|e| format!("read Cargo.toml: {e}"))?;
    let parsed: toml::Value =
        toml::from_str(&cargo_content).map_err(|e| format!("parse Cargo.toml: {e}"))?;
    let crate_name = parsed
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(String::from)
        .ok_or_else(|| "no [package] name in Cargo.toml".to_string())?;
    let status = std::process::Command::new("cargo")
        .args(["build", "--target", "wasm32-unknown-unknown", "--release"])
        .current_dir(path)
        .status()
        .map_err(|e| format!("cargo build failed to start: {e}"))?;
    if !status.success() {
        return Err(
            "cargo build failed. Ensure wasm32-unknown-unknown target is installed: \
                     `rustup target add wasm32-unknown-unknown`"
                .to_string(),
        );
    }
    let wasm_src = path
        .join("target/wasm32-unknown-unknown/release")
        .join(format!("{}.wasm", crate_name.replace('-', "_")));
    if !wasm_src.exists() {
        return Err(format!("built WASM not found: {}", wasm_src.display()));
    }
    let out_dir = output.unwrap_or_else(|| Path::new("plugins"));
    let dest = out_dir.join(&crate_name);
    std::fs::create_dir_all(&dest).map_err(|e| format!("create {}: {e}", dest.display()))?;
    let wasm_dest = dest.join("plugin.wasm");
    std::fs::copy(&wasm_src, &wasm_dest).map_err(|e| format!("copy wasm: {e}"))?;
    let wasm_bytes = std::fs::read(&wasm_dest).map_err(|e| format!("read wasm: {e}"))?;
    let hash = blake3::hash(&wasm_bytes).to_hex().to_string();
    let manifest = format!(
        "name: {crate_name}\nversion: \"0.1.0\"\nabi_version: 1\n\
         wasm: plugin.wasm\nblake3: {hash}\n"
    );
    std::fs::write(dest.join("plugin.yaml"), &manifest).map_err(|e| format!("write: {e}"))?;
    if json {
        println!(
            "{}",
            serde_json::json!({
                "name": crate_name, "path": dest.display().to_string(),
                "wasm": wasm_dest.display().to_string(), "blake3": hash,
            })
        );
    } else {
        println!("Built plugin '{crate_name}'");
        println!("  wasm: {}", wasm_dest.display());
        println!("  blake3: {hash}");
        println!("  manifest: {}", dest.join("plugin.yaml").display());
    }
    Ok(())
}

/// Remove an installed plugin directory.
fn cmd_plugin_remove(name: &str, plugin_dir: &Path, yes: bool, json: bool) -> Result<(), String> {
    let target = plugin_dir.join(name);
    if !target.exists() {
        return Err(format!(
            "plugin '{}' not found in {}",
            name,
            plugin_dir.display()
        ));
    }
    if !target.join("plugin.yaml").exists() {
        return Err(format!(
            "{} has no plugin.yaml — refusing to remove",
            target.display()
        ));
    }
    if !yes {
        eprintln!(
            "Would remove plugin '{}' at {}. Use --yes to confirm.",
            name,
            target.display()
        );
        return Ok(());
    }
    let version = resolve_manifest(plugin_dir, name)
        .map(|m| m.version)
        .unwrap_or_else(|_| "unknown".to_string());
    std::fs::remove_dir_all(&target).map_err(|e| format!("remove {}: {e}", target.display()))?;
    if json {
        println!(
            "{}",
            serde_json::json!({"removed": name, "version": version,
            "path": target.display().to_string()})
        );
    } else {
        println!("Removed plugin '{}' v{}", name, version);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_plugin_list(dir.path(), false).is_ok());
    }
    #[test]
    fn list_empty_dir_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_plugin_list(dir.path(), true).is_ok());
    }
    #[test]
    fn verify_missing_manifest() {
        assert!(cmd_plugin_verify(Path::new("/nonexistent/plugin"), false).is_err());
    }
    #[test]
    fn list_nonexistent_dir() {
        assert!(cmd_plugin_list(Path::new("/nonexistent/plugins"), false).is_ok());
    }
    #[test]
    fn dispatch_list() {
        let dir = tempfile::tempdir().unwrap();
        let cmd = PluginCmd::List {
            plugin_dir: dir.path().to_path_buf(),
            json: false,
        };
        assert!(dispatch_plugin(cmd).is_ok());
    }
    #[test]
    fn init_creates_scaffold() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_plugin_init("my-plugin", Some(dir.path()), false).is_ok());
        assert!(dir.path().join("my-plugin/plugin.yaml").exists());
        assert!(dir.path().join("my-plugin/plugin.wasm").exists());
        let m = std::fs::read_to_string(dir.path().join("my-plugin/plugin.yaml")).unwrap();
        assert!(m.contains("name: my-plugin"));
        assert!(m.contains("blake3:"));
    }
    #[test]
    fn init_json_output() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_plugin_init("test-plugin", Some(dir.path()), true).is_ok());
    }
    #[test]
    fn init_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("existing")).unwrap();
        let r = cmd_plugin_init("existing", Some(dir.path()), false);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("already exists"));
    }
    #[test]
    fn init_verify_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        cmd_plugin_init("roundtrip", Some(dir.path()), false).unwrap();
        let v = resolve_and_verify(dir.path(), "roundtrip");
        assert!(v.is_ok(), "verify failed: {:?}", v.err());
        assert_eq!(v.unwrap().status, PluginStatus::Converged);
    }
    #[test]
    fn dispatch_init() {
        let dir = tempfile::tempdir().unwrap();
        let cmd = PluginCmd::Init {
            name: "d-test".into(),
            output: Some(dir.path().into()),
            json: false,
        };
        assert!(dispatch_plugin(cmd).is_ok());
    }
    #[test]
    fn install_from_local_dir() {
        let src = tempfile::tempdir().unwrap();
        cmd_plugin_init("src-plug", Some(src.path()), false).unwrap();
        let dest = tempfile::tempdir().unwrap();
        let r = cmd_plugin_install(
            src.path().join("src-plug").to_str().unwrap(),
            dest.path(),
            false,
        );
        assert!(r.is_ok(), "install failed: {:?}", r.err());
        assert!(dest.path().join("src-plug/plugin.yaml").exists());
    }
    #[test]
    fn install_already_exists() {
        let src = tempfile::tempdir().unwrap();
        cmd_plugin_init("dup", Some(src.path()), false).unwrap();
        let dest = tempfile::tempdir().unwrap();
        cmd_plugin_install(src.path().join("dup").to_str().unwrap(), dest.path(), false).unwrap();
        let r = cmd_plugin_install(src.path().join("dup").to_str().unwrap(), dest.path(), false);
        assert!(r.is_err());
    }
    #[test]
    fn install_missing_source() {
        let dest = tempfile::tempdir().unwrap();
        assert!(cmd_plugin_install("/nonexistent/path", dest.path(), false).is_err());
    }
    #[test]
    fn install_no_manifest() {
        let src = tempfile::tempdir().unwrap();
        let dest = tempfile::tempdir().unwrap();
        let r = cmd_plugin_install(src.path().to_str().unwrap(), dest.path(), false);
        assert!(r.unwrap_err().contains("no plugin.yaml"));
    }
    #[test]
    fn remove_plugin() {
        let dir = tempfile::tempdir().unwrap();
        cmd_plugin_init("removable", Some(dir.path()), false).unwrap();
        cmd_plugin_remove("removable", dir.path(), true, false).unwrap();
        assert!(!dir.path().join("removable").exists());
    }
    #[test]
    fn remove_without_yes() {
        let dir = tempfile::tempdir().unwrap();
        cmd_plugin_init("keep", Some(dir.path()), false).unwrap();
        cmd_plugin_remove("keep", dir.path(), false, false).unwrap();
        assert!(dir.path().join("keep").exists()); // not removed without --yes
    }
    #[test]
    fn remove_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_plugin_remove("nope", dir.path(), true, false).is_err());
    }
    #[test]
    fn remove_no_manifest() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("bare")).unwrap();
        let r = cmd_plugin_remove("bare", dir.path(), true, false);
        assert!(r.unwrap_err().contains("plugin.yaml"));
    }
    #[test]
    fn build_no_cargo_toml() {
        let dir = tempfile::tempdir().unwrap();
        let r = cmd_plugin_build(dir.path(), None, false);
        assert!(r.unwrap_err().contains("Cargo.toml"));
    }
    #[test]
    fn build_invalid_cargo_toml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[[[bad").unwrap();
        assert!(cmd_plugin_build(dir.path(), None, false)
            .unwrap_err()
            .contains("parse"));
    }
    #[test]
    fn build_no_package_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        assert!(cmd_plugin_build(dir.path(), None, false)
            .unwrap_err()
            .contains("[package] name"));
    }
}
