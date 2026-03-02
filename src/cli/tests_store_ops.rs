//! Tests for `forjar store` CLI commands (gc, list, diff, sync).

#[cfg(test)]
mod tests {
    use crate::cli::store_ops::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_store_with_meta(dir: &TempDir) -> std::path::PathBuf {
        let store = dir.path().join("store");
        fs::create_dir_all(&store).unwrap();

        let e1 = store.join("aaaa111122223333444455556666777788889999000011112222333344445555");
        fs::create_dir_all(e1.join("content")).unwrap();
        fs::write(
            e1.join("meta.yaml"),
            r#"
schema: "1.0"
store_hash: "blake3:aaaa111122223333444455556666777788889999000011112222333344445555"
recipe_hash: "blake3:0000"
input_hashes: []
arch: "x86_64"
provider: "apt"
created_at: "2026-03-02T10:00:00Z"
generator: "forjar 1.0"
references: []
provenance:
  origin_provider: "apt"
  origin_ref: "nginx"
  origin_hash: "sha256:abc123"
  derived_from: null
  derivation_depth: 0
"#,
        )
        .unwrap();

        store
    }

    #[test]
    fn test_store_gc_empty() {
        let dir = TempDir::new().unwrap();
        let store = dir.path().join("store");
        fs::create_dir_all(&store).unwrap();
        let state = dir.path().join("state");
        fs::create_dir_all(&state).unwrap();

        let result = cmd_store_gc(&store, &state, true, None, 5, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_store_gc_dry_run() {
        let dir = TempDir::new().unwrap();
        let store = make_store_with_meta(&dir);
        let state = dir.path().join("state");
        fs::create_dir_all(&state).unwrap();

        let result = cmd_store_gc(&store, &state, true, None, 5, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_store_gc_json() {
        let dir = TempDir::new().unwrap();
        let store = make_store_with_meta(&dir);
        let state = dir.path().join("state");
        fs::create_dir_all(&state).unwrap();

        let result = cmd_store_gc(&store, &state, true, None, 5, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_store_list_empty() {
        let dir = TempDir::new().unwrap();
        let store = dir.path().join("store");
        fs::create_dir_all(&store).unwrap();

        let result = cmd_store_list(&store, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_store_list_with_provider() {
        let dir = TempDir::new().unwrap();
        let store = make_store_with_meta(&dir);

        let result = cmd_store_list(&store, true, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_store_list_json() {
        let dir = TempDir::new().unwrap();
        let store = make_store_with_meta(&dir);

        let result = cmd_store_list(&store, true, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_store_diff() {
        let dir = TempDir::new().unwrap();
        let store = make_store_with_meta(&dir);
        let hash = "blake3:aaaa111122223333444455556666777788889999000011112222333344445555";

        let result = cmd_store_diff(hash, &store, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_store_diff_json() {
        let dir = TempDir::new().unwrap();
        let store = make_store_with_meta(&dir);
        let hash = "blake3:aaaa111122223333444455556666777788889999000011112222333344445555";

        let result = cmd_store_diff(hash, &store, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_store_diff_missing() {
        let dir = TempDir::new().unwrap();
        let store = dir.path().join("store");
        fs::create_dir_all(&store).unwrap();

        let result = cmd_store_diff("blake3:deadbeef", &store, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_store_sync_dry_run() {
        let dir = TempDir::new().unwrap();
        let store = make_store_with_meta(&dir);
        let hash = "blake3:aaaa111122223333444455556666777788889999000011112222333344445555";

        let result = cmd_store_sync(hash, &store, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_store_sync_json() {
        let dir = TempDir::new().unwrap();
        let store = make_store_with_meta(&dir);
        let hash = "blake3:aaaa111122223333444455556666777788889999000011112222333344445555";

        let result = cmd_store_sync(hash, &store, false, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_store_list_nonexistent() {
        let dir = TempDir::new().unwrap();
        let store = dir.path().join("no-store");

        let result = cmd_store_list(&store, false, false);
        assert!(result.is_err());
    }
}
