//! FJ-1311–FJ-1313: `forjar pin` — input pinning CLI commands.
//!
//! - `forjar pin` — pin all inputs to current versions
//! - `forjar pin --update <name>` — re-resolve and re-hash specific pin
//! - `forjar pin --update` — update all pins
//! - `forjar pin --check` — CI gate — fail if lock file is stale

use crate::core::store::lockfile::{
    check_completeness, check_staleness, read_lockfile, write_lockfile, LockFile, Pin,
};
use std::collections::BTreeMap;
use std::path::Path;

/// Pin all inputs to current versions, creating or updating the lock file.
pub(crate) fn cmd_pin(file: &Path, state_dir: &Path, json: bool) -> Result<(), String> {
    let resources = resolve_resources(file)?;
    let lock_path = lock_file_path(state_dir);

    let mut pins = BTreeMap::new();
    for (name, provider, version) in &resources {
        let hash = compute_pin_hash(name, provider, version.as_deref());
        pins.insert(
            name.clone(),
            Pin {
                provider: provider.clone(),
                version: version.clone(),
                hash,
                git_rev: None,
                pin_type: None,
            },
        );
    }

    let lockfile = LockFile {
        schema: "1.0".to_string(),
        pins,
    };
    write_lockfile(&lock_path, &lockfile)?;

    if json {
        let yaml = serde_yaml_ng::to_string(&lockfile).map_err(|e| format!("serialize: {e}"))?;
        println!("{yaml}");
    } else {
        println!(
            "Pinned {} inputs to {}",
            lockfile.pins.len(),
            lock_path.display()
        );
        for (name, pin) in &lockfile.pins {
            let ver = pin.version.as_deref().unwrap_or("latest");
            println!("  {name}: {ver} ({})", &pin.hash[..20]);
        }
    }
    Ok(())
}

/// Update a specific pin (or all) by re-resolving and re-hashing.
pub(crate) fn cmd_pin_update(
    file: &Path,
    state_dir: &Path,
    target: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let lock_path = lock_file_path(state_dir);
    let mut lockfile = read_lockfile(&lock_path).unwrap_or(LockFile {
        schema: "1.0".to_string(),
        pins: BTreeMap::new(),
    });

    let resources = resolve_resources(file)?;
    let mut updated = Vec::new();

    for (name, provider, version) in &resources {
        if let Some(t) = target {
            if name != t {
                continue;
            }
        }
        let hash = compute_pin_hash(name, provider, version.as_deref());
        let changed = lockfile.pins.get(name).is_none_or(|p| p.hash != hash);
        lockfile.pins.insert(
            name.clone(),
            Pin {
                provider: provider.clone(),
                version: version.clone(),
                hash: hash.clone(),
                git_rev: None,
                pin_type: None,
            },
        );
        if changed {
            updated.push(name.clone());
        }
    }

    write_lockfile(&lock_path, &lockfile)?;

    if json {
        let yaml = serde_yaml_ng::to_string(&lockfile).map_err(|e| format!("serialize: {e}"))?;
        println!("{yaml}");
    } else if updated.is_empty() {
        println!("All pins are up to date");
    } else {
        println!("Updated {} pin(s):", updated.len());
        for name in &updated {
            println!("  {name}");
        }
    }
    Ok(())
}

/// CI gate: fail if lock file is stale or incomplete.
pub(crate) fn cmd_pin_check(file: &Path, state_dir: &Path, json: bool) -> Result<(), String> {
    let lock_path = lock_file_path(state_dir);
    let lockfile = read_lockfile(&lock_path)?;
    let resources = resolve_resources(file)?;

    let current_hashes: BTreeMap<String, String> = resources
        .iter()
        .map(|(n, p, v)| (n.clone(), compute_pin_hash(n, p, v.as_deref())))
        .collect();
    let current_names: Vec<String> = resources.iter().map(|(n, _, _)| n.clone()).collect();

    let stale = check_staleness(&lockfile, &current_hashes);
    let missing = check_completeness(&lockfile, &current_names);

    if json {
        let report = serde_json::json!({
            "stale": stale.len(),
            "missing": missing.len(),
            "pass": stale.is_empty() && missing.is_empty(),
            "stale_pins": stale.iter().map(|s| &s.name).collect::<Vec<_>>(),
            "missing_inputs": missing,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        if !stale.is_empty() {
            println!("Stale pins ({}):", stale.len());
            for s in &stale {
                println!(
                    "  {}: locked={} current={}",
                    s.name,
                    &s.locked_hash[..16],
                    &s.current_hash[..16]
                );
            }
        }
        if !missing.is_empty() {
            println!("Missing pins ({}):", missing.len());
            for m in &missing {
                println!("  {m}");
            }
        }
    }

    if stale.is_empty() && missing.is_empty() {
        if !json {
            println!("Lock file is fresh and complete — PASS");
        }
        Ok(())
    } else {
        Err(format!(
            "Lock file check FAILED: {} stale, {} missing",
            stale.len(),
            missing.len()
        ))
    }
}

/// Build lock file path from state directory.
fn lock_file_path(state_dir: &Path) -> std::path::PathBuf {
    state_dir.join("forjar.inputs.lock.yaml")
}

/// Resolve resources from config to (name, provider, version) tuples.
fn resolve_resources(file: &Path) -> Result<Vec<(String, String, Option<String>)>, String> {
    let content =
        std::fs::read_to_string(file).map_err(|e| format!("read {}: {e}", file.display()))?;
    let doc: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("parse {}: {e}", file.display()))?;
    let resources = doc
        .get("resources")
        .and_then(|r| r.as_mapping())
        .ok_or_else(|| "no resources section found".to_string())?;

    let mut result = Vec::new();
    for (key, val) in resources {
        let name = key.as_str().unwrap_or("").to_string();
        let provider = val
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("file")
            .to_string();
        let version = val
            .get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        result.push((name, provider, version));
    }
    result.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(result)
}

/// Compute BLAKE3 hash for a pin (name + provider + version).
fn compute_pin_hash(name: &str, provider: &str, version: Option<&str>) -> String {
    let input = format!("{}:{}:{}", name, provider, version.unwrap_or("latest"));
    format!("blake3:{}", blake3::hash(input.as_bytes()).to_hex())
}
