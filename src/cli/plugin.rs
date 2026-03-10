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
        PluginCmd::Run {
            name,
            operation,
            plugin_dir,
            config,
            json,
        } => cmd_plugin_run(&name, &operation, &plugin_dir, &config, json),
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

/// FJ-3404: Execute a plugin operation via the WASM runtime.
fn cmd_plugin_run(
    name: &str,
    operation: &str,
    plugin_dir: &Path,
    config: &str,
    json: bool,
) -> Result<(), String> {
    let valid_ops = ["check", "apply", "destroy"];
    if !valid_ops.contains(&operation) {
        return Err(format!(
            "invalid operation '{operation}': use check, apply, or destroy"
        ));
    }
    let config_json: serde_json::Value =
        serde_json::from_str(config).map_err(|e| format!("parse config: {e}"))?;
    let result = crate::core::plugin_dispatch::dispatch_check(plugin_dir, name, &config_json);
    if !result.success {
        return Err(format!("plugin resolve failed: {}", result.message));
    }
    let dispatch_fn = match operation {
        "check" => crate::core::plugin_dispatch::dispatch_check,
        "apply" => crate::core::plugin_dispatch::dispatch_apply,
        "destroy" => crate::core::plugin_dispatch::dispatch_destroy,
        _ => unreachable!(),
    };
    let result = dispatch_fn(plugin_dir, name, &config_json);
    let runtime = if crate::core::plugin_runtime::is_runtime_available() {
        "wasmi"
    } else {
        "stub"
    };
    if json {
        println!(
            "{}",
            serde_json::json!({
                "plugin": result.plugin_name,
                "operation": result.operation,
                "success": result.success,
                "message": result.message,
                "status": format!("{:?}", result.status),
                "runtime": runtime,
            })
        );
    } else {
        println!("Plugin:    {}", result.plugin_name);
        println!("Operation: {}", result.operation);
        println!("Runtime:   {runtime}");
        println!("Status:    {:?}", result.status);
        println!("Success:   {}", result.success);
        if !result.message.is_empty() {
            println!("Message:   {}", result.message);
        }
    }
    if !result.success {
        Err(format!("plugin {operation} failed: {}", result.message))
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests_plugin.rs"]
mod tests;
