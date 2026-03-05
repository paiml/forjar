//! FJ-1325/FJ-1326: Garbage collection — GC roots and mark-and-sweep.
//!
//! GC roots are store hashes reachable from: current profile symlink,
//! profile generations (keep last N), lock file pins, `.gc-roots/` symlinks.
//!
//! Mark-and-sweep: walk roots, follow `references` in `meta.yaml`, mark
//! as live. Unreachable entries are dead (candidates for deletion).

use super::meta::read_meta;
use std::collections::BTreeSet;
use std::path::Path;

/// GC configuration.
pub struct GcConfig {
    /// Number of profile generations to keep.
    pub keep_generations: usize,
    /// Store entries older than this many days are eligible for collection.
    pub older_than_days: Option<u64>,
}

impl Default for GcConfig {
    fn default() -> Self {
        Self {
            keep_generations: 5,
            older_than_days: None,
        }
    }
}

/// Result of a GC analysis (mark phase).
#[derive(Debug, Clone)]
pub struct GcReport {
    /// Store hashes reachable from GC roots.
    pub live: BTreeSet<String>,
    /// Store hashes not reachable (candidates for deletion).
    pub dead: BTreeSet<String>,
    /// Total number of store entries.
    pub total: usize,
}

/// Collect GC roots from multiple sources.
///
/// Sources: profile generations, lock file pins, explicit gc-roots dir.
pub fn collect_roots(
    profile_hashes: &[String],
    lockfile_hashes: &[String],
    gc_roots_dir: Option<&Path>,
) -> BTreeSet<String> {
    let mut roots: BTreeSet<String> = profile_hashes.iter().cloned().collect();
    roots.extend(lockfile_hashes.iter().cloned());
    if let Some(dir) = gc_roots_dir {
        roots.extend(scan_gc_roots_dir(dir));
    }
    roots
}

/// Mark-and-sweep: starting from roots, follow references to mark live entries.
pub fn mark_and_sweep(roots: &BTreeSet<String>, store_dir: &Path) -> Result<GcReport, String> {
    let all_entries = list_store_entries(store_dir)?;
    let live = mark_live(roots, store_dir);
    let dead: BTreeSet<String> = all_entries.difference(&live).cloned().collect();
    let total = all_entries.len();
    Ok(GcReport { live, dead, total })
}

/// BFS from roots following references in meta.yaml.
fn mark_live(roots: &BTreeSet<String>, store_dir: &Path) -> BTreeSet<String> {
    let mut live = BTreeSet::new();
    let mut queue: Vec<String> = roots.iter().cloned().collect();

    while let Some(hash) = queue.pop() {
        if !live.insert(hash.clone()) {
            continue;
        }
        let entry_dir = store_dir.join(hash.strip_prefix("blake3:").unwrap_or(&hash));
        if let Ok(meta) = read_meta(&entry_dir) {
            for r in &meta.references {
                if !live.contains(r) {
                    queue.push(r.clone());
                }
            }
        }
    }
    live
}

/// Scan .gc-roots/ directory for symlinks pointing to store entries.
fn scan_gc_roots_dir(dir: &Path) -> Vec<String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    entries
        .flatten()
        .filter(|e| e.path().is_symlink())
        .filter_map(|e| std::fs::read_link(e.path()).ok())
        .filter_map(|target| extract_store_hash(&target))
        .collect()
}

/// List all store entry hashes (directory names under store_dir).
fn list_store_entries(store_dir: &Path) -> Result<BTreeSet<String>, String> {
    let read_dir = std::fs::read_dir(store_dir)
        .map_err(|e| format!("read store dir {}: {e}", store_dir.display()))?;
    let entries = read_dir
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
        .filter(|name| name != ".gc-roots")
        .map(|name| format!("blake3:{name}"))
        .collect();
    Ok(entries)
}

/// Extract a store hash from a symlink target path.
///
/// Expects paths like `/var/forjar/store/<hex>/content` → `blake3:<hex>`.
fn extract_store_hash(target: &Path) -> Option<String> {
    target
        .to_str()?
        .split('/')
        .find(|c| c.len() == 64 && c.chars().all(|ch| ch.is_ascii_hexdigit()))
        .map(|c| format!("blake3:{c}"))
}
