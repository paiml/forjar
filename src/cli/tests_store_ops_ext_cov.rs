//! Coverage tests for store_ops.rs — human_bytes, list_store_entries,
//! collect_profile_hashes, local_machine.

use std::path::Path;

// ── human_bytes ─────────────────────────────────────────────────

#[test]
fn human_bytes_zero() {
    assert_eq!(super::store_ops::human_bytes(0), "0 B");
}

#[test]
fn human_bytes_small() {
    assert_eq!(super::store_ops::human_bytes(512), "512 B");
}

#[test]
fn human_bytes_1023() {
    assert_eq!(super::store_ops::human_bytes(1023), "1023 B");
}

#[test]
fn human_bytes_1kb() {
    assert_eq!(super::store_ops::human_bytes(1024), "1.0 KB");
}

#[test]
fn human_bytes_100kb() {
    assert_eq!(super::store_ops::human_bytes(102_400), "100.0 KB");
}

#[test]
fn human_bytes_1mb() {
    assert_eq!(super::store_ops::human_bytes(1_048_576), "1.0 MB");
}

#[test]
fn human_bytes_50mb() {
    assert_eq!(super::store_ops::human_bytes(52_428_800), "50.0 MB");
}

// ── list_store_entries ──────────────────────────────────────────

#[test]
fn list_store_entries_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let entries = super::store_ops::list_store_entries(dir.path()).unwrap();
    assert!(entries.is_empty());
}

#[test]
fn list_store_entries_with_hash_dir_no_meta() {
    let dir = tempfile::tempdir().unwrap();
    let hash_dir = dir.path().join("abc123def456");
    std::fs::create_dir_all(&hash_dir).unwrap();
    let entries = super::store_ops::list_store_entries(dir.path()).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, "blake3:abc123def456");
    assert_eq!(entries[0].1, "unknown");
    assert_eq!(entries[0].2, "unknown");
}

#[test]
fn list_store_entries_with_meta() {
    let dir = tempfile::tempdir().unwrap();
    let hash_dir = dir.path().join("abc123");
    std::fs::create_dir_all(&hash_dir).unwrap();
    let meta_yaml = r#"
schema: "1.0"
store_hash: "blake3:abc123"
recipe_hash: "blake3:recipe1"
input_hashes: []
arch: x86_64
provider: apt
created_at: "2026-03-08T12:00:00Z"
generator: "forjar 1.0.0"
references: []
"#;
    std::fs::write(hash_dir.join("meta.yaml"), meta_yaml).unwrap();
    let entries = super::store_ops::list_store_entries(dir.path()).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].1, "apt");
    assert_eq!(entries[0].2, "x86_64");
}

#[test]
fn list_store_entries_skips_gc_roots() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".gc-roots")).unwrap();
    std::fs::create_dir_all(dir.path().join("abc123")).unwrap();
    let entries = super::store_ops::list_store_entries(dir.path()).unwrap();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].0.contains("abc123"));
}

#[test]
fn list_store_entries_skips_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("some-file.txt"), "data").unwrap();
    let entries = super::store_ops::list_store_entries(dir.path()).unwrap();
    assert!(entries.is_empty());
}

#[test]
fn list_store_entries_nonexistent() {
    let result =
        super::store_ops::list_store_entries(Path::new("/tmp/forjar-nonexistent-store-xyz"));
    assert!(result.is_err());
}

// ── collect_profile_hashes ──────────────────────────────────────

#[test]
fn collect_profile_hashes_no_profiles_dir() {
    let dir = tempfile::tempdir().unwrap();
    // No profiles subdirectory
    let store_dir = dir.path().join("store");
    std::fs::create_dir_all(&store_dir).unwrap();
    let hashes = super::store_ops::collect_profile_hashes(&store_dir);
    assert!(hashes.is_empty());
}

// ── collect_lock_hashes ─────────────────────────────────────────

#[test]
fn collect_lock_hashes_no_lockfile() {
    let dir = tempfile::tempdir().unwrap();
    let hashes = super::store_ops::collect_lock_hashes(dir.path());
    assert!(hashes.is_empty());
}

// ── local_machine ───────────────────────────────────────────────

#[test]
fn local_machine_fields() {
    let m = super::store_ops::local_machine();
    assert_eq!(m.hostname, "localhost");
    assert_eq!(m.addr, "127.0.0.1");
    assert_eq!(m.user, "root");
    assert!(!m.arch.is_empty());
}
