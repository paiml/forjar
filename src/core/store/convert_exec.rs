//! FJ-1363: Convert --apply execution.
//!
//! Bridges `convert::analyze_conversion()` → actual YAML modification.
//! Applies automated conversion changes (version pins, store flags,
//! lock file generation) with backup and atomic write.

use super::convert::{ChangeType, ConversionReport};
use super::lockfile::{write_lockfile, LockFile, Pin};
use super::pin_resolve::pin_hash;
use super::purity::PurityLevel;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Result of applying automated conversion changes.
#[derive(Debug, Clone)]
pub struct ConversionApplyResult {
    /// Number of changes applied
    pub changes_applied: usize,
    /// Path to the backup file
    pub backup_path: PathBuf,
    /// New purity level after changes
    pub new_purity: PurityLevel,
    /// Lock file entries generated
    pub lock_pins_generated: usize,
}

/// Apply automated conversion changes to a config file.
///
/// Steps:
/// 1. Backup original config (forjar.yaml → forjar.yaml.bak)
/// 2. Parse YAML into mutable Value
/// 3. Apply version pins (AddVersionPin)
/// 4. Add store: true flags (EnableStore)
/// 5. Write updated config atomically
/// 6. Generate lock file entries (GenerateLockPin)
pub fn apply_conversion(
    config_path: &Path,
    report: &ConversionReport,
) -> Result<ConversionApplyResult, String> {
    if report.auto_change_count == 0 {
        return Ok(ConversionApplyResult {
            changes_applied: 0,
            backup_path: config_path.to_path_buf(),
            new_purity: report.current_purity,
            lock_pins_generated: 0,
        });
    }

    // Step 1: Backup
    let backup_path = backup_config(config_path)?;

    // Step 2: Parse YAML
    let content = std::fs::read_to_string(config_path)
        .map_err(|e| format!("read config {}: {e}", config_path.display()))?;
    let mut doc: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("parse config YAML: {e}"))?;

    // Steps 3-4: Apply changes
    let (applied, lock_pins) = apply_report_changes(&mut doc, report);

    // Step 5: Write updated config atomically
    let yaml_out =
        serde_yaml_ng::to_string(&doc).map_err(|e| format!("serialize updated YAML: {e}"))?;
    atomic_write(config_path, &yaml_out)?;

    // Step 6: Write lock file if pins were generated
    let lock_count = lock_pins.len();
    if !lock_pins.is_empty() {
        let lock_path = lock_file_path(config_path);
        let lockfile = LockFile {
            schema: "1.0".to_string(),
            pins: lock_pins,
        };
        write_lockfile(&lock_path, &lockfile)?;
    }

    Ok(ConversionApplyResult {
        changes_applied: applied,
        backup_path,
        new_purity: report.projected_purity,
        lock_pins_generated: lock_count,
    })
}

/// Apply all auto-changes from a conversion report, returning (count, lock_pins).
fn apply_report_changes(
    doc: &mut serde_yaml_ng::Value,
    report: &ConversionReport,
) -> (usize, BTreeMap<String, Pin>) {
    let mut applied = 0usize;
    let mut lock_pins = BTreeMap::new();

    for resource_conv in &report.resources {
        for change in &resource_conv.auto_changes {
            match change.change_type {
                ChangeType::AddVersionPin => {
                    if apply_version_pin(doc, &resource_conv.name) {
                        applied += 1;
                    }
                }
                ChangeType::EnableStore => {
                    if apply_store_flag(doc, &resource_conv.name) {
                        applied += 1;
                    }
                }
                ChangeType::GenerateLockPin => {
                    let hash = pin_hash(&resource_conv.provider, &resource_conv.name, "latest");
                    lock_pins.insert(
                        resource_conv.name.clone(),
                        Pin {
                            provider: resource_conv.provider.clone(),
                            version: None,
                            hash,
                            git_rev: None,
                            pin_type: None,
                        },
                    );
                    applied += 1;
                }
            }
        }
    }

    (applied, lock_pins)
}

/// Apply a version pin to a resource in the YAML document.
///
/// Sets `version: "latest"` as a placeholder for the resource to be
/// resolved later by `pin_resolve::resolve_all_pins()`.
fn apply_version_pin(doc: &mut serde_yaml_ng::Value, resource_name: &str) -> bool {
    if let Some(resource) = find_resource_mut(doc, resource_name) {
        if resource.get("version").is_none() {
            resource["version"] = serde_yaml_ng::Value::String("latest".to_string());
            return true;
        }
    }
    false
}

/// Add `store: true` to a resource in the YAML document.
fn apply_store_flag(doc: &mut serde_yaml_ng::Value, resource_name: &str) -> bool {
    if let Some(resource) = find_resource_mut(doc, resource_name) {
        if resource.get("store").is_none() {
            resource["store"] = serde_yaml_ng::Value::Bool(true);
            return true;
        }
    }
    false
}

/// Find a resource by name in the YAML document (mutable).
///
/// Searches under `resources:` array for an entry with `name: <resource_name>`.
fn find_resource_mut<'a>(
    doc: &'a mut serde_yaml_ng::Value,
    name: &str,
) -> Option<&'a mut serde_yaml_ng::Value> {
    let resources = doc.get_mut("resources")?;
    let seq = resources.as_sequence_mut()?;
    seq.iter_mut().find(|r| {
        r.get("name")
            .and_then(|n| n.as_str())
            .is_some_and(|n| n == name)
    })
}

/// Backup the original config file.
fn backup_config(path: &Path) -> Result<PathBuf, String> {
    let backup = path.with_extension("yaml.bak");
    std::fs::copy(path, &backup)
        .map_err(|e| format!("backup {} → {}: {e}", path.display(), backup.display()))?;
    Ok(backup)
}

/// Atomic write via temp file + rename.
fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    let tmp = path.with_extension("yaml.tmp");
    std::fs::write(&tmp, content).map_err(|e| format!("write tmp {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path)
        .map_err(|e| format!("rename {} → {}: {e}", tmp.display(), path.display()))?;
    Ok(())
}

/// Derive the lock file path from a config path.
fn lock_file_path(config_path: &Path) -> PathBuf {
    let parent = config_path.parent().unwrap_or(Path::new("."));
    parent.join("forjar.inputs.lock.yaml")
}
