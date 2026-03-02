//! Tests for `forjar archive` CLI commands.

#[cfg(test)]
mod tests {
    use crate::cli::store_archive::*;
    use crate::core::store::far::{encode_far, FarManifest, FarFileEntry, FarProvenance};
    use std::fs;
    use tempfile::TempDir;

    fn write_far(dir: &TempDir) -> std::path::PathBuf {
        let manifest = FarManifest {
            name: "test-pkg".to_string(),
            version: "1.0.0".to_string(),
            arch: "x86_64".to_string(),
            store_hash: "blake3:abcd1234".to_string(),
            tree_hash: "blake3:efgh5678".to_string(),
            file_count: 1,
            total_size: 5,
            files: vec![FarFileEntry {
                path: "hello.txt".to_string(),
                size: 5,
                blake3: "blake3:abc".to_string(),
            }],
            provenance: FarProvenance {
                origin_provider: "apt".to_string(),
                origin_ref: Some("nginx".to_string()),
                origin_hash: None,
                created_at: "2026-03-02T10:00:00Z".to_string(),
                generator: "forjar 1.0".to_string(),
            },
            kernel_contracts: None,
        };

        let data = b"hello";
        let hash = blake3::hash(data);
        let chunks = vec![(*hash.as_bytes(), data.to_vec())];

        let far_path = dir.path().join("test.far");
        let writer = fs::File::create(&far_path).unwrap();
        encode_far(&manifest, &chunks, writer).unwrap();
        far_path
    }

    #[test]
    fn test_archive_inspect() {
        let dir = TempDir::new().unwrap();
        let far = write_far(&dir);

        let result = cmd_archive_inspect(&far, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_archive_inspect_json() {
        let dir = TempDir::new().unwrap();
        let far = write_far(&dir);

        let result = cmd_archive_inspect(&far, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_archive_verify() {
        let dir = TempDir::new().unwrap();
        let far = write_far(&dir);

        let result = cmd_archive_verify(&far, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_archive_verify_json() {
        let dir = TempDir::new().unwrap();
        let far = write_far(&dir);

        let result = cmd_archive_verify(&far, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_archive_inspect_bad_file() {
        let dir = TempDir::new().unwrap();
        let bad = dir.path().join("bad.far");
        fs::write(&bad, "not a FAR file").unwrap();

        let result = cmd_archive_inspect(&bad, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_archive_verify_bad_file() {
        let dir = TempDir::new().unwrap();
        let bad = dir.path().join("bad.far");
        fs::write(&bad, "not a FAR file").unwrap();

        let result = cmd_archive_verify(&bad, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_archive_unpack_creates_dir() {
        let dir = TempDir::new().unwrap();
        let far = write_far(&dir);
        let store = dir.path().join("store");
        fs::create_dir_all(&store).unwrap();

        let result = cmd_archive_unpack(&far, &store);
        assert!(result.is_ok());

        let entry_dir = store.join("abcd1234");
        assert!(entry_dir.exists());
    }

    #[test]
    fn test_archive_unpack_already_exists() {
        let dir = TempDir::new().unwrap();
        let far = write_far(&dir);
        let store = dir.path().join("store");
        fs::create_dir_all(store.join("abcd1234")).unwrap();

        let result = cmd_archive_unpack(&far, &store);
        assert!(result.is_ok()); // should succeed silently
    }

    #[test]
    fn test_archive_pack_missing_entry() {
        let dir = TempDir::new().unwrap();
        let store = dir.path().join("store");
        fs::create_dir_all(&store).unwrap();

        let result = cmd_archive_pack("blake3:deadbeef", &store, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_archive_inspect_nonexistent() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("nope.far");

        let result = cmd_archive_inspect(&missing, false);
        assert!(result.is_err());
    }
}
