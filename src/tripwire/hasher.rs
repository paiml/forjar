//! FJ-014: BLAKE3 state hashing for resources, files, and directories.

use provable_contracts_macros::contract;
use std::io::Read;
use std::path::Path;

pub(crate) const STREAM_BUF_SIZE: usize = 65536;

/// Hash a file's contents. Returns `"blake3:{hex}"`.
#[contract("blake3-state-v1", equation = "hash_file")]
pub fn hash_file(path: &Path) -> Result<String, String> {
    // Contract: blake3-state-v1.yaml precondition (pv codegen)
    contract_pre_hash_file!(path);
    let mut file =
        std::fs::File::open(path).map_err(|e| format!("cannot open {}: {}", path.display(), e))?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; STREAM_BUF_SIZE];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("read error {}: {}", path.display(), e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let result = format!("blake3:{}", hasher.finalize().to_hex());
    // FJ-2200: Determinism — hash always starts with "blake3:" prefix and has 64 hex chars
    debug_assert!(result.starts_with("blake3:"), "hash_file: missing prefix");
    debug_assert_eq!(result.len(), 71, "hash_file: unexpected length");
    contract_post_configuration!(&result);
    Ok(result)
}

/// Hash a string. Returns `"blake3:{hex}"`.
///
/// FJ-2200: Contract — determinism: same input always produces same hash.
///
/// # Panics
///
/// Panics in debug builds if `s.is_empty()` — the `aprender-contracts`
/// `blake3-state-v1` precondition forbids empty input. Callers that may
/// legitimately hash empty data (e.g. drift stdout capture, optional script
/// logging) must use [`hash_string_or_sentinel`] instead.
#[contract("blake3-state-v1", equation = "hash_string")]
pub fn hash_string(s: &str) -> String {
    // Contract: blake3-state-v1.yaml precondition (pv codegen)
    contract_pre_hash_string!(s.as_bytes());
    let result = format!("blake3:{}", blake3::hash(s.as_bytes()).to_hex());
    debug_assert!(result.starts_with("blake3:"), "hash_string: missing prefix");
    debug_assert_eq!(result.len(), 71, "hash_string: unexpected length");
    contract_post_configuration!(&result);
    result
}

/// Hash a string, or return a deterministic "empty" sentinel hash if the
/// input is empty.
///
/// Use this at call sites where the input is arbitrary text that may
/// legitimately be empty — e.g. hashing command stdout for drift detection
/// when the queried file doesn't exist yet (stdout is `""`), or hashing an
/// optional script that a resource didn't provide.
///
/// The STRONG `aprender-contracts blake3-state-v1` precondition forbids
/// empty input to [`hash_string`]. This wrapper upholds the contract by
/// routing empty inputs through a fixed non-empty sentinel
/// (`"sentinel:empty-input-v1"`) while still producing a deterministic,
/// prefixed BLAKE3 output that looks identical to any other hash.
pub fn hash_string_or_sentinel(s: &str) -> String {
    if s.is_empty() {
        // Distinct from any real payload; satisfies `!input.is_empty()`
        // precondition of the underlying `hash_string` while keeping
        // `empty_input` a deterministic, recognisable hash identity.
        return hash_string("sentinel:empty-input-v1");
    }
    hash_string(s)
}

/// Hash a directory (sorted walk, relative paths included in hash).
/// Skips symlinks.
pub fn hash_directory(path: &Path) -> Result<String, String> {
    let mut entries: Vec<(String, String)> = Vec::new();

    fn walk(
        base: &Path,
        current: &Path,
        entries: &mut Vec<(String, String)>,
    ) -> Result<(), String> {
        let read_dir = std::fs::read_dir(current)
            .map_err(|e| format!("cannot read dir {}: {}", current.display(), e))?;
        let mut children: Vec<std::fs::DirEntry> = read_dir.filter_map(|e| e.ok()).collect();
        children.sort_by_key(|e| e.file_name());

        for entry in children {
            let ft = entry.file_type().map_err(|e| format!("stat error: {e}"))?;
            if ft.is_symlink() {
                continue;
            }
            let path = entry.path();
            let rel = path
                .strip_prefix(base)
                .map_err(|e| format!("path prefix error: {e}"))?
                .to_string_lossy()
                .to_string();
            if ft.is_file() {
                let hash = hash_file(&path)?;
                entries.push((rel, hash));
            } else if ft.is_dir() {
                walk(base, &path, entries)?;
            }
        }
        Ok(())
    }

    walk(path, path, &mut entries)?;

    let mut hasher = blake3::Hasher::new();
    for (rel, hash) in &entries {
        hasher.update(rel.as_bytes());
        hasher.update(b"\0");
        hasher.update(hash.as_bytes());
        hasher.update(b"\n");
    }
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

/// Compute a composite hash from multiple component hashes.
///
/// FJ-2200: Contract — determinism: same components always produce same hash.
#[contract("blake3-state-v1", equation = "composite_hash")]
pub fn composite_hash(components: &[&str]) -> String {
    // Contract: blake3-state-v1.yaml precondition (pv codegen)
    contract_pre_composite_hash!(components);
    let mut hasher = blake3::Hasher::new();
    for c in components {
        hasher.update(c.as_bytes());
        hasher.update(b"\0");
    }
    let result = format!("blake3:{}", hasher.finalize().to_hex());
    debug_assert!(
        result.starts_with("blake3:"),
        "composite_hash: missing prefix"
    );
    debug_assert_eq!(result.len(), 71, "composite_hash: unexpected length");
    result
}
