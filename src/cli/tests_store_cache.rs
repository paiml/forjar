//! Tests for `forjar cache` CLI commands.

#[cfg(test)]
mod tests {
    use crate::cli::store_cache::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_store(dir: &TempDir) -> std::path::PathBuf {
        let store = dir.path().join("store");
        fs::create_dir_all(&store).unwrap();

        // Create two fake store entries
        let e1 = store.join("aabbccdd00112233445566778899aabbccddeeff0011223344556677889900aa");
        fs::create_dir_all(e1.join("content")).unwrap();
        fs::write(e1.join("content/hello.txt"), "hello").unwrap();
        fs::write(e1.join("meta.yaml"), r#"
schema: "1.0"
store_hash: "blake3:aabbccdd00112233445566778899aabbccddeeff0011223344556677889900aa"
recipe_hash: "blake3:0000"
input_hashes: []
arch: "x86_64"
provider: "apt"
created_at: "2026-03-02T10:00:00Z"
generator: "forjar 1.0"
references: []
"#).unwrap();

        let e2 = store.join("1122334455667788990011223344556677889900aabbccddeeff00112233aabb");
        fs::create_dir_all(e2.join("content")).unwrap();
        fs::write(e2.join("content/tool"), "bin").unwrap();
        fs::write(e2.join("meta.yaml"), r#"
schema: "1.0"
store_hash: "blake3:1122334455667788990011223344556677889900aabbccddeeff00112233aabb"
recipe_hash: "blake3:1111"
input_hashes: []
arch: "x86_64"
provider: "cargo"
created_at: "2026-03-02T11:00:00Z"
generator: "forjar 1.0"
references: []
"#).unwrap();

        store
    }

    #[test]
    fn test_cache_list_empty() {
        let dir = TempDir::new().unwrap();
        let store = dir.path().join("store");
        fs::create_dir_all(&store).unwrap();

        let result = cmd_cache_list(&store, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cache_list_with_entries() {
        let dir = TempDir::new().unwrap();
        let store = make_store(&dir);

        let result = cmd_cache_list(&store, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cache_list_json() {
        let dir = TempDir::new().unwrap();
        let store = make_store(&dir);

        let result = cmd_cache_list(&store, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cache_push_dry_run() {
        let dir = TempDir::new().unwrap();
        let store = make_store(&dir);

        let result = cmd_cache_push("forjar@cache.internal:/var/cache", &store, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cache_push_specific_hash() {
        let dir = TempDir::new().unwrap();
        let store = make_store(&dir);

        let result = cmd_cache_push(
            "forjar@cache.internal:/var/cache",
            &store,
            Some("aabbcc"),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_cache_push_invalid_remote() {
        let dir = TempDir::new().unwrap();
        let store = make_store(&dir);

        let result = cmd_cache_push("not-a-remote", &store, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_pull_already_exists() {
        let dir = TempDir::new().unwrap();
        let store = make_store(&dir);
        let hash = "blake3:aabbccdd00112233445566778899aabbccddeeff0011223344556677889900aa";

        let result = cmd_cache_pull(hash, &store);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cache_pull_missing() {
        let dir = TempDir::new().unwrap();
        let store = dir.path().join("store");
        fs::create_dir_all(&store).unwrap();

        let result = cmd_cache_pull("blake3:deadbeef", &store);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cache_verify_empty() {
        let dir = TempDir::new().unwrap();
        let store = dir.path().join("store");
        fs::create_dir_all(&store).unwrap();

        let result = cmd_cache_verify(&store, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cache_verify_json() {
        let dir = TempDir::new().unwrap();
        let store = dir.path().join("store");
        fs::create_dir_all(&store).unwrap();

        let result = cmd_cache_verify(&store, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cache_list_nonexistent_dir() {
        let dir = TempDir::new().unwrap();
        let store = dir.path().join("no-such-dir");

        let result = cmd_cache_list(&store, false);
        assert!(result.is_err());
    }
}
