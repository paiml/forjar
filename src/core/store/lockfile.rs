//! FJ-1310: Input lock file format.
//!
//! `forjar.inputs.lock.yaml` pins all resolved inputs to specific versions
//! and BLAKE3 hashes. Analogous to `flake.lock` / `Cargo.lock`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Lock file schema.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LockFile {
    pub schema: String,
    pub pins: BTreeMap<String, Pin>,
}

/// A single pinned input.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pin {
    pub provider: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_rev: Option<String>,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub pin_type: Option<String>,
}

/// Read a lock file from disk.
pub fn read_lockfile(path: &Path) -> Result<LockFile, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    parse_lockfile(&content)
}

/// Parse a lock file from YAML string.
pub fn parse_lockfile(yaml: &str) -> Result<LockFile, String> {
    serde_yaml_ng::from_str(yaml).map_err(|e| format!("parse lock file: {e}"))
}

/// Write a lock file atomically (temp file + rename).
pub fn write_lockfile(path: &Path, lockfile: &LockFile) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create dir {}: {e}", parent.display()))?;
    }
    let yaml = serde_yaml_ng::to_string(lockfile)
        .map_err(|e| format!("serialize lock file: {e}"))?;
    let tmp = path.with_extension("lock.yaml.tmp");
    std::fs::write(&tmp, &yaml)
        .map_err(|e| format!("write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path)
        .map_err(|e| format!("rename {} → {}: {e}", tmp.display(), path.display()))?;
    Ok(())
}

/// Stale pin: a pin whose hash no longer matches a given current hash.
#[derive(Debug, Clone, PartialEq)]
pub struct StalenessEntry {
    pub name: String,
    pub locked_hash: String,
    pub current_hash: String,
}

/// Check lock file for stale pins against current resolved hashes.
///
/// Returns entries where the locked hash differs from the current hash.
pub fn check_staleness(
    lockfile: &LockFile,
    current_hashes: &BTreeMap<String, String>,
) -> Vec<StalenessEntry> {
    let mut stale = Vec::new();
    for (name, pin) in &lockfile.pins {
        if let Some(current) = current_hashes.get(name) {
            if pin.hash != *current {
                stale.push(StalenessEntry {
                    name: name.clone(),
                    locked_hash: pin.hash.clone(),
                    current_hash: current.clone(),
                });
            }
        }
    }
    stale.sort_by(|a, b| a.name.cmp(&b.name));
    stale
}

/// Check completeness: all current inputs must have a pin.
/// Returns names of inputs missing from the lock file.
pub fn check_completeness(
    lockfile: &LockFile,
    current_inputs: &[String],
) -> Vec<String> {
    let mut missing: Vec<String> = current_inputs
        .iter()
        .filter(|name| !lockfile.pins.contains_key(name.as_str()))
        .cloned()
        .collect();
    missing.sort();
    missing
}
