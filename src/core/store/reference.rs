//! FJ-1304: Reference scanning for store paths.
//!
//! Scans store entry output files for BLAKE3 store path hashes, building
//! the reference graph needed by garbage collection (Section 7).
//!
//! References are discovered by conservative scanning — any 64-char hex
//! string prefixed by `blake3:` that matches a known store hash is
//! recorded as a reference.

use std::collections::BTreeSet;
use std::path::Path;

/// Pattern length for a BLAKE3 store hash: "blake3:" (7) + 64 hex = 71 chars.
const BLAKE3_PREFIX: &str = "blake3:";
const HEX_HASH_LEN: usize = 64;

/// Scan a single file's contents for store hash references.
///
/// Returns the set of `blake3:<hex>` strings found in the file content
/// that also appear in `known_hashes`.
pub fn scan_file_refs(content: &[u8], known_hashes: &BTreeSet<String>) -> BTreeSet<String> {
    let text = String::from_utf8_lossy(content);
    let mut refs = BTreeSet::new();

    for (idx, _) in text.match_indices(BLAKE3_PREFIX) {
        let end = idx + BLAKE3_PREFIX.len() + HEX_HASH_LEN;
        if end <= text.len() {
            let candidate = &text[idx..end];
            if is_valid_blake3_hash(candidate) && known_hashes.contains(candidate) {
                refs.insert(candidate.to_string());
            }
        }
    }
    refs
}

/// Scan all files in a directory tree for store hash references.
///
/// Returns the union of references found across all files.
pub fn scan_directory_refs(
    dir: &Path,
    known_hashes: &BTreeSet<String>,
) -> Result<BTreeSet<String>, String> {
    let mut all_refs = BTreeSet::new();
    scan_dir_recursive(dir, known_hashes, &mut all_refs)?;
    Ok(all_refs)
}

/// Check whether a string matches `blake3:<64 hex chars>`.
pub fn is_valid_blake3_hash(s: &str) -> bool {
    if let Some(hex) = s.strip_prefix(BLAKE3_PREFIX) {
        hex.len() == HEX_HASH_LEN && hex.chars().all(|c| c.is_ascii_hexdigit())
    } else {
        false
    }
}

fn scan_dir_recursive(
    dir: &Path,
    known_hashes: &BTreeSet<String>,
    refs: &mut BTreeSet<String>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("read dir {}: {e}", dir.display()))?;
    let mut children: Vec<std::fs::DirEntry> = entries.filter_map(|e| e.ok()).collect();
    children.sort_by_key(|e| e.file_name());

    for entry in children {
        let ft = entry.file_type().map_err(|e| format!("stat: {e}"))?;
        let path = entry.path();
        if ft.is_file() {
            if let Ok(content) = std::fs::read(&path) {
                let file_refs = scan_file_refs(&content, known_hashes);
                refs.extend(file_refs);
            }
        } else if ft.is_dir() {
            scan_dir_recursive(&path, known_hashes, refs)?;
        }
    }
    Ok(())
}
