//! FJ-014: BLAKE3 state hashing for resources, files, and directories.

use provable_contracts_macros::contract;
use std::io::Read;
use std::path::Path;

const STREAM_BUF_SIZE: usize = 65536;

/// Hash a file's contents. Returns `"blake3:{hex}"`.
#[contract("blake3-state-v1", equation = "hash_file")]
pub fn hash_file(path: &Path) -> Result<String, String> {
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
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

/// Hash a string. Returns `"blake3:{hex}"`.
#[contract("blake3-state-v1", equation = "hash_string")]
pub fn hash_string(s: &str) -> String {
    format!("blake3:{}", blake3::hash(s.as_bytes()).to_hex())
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
            let ft = entry
                .file_type()
                .map_err(|e| format!("stat error: {}", e))?;
            if ft.is_symlink() {
                continue;
            }
            let path = entry.path();
            let rel = path
                .strip_prefix(base)
                .map_err(|e| format!("path prefix error: {}", e))?
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
#[contract("blake3-state-v1", equation = "composite_hash")]
pub fn composite_hash(components: &[&str]) -> String {
    let mut hasher = blake3::Hasher::new();
    for c in components {
        hasher.update(c.as_bytes());
        hasher.update(b"\0");
    }
    format!("blake3:{}", hasher.finalize().to_hex())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_fj014_hash_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "hello world").unwrap();
        let h = hash_file(&path).unwrap();
        assert!(h.starts_with("blake3:"));
        assert_eq!(h.len(), 7 + 64); // "blake3:" + 64 hex chars
    }

    #[test]
    fn test_fj014_hash_file_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("det.txt");
        std::fs::write(&path, "deterministic").unwrap();
        let h1 = hash_file(&path).unwrap();
        let h2 = hash_file(&path).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_fj014_hash_string() {
        let h1 = hash_string("hello");
        let h2 = hash_string("hello");
        let h3 = hash_string("world");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
        assert!(h1.starts_with("blake3:"));
    }

    #[test]
    fn test_fj014_hash_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "aaa").unwrap();
        std::fs::write(dir.path().join("b.txt"), "bbb").unwrap();
        let h = hash_directory(dir.path()).unwrap();
        assert!(h.starts_with("blake3:"));
    }

    #[test]
    fn test_fj014_hash_directory_order_independent_of_creation() {
        // Same files, deterministic hash regardless of creation order
        let d1 = tempfile::tempdir().unwrap();
        std::fs::write(d1.path().join("b.txt"), "bbb").unwrap();
        std::fs::write(d1.path().join("a.txt"), "aaa").unwrap();

        let d2 = tempfile::tempdir().unwrap();
        std::fs::write(d2.path().join("a.txt"), "aaa").unwrap();
        std::fs::write(d2.path().join("b.txt"), "bbb").unwrap();

        assert_eq!(
            hash_directory(d1.path()).unwrap(),
            hash_directory(d2.path()).unwrap()
        );
    }

    #[test]
    fn test_fj014_composite_hash() {
        let h = composite_hash(&["blake3:aaa", "blake3:bbb"]);
        assert!(h.starts_with("blake3:"));
        // Different inputs → different hash
        let h2 = composite_hash(&["blake3:bbb", "blake3:aaa"]);
        assert_ne!(h, h2);
    }

    #[test]
    fn test_fj014_hash_file_not_found() {
        let result = hash_file(Path::new("/nonexistent/file.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_fj014_hash_directory_with_symlink_and_subdirs() {
        let dir = tempfile::tempdir().unwrap();
        // File in root
        std::fs::write(dir.path().join("root.txt"), "root").unwrap();
        // Subdirectory with file
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub").join("nested.txt"), "nested").unwrap();
        // Symlink — should be skipped
        #[cfg(unix)]
        std::os::unix::fs::symlink(dir.path().join("root.txt"), dir.path().join("link.txt"))
            .unwrap();

        let h = hash_directory(dir.path()).unwrap();
        assert!(h.starts_with("blake3:"));

        // Verify symlink doesn't affect hash: remove symlink, hash should be same
        #[cfg(unix)]
        {
            let h_with_link = h.clone();
            std::fs::remove_file(dir.path().join("link.txt")).unwrap();
            let h_without_link = hash_directory(dir.path()).unwrap();
            assert_eq!(
                h_with_link, h_without_link,
                "symlink should not affect hash"
            );
        }
    }

    // ── Falsification tests (BLAKE3 State Contract) ─────────────

    proptest! {
        /// FALSIFY-B3-001: hash_string always produces "blake3:" prefix + 64 hex chars.
        #[test]
        fn falsify_b3_001_hash_string_prefix_format(s in ".*") {
            let h = hash_string(&s);
            prop_assert!(h.starts_with("blake3:"), "missing blake3: prefix");
            prop_assert_eq!(h.len(), 71, "expected 7 prefix + 64 hex = 71 chars");
        }

        /// FALSIFY-B3-002: hash_string is deterministic.
        #[test]
        fn falsify_b3_002_hash_string_determinism(s in ".*") {
            let h1 = hash_string(&s);
            let h2 = hash_string(&s);
            prop_assert_eq!(h1, h2, "hash_string must be deterministic");
        }

        /// FALSIFY-B3-003: composite_hash is order-sensitive.
        #[test]
        fn falsify_b3_003_composite_order_sensitivity(a in "[a-z]{1,8}", b in "[a-z]{1,8}") {
            prop_assume!(a != b);
            let h_ab = composite_hash(&[&a, &b]);
            let h_ba = composite_hash(&[&b, &a]);
            prop_assert_ne!(h_ab, h_ba, "composite_hash must be order-sensitive");
        }
    }
}
