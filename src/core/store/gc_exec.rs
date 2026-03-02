//! FJ-1365: GC sweep execution.
//!
//! Bridges `gc::mark_and_sweep()` → actual filesystem deletion with
//! path traversal protection, dry-run support, and journal logging.

use super::gc::GcReport;
use std::path::Path;

/// Result of an executed GC sweep.
#[derive(Debug, Clone)]
pub struct GcSweepResult {
    /// Store hashes that were successfully removed
    pub removed: Vec<String>,
    /// Bytes freed by removal
    pub bytes_freed: u64,
    /// Entries that failed removal: (hash, error message)
    pub errors: Vec<(String, String)>,
}

/// Dry-run entry: what would be deleted.
#[derive(Debug, Clone)]
pub struct DryRunEntry {
    pub hash: String,
    pub size_bytes: u64,
}

/// Sweep dead entries from the store.
///
/// Validates all paths are under `store_dir` (no traversal attacks).
/// Continues on partial failure — collects errors per entry.
pub fn sweep(report: &GcReport, store_dir: &Path) -> Result<GcSweepResult, String> {
    if report.dead.is_empty() {
        return Ok(GcSweepResult {
            removed: Vec::new(),
            bytes_freed: 0,
            errors: Vec::new(),
        });
    }

    let mut removed = Vec::new();
    let mut bytes_freed = 0u64;
    let mut errors = Vec::new();

    // Write journal before deletion for recovery
    let journal_entries: Vec<(String, u64)> = report
        .dead
        .iter()
        .map(|h| {
            let path = entry_path(store_dir, h);
            let size = dir_size(&path);
            (h.clone(), size)
        })
        .collect();
    write_gc_journal(store_dir, &journal_entries)?;

    for hash in &report.dead {
        let path = entry_path(store_dir, hash);

        if let Err(e) = validate_store_path(&path, store_dir) {
            errors.push((hash.clone(), e));
            continue;
        }

        if !path.exists() {
            removed.push(hash.clone());
            continue;
        }

        let size = dir_size(&path);
        match std::fs::remove_dir_all(&path) {
            Ok(()) => {
                removed.push(hash.clone());
                bytes_freed += size;
            }
            Err(e) => {
                errors.push((hash.clone(), format!("rm {}: {e}", path.display())));
            }
        }
    }

    Ok(GcSweepResult {
        removed,
        bytes_freed,
        errors,
    })
}

/// Dry-run sweep: report what would be deleted without removing anything.
pub fn sweep_dry_run(report: &GcReport, store_dir: &Path) -> Vec<DryRunEntry> {
    report
        .dead
        .iter()
        .map(|hash| {
            let path = entry_path(store_dir, hash);
            let size = dir_size(&path);
            DryRunEntry {
                hash: hash.clone(),
                size_bytes: size,
            }
        })
        .collect()
}

/// Validate a path is safely under the store directory.
///
/// Prevents directory traversal attacks (e.g., `blake3:../../etc`).
fn validate_store_path(path: &Path, store_dir: &Path) -> Result<(), String> {
    // Canonicalize store_dir (it must exist)
    let canon_store = store_dir
        .canonicalize()
        .map_err(|e| format!("canonicalize store dir: {e}"))?;

    // For the entry path, check the parent exists (entry might not yet)
    let resolved = if path.exists() {
        path.canonicalize()
            .map_err(|e| format!("canonicalize {}: {e}", path.display()))?
    } else if let Some(parent) = path.parent() {
        if parent.exists() {
            let canon_parent = parent
                .canonicalize()
                .map_err(|e| format!("canonicalize parent: {e}"))?;
            canon_parent.join(path.file_name().unwrap_or_default())
        } else {
            return Err(format!("parent dir does not exist: {}", parent.display()));
        }
    } else {
        return Err("path has no parent directory".to_string());
    };

    if !resolved.starts_with(&canon_store) {
        return Err(format!(
            "path {} escapes store dir {}",
            resolved.display(),
            canon_store.display()
        ));
    }

    Ok(())
}

/// Build the filesystem path for a store entry from its hash.
fn entry_path(store_dir: &Path, hash: &str) -> std::path::PathBuf {
    let bare = hash.strip_prefix("blake3:").unwrap_or(hash);
    store_dir.join(bare)
}

/// Calculate directory size recursively.
pub fn dir_size(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    let entries = match std::fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return 0,
    };
    let mut total = 0u64;
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            total += dir_size(&p);
        } else if p.is_file() {
            total += std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
        }
    }
    total
}

/// Write GC journal entry for recovery.
///
/// Records what was deleted so a failed GC can be diagnosed.
fn write_gc_journal(store_dir: &Path, removed: &[(String, u64)]) -> Result<(), String> {
    let journal_dir = store_dir.join(".gc-journal");
    std::fs::create_dir_all(&journal_dir).map_err(|e| format!("create gc journal dir: {e}"))?;

    let timestamp = crate::tripwire::eventlog::now_iso8601();
    let filename = format!("gc-{}.yaml", timestamp.replace(':', "-"));
    let path = journal_dir.join(filename);

    let mut content = String::from("# GC journal — entries marked for removal\n");
    content.push_str(&format!("timestamp: \"{timestamp}\"\n"));
    content.push_str("entries:\n");
    for (hash, size) in removed {
        content.push_str(&format!("  - hash: \"{hash}\"\n    size_bytes: {size}\n"));
    }

    std::fs::write(&path, &content)
        .map_err(|e| format!("write gc journal {}: {e}", path.display()))?;
    Ok(())
}
