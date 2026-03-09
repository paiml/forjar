//! Coverage tests for iso_export.rs — hash_file_blake3, compute_root_hash.

use super::iso_export::*;

// ── hash_file_blake3 ─────────────────────────────────────────────

#[test]
fn hash_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("test.txt");
    std::fs::write(&f, "hello world").unwrap();
    let hash = hash_file_blake3(&f);
    assert_eq!(hash.len(), 64);
    assert_ne!(hash, "0".repeat(64));
}

#[test]
fn hash_nonexistent_file() {
    let hash = hash_file_blake3(std::path::Path::new("/tmp/forjar-nonexistent-xyz.txt"));
    assert_eq!(hash, "0".repeat(64));
}

#[test]
fn hash_empty_file() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("empty.txt");
    std::fs::write(&f, "").unwrap();
    let hash = hash_file_blake3(&f);
    assert_eq!(hash.len(), 64);
    assert_ne!(hash, "0".repeat(64)); // empty file still has a hash
}

#[test]
fn hash_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("det.txt");
    std::fs::write(&f, "deterministic").unwrap();
    let h1 = hash_file_blake3(&f);
    let h2 = hash_file_blake3(&f);
    assert_eq!(h1, h2);
}

// ── compute_root_hash ────────────────────────────────────────────

#[test]
fn root_hash_empty() {
    let files: Vec<IsoFile> = vec![];
    let hash = compute_root_hash(&files);
    assert_eq!(hash.len(), 64);
}

#[test]
fn root_hash_single() {
    let files = vec![IsoFile {
        path: "test.txt".to_string(),
        size: 11,
        blake3: "a".repeat(64),
        category: "config".to_string(),
    }];
    let hash = compute_root_hash(&files);
    assert_eq!(hash.len(), 64);
}

#[test]
fn root_hash_multiple() {
    let files = vec![
        IsoFile {
            path: "a.txt".to_string(),
            size: 5,
            blake3: "a".repeat(64),
            category: "config".to_string(),
        },
        IsoFile {
            path: "b.txt".to_string(),
            size: 10,
            blake3: "b".repeat(64),
            category: "state".to_string(),
        },
    ];
    let hash = compute_root_hash(&files);
    assert_eq!(hash.len(), 64);
}

#[test]
fn root_hash_deterministic() {
    let files = vec![IsoFile {
        path: "x.txt".to_string(),
        size: 1,
        blake3: "c".repeat(64),
        category: "binary".to_string(),
    }];
    let h1 = compute_root_hash(&files);
    let h2 = compute_root_hash(&files);
    assert_eq!(h1, h2);
}

#[test]
fn root_hash_order_matters() {
    let files_ab = vec![
        IsoFile { path: "a".to_string(), size: 1, blake3: "1".repeat(64), category: "c".to_string() },
        IsoFile { path: "b".to_string(), size: 2, blake3: "2".repeat(64), category: "c".to_string() },
    ];
    let files_ba = vec![
        IsoFile { path: "b".to_string(), size: 2, blake3: "2".repeat(64), category: "c".to_string() },
        IsoFile { path: "a".to_string(), size: 1, blake3: "1".repeat(64), category: "c".to_string() },
    ];
    let h_ab = compute_root_hash(&files_ab);
    let h_ba = compute_root_hash(&files_ba);
    assert_ne!(h_ab, h_ba);
}
