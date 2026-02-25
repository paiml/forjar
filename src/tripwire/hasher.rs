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

    #[test]
    fn test_fj014_hash_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.txt");
        std::fs::write(&path, "").unwrap();
        let h = hash_file(&path).unwrap();
        assert!(h.starts_with("blake3:"));
        assert_eq!(h.len(), 71); // prefix + 64 hex
    }

    #[test]
    fn test_fj014_hash_empty_string() {
        let h = hash_string("");
        assert!(h.starts_with("blake3:"));
        assert_eq!(h.len(), 71);
        // Empty string should produce a consistent hash
        let h2 = hash_string("");
        assert_eq!(h, h2);
    }

    #[test]
    fn test_fj014_hash_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let h = hash_directory(dir.path()).unwrap();
        assert!(h.starts_with("blake3:"));
        // Empty dir should have a consistent hash
        let h2 = hash_directory(dir.path()).unwrap();
        assert_eq!(h, h2);
    }

    #[test]
    fn test_fj014_hash_file_vs_string_consistency() {
        // Hashing a file should produce the same result as hashing its content string
        let dir = tempfile::tempdir().unwrap();
        let content = "test content for consistency check";
        let path = dir.path().join("consistency.txt");
        std::fs::write(&path, content).unwrap();
        let file_hash = hash_file(&path).unwrap();
        let string_hash = hash_string(content);
        assert_eq!(
            file_hash, string_hash,
            "file hash should equal string hash of same content"
        );
    }

    #[test]
    fn test_fj014_hash_large_content() {
        // Test streaming hash with content larger than STREAM_BUF_SIZE (64KB)
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("large.bin");
        let content = "x".repeat(100_000); // 100KB > 64KB buffer
        std::fs::write(&path, &content).unwrap();
        let h = hash_file(&path).unwrap();
        assert!(h.starts_with("blake3:"));
        // Verify determinism for large files
        let h2 = hash_file(&path).unwrap();
        assert_eq!(h, h2);
    }

    #[test]
    fn test_fj014_composite_hash_empty() {
        let h = composite_hash(&[]);
        assert!(h.starts_with("blake3:"));
        // Empty composite should be deterministic
        let h2 = composite_hash(&[]);
        assert_eq!(h, h2);
    }

    #[test]
    fn test_fj014_composite_hash_single() {
        let h = composite_hash(&["only-one"]);
        assert!(h.starts_with("blake3:"));
    }

    #[test]
    fn test_fj014_hash_directory_content_change() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("f.txt"), "original").unwrap();
        let h1 = hash_directory(dir.path()).unwrap();
        std::fs::write(dir.path().join("f.txt"), "modified").unwrap();
        let h2 = hash_directory(dir.path()).unwrap();
        assert_ne!(
            h1, h2,
            "directory hash should change when file content changes"
        );
    }

    #[test]
    fn test_fj014_hash_directory_file_added() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "aaa").unwrap();
        let h1 = hash_directory(dir.path()).unwrap();
        std::fs::write(dir.path().join("b.txt"), "bbb").unwrap();
        let h2 = hash_directory(dir.path()).unwrap();
        assert_ne!(h1, h2, "directory hash should change when file is added");
    }

    #[test]
    fn test_fj014_hash_directory_not_found() {
        let result = hash_directory(Path::new("/nonexistent/directory"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot read dir"));
    }

    #[test]
    fn test_fj014_hash_file_exact_buffer_size() {
        // Test file size exactly at STREAM_BUF_SIZE boundary
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("exact.bin");
        let content = "x".repeat(STREAM_BUF_SIZE);
        std::fs::write(&path, &content).unwrap();
        let h = hash_file(&path).unwrap();
        assert!(h.starts_with("blake3:"));
        assert_eq!(h.len(), 71);
    }

    #[test]
    fn test_fj014_hash_directory_deep_nesting() {
        let dir = tempfile::tempdir().unwrap();
        let deep = dir.path().join("a").join("b").join("c");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(deep.join("deep.txt"), "deep content").unwrap();
        let h = hash_directory(dir.path()).unwrap();
        assert!(h.starts_with("blake3:"));
        // Hash should differ from empty dir
        let empty = tempfile::tempdir().unwrap();
        let h_empty = hash_directory(empty.path()).unwrap();
        assert_ne!(h, h_empty);
    }

    #[test]
    fn test_fj014_composite_hash_deterministic() {
        let components = &["a", "b", "c"];
        let h1 = composite_hash(components);
        let h2 = composite_hash(components);
        assert_eq!(h1, h2, "composite_hash must be deterministic");
    }

    #[test]
    fn test_fj014_hash_string_differs_by_single_char() {
        let h1 = hash_string("abc");
        let h2 = hash_string("abd");
        assert_ne!(h1, h2, "single char difference must produce different hash");
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

    // --- FJ-132: Hasher edge case tests ---

    #[test]
    fn test_fj132_hash_string_empty() {
        let h = hash_string("");
        assert!(h.starts_with("blake3:"));
        assert_eq!(h.len(), 71);
    }

    #[test]
    fn test_fj132_hash_string_unicode() {
        let h = hash_string("Hello 世界 🌍");
        assert!(h.starts_with("blake3:"));
        assert_eq!(h.len(), 71);
    }

    #[test]
    fn test_fj132_hash_file_large() {
        // Test with a file larger than the stream buffer
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("large.bin");
        let content = "x".repeat(STREAM_BUF_SIZE * 3 + 42);
        std::fs::write(&path, &content).unwrap();
        let h = hash_file(&path).unwrap();
        assert!(h.starts_with("blake3:"));
        assert_eq!(h.len(), 71);
    }

    #[test]
    fn test_fj132_hash_directory_with_multiple_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "alpha").unwrap();
        std::fs::write(dir.path().join("b.txt"), "beta").unwrap();
        std::fs::write(dir.path().join("c.txt"), "gamma").unwrap();
        let h = hash_directory(dir.path()).unwrap();
        assert!(h.starts_with("blake3:"));

        // Changing one file changes the hash
        std::fs::write(dir.path().join("b.txt"), "beta-changed").unwrap();
        let h2 = hash_directory(dir.path()).unwrap();
        assert_ne!(h, h2, "modifying a file should change directory hash");
    }

    #[test]
    fn test_fj132_composite_hash_empty() {
        let h: String = composite_hash(&[]);
        assert!(h.starts_with("blake3:"));
        assert_eq!(h.len(), 71);
    }

    #[test]
    fn test_fj132_composite_hash_single_element() {
        let h = composite_hash(&["only"]);
        assert!(h.starts_with("blake3:"));
        // Single element composite should differ from hash_string of same value
        // because composite joins with \0
        let h_str = hash_string("only");
        assert_ne!(h, h_str, "composite(x) != hash_string(x) due to separator");
    }

    // --- FJ-036: Hasher determinism and coverage tests ---

    #[test]
    fn test_fj036_hash_desired_state_deterministic() {
        use crate::core::planner::hash_desired_state;
        use crate::core::types::{MachineTarget, Resource, ResourceType};
        use std::collections::HashMap;

        let r = Resource {
            resource_type: ResourceType::Package,
            machine: MachineTarget::Single("m1".to_string()),
            state: Some("present".to_string()),
            depends_on: vec![],
            provider: Some("apt".to_string()),
            packages: vec!["curl".to_string()],
            version: None,
            path: None,
            content: None,
            source: None,
            target: None,
            owner: None,
            group: None,
            mode: None,
            name: None,
            enabled: None,
            restart_on: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
        };
        let h1 = hash_desired_state(&r);
        let h2 = hash_desired_state(&r);
        assert_eq!(h1, h2, "hash_desired_state must be deterministic");
        assert!(h1.starts_with("blake3:"));
    }

    #[test]
    fn test_fj036_hash_desired_state_changes_on_content() {
        use crate::core::planner::hash_desired_state;
        use crate::core::types::{MachineTarget, Resource, ResourceType};
        use std::collections::HashMap;

        let r1 = Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("m1".to_string()),
            state: Some("present".to_string()),
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            path: Some("/etc/app.conf".to_string()),
            content: Some("original content".to_string()),
            source: None,
            target: None,
            owner: None,
            group: None,
            mode: None,
            name: None,
            enabled: None,
            restart_on: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
        };
        let r2 = Resource {
            content: Some("changed content".to_string()),
            ..r1.clone()
        };
        let h1 = hash_desired_state(&r1);
        let h2 = hash_desired_state(&r2);
        assert_ne!(h1, h2, "hash must differ when resource content changes");
    }

    #[test]
    fn test_fj036_hash_directory_empty() {
        let dir = tempfile::tempdir().unwrap();
        let h = hash_directory(dir.path()).unwrap();
        assert!(!h.is_empty(), "hash of empty directory must be non-empty");
        assert!(h.starts_with("blake3:"));
        assert_eq!(h.len(), 71); // "blake3:" + 64 hex chars
    }

    #[test]
    fn test_fj036_hash_string_deterministic() {
        let input = "forjar determinism check";
        let h1 = hash_string(input);
        let h2 = hash_string(input);
        assert_eq!(
            h1, h2,
            "hash_string must produce identical output for same input"
        );
        assert!(h1.starts_with("blake3:"));
    }
}
