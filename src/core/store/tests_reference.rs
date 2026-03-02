//! Tests for FJ-1304: Reference scanning for store paths.

use super::reference::{is_valid_blake3_hash, scan_directory_refs, scan_file_refs};
use std::collections::BTreeSet;

fn known_set(hashes: &[&str]) -> BTreeSet<String> {
    hashes.iter().map(|s| s.to_string()).collect()
}

const HASH_A: &str = "blake3:a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";
const HASH_B: &str = "blake3:1111111111111111111111111111111111111111111111111111111111111111";
const HASH_C: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn test_fj1304_valid_blake3_hash() {
    assert!(is_valid_blake3_hash(HASH_A));
    assert!(is_valid_blake3_hash(HASH_B));
    assert!(is_valid_blake3_hash(HASH_C));
}

#[test]
fn test_fj1304_invalid_blake3_hash() {
    assert!(!is_valid_blake3_hash("blake3:short"));
    assert!(!is_valid_blake3_hash(
        "sha256:a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2"
    ));
    assert!(!is_valid_blake3_hash(
        "blake3:zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"
    ));
    assert!(!is_valid_blake3_hash(""));
    assert!(!is_valid_blake3_hash("blake3:"));
}

#[test]
fn test_fj1304_scan_file_finds_known_refs() {
    let content = format!("some text {HASH_A} more text {HASH_B} end");
    let known = known_set(&[HASH_A, HASH_B]);
    let refs = scan_file_refs(content.as_bytes(), &known);
    assert_eq!(refs.len(), 2);
    assert!(refs.contains(HASH_A));
    assert!(refs.contains(HASH_B));
}

#[test]
fn test_fj1304_scan_file_ignores_unknown_hashes() {
    let content = format!("ref to {HASH_A} and {HASH_C}");
    let known = known_set(&[HASH_A]); // HASH_C not known
    let refs = scan_file_refs(content.as_bytes(), &known);
    assert_eq!(refs.len(), 1);
    assert!(refs.contains(HASH_A));
    assert!(!refs.contains(HASH_C));
}

#[test]
fn test_fj1304_scan_file_no_matches() {
    let content = b"no hashes here just plain text";
    let known = known_set(&[HASH_A]);
    let refs = scan_file_refs(content, &known);
    assert!(refs.is_empty());
}

#[test]
fn test_fj1304_scan_file_deduplicates() {
    let content = format!("{HASH_A} repeat {HASH_A} again {HASH_A}");
    let known = known_set(&[HASH_A]);
    let refs = scan_file_refs(content.as_bytes(), &known);
    assert_eq!(refs.len(), 1);
}

#[test]
fn test_fj1304_scan_file_binary_content() {
    let mut content = vec![0u8; 100];
    let hash_bytes = HASH_A.as_bytes();
    content[10..10 + hash_bytes.len()].copy_from_slice(hash_bytes);
    let known = known_set(&[HASH_A]);
    let refs = scan_file_refs(&content, &known);
    assert_eq!(refs.len(), 1);
}

#[test]
fn test_fj1304_scan_directory_refs() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("file1.txt"), format!("link to {HASH_A}")).unwrap();
    std::fs::write(dir.path().join("file2.txt"), format!("link to {HASH_B}")).unwrap();
    let known = known_set(&[HASH_A, HASH_B]);
    let refs = scan_directory_refs(dir.path(), &known).unwrap();
    assert_eq!(refs.len(), 2);
}

#[test]
fn test_fj1304_scan_directory_recursive() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(sub.join("nested.txt"), format!("ref {HASH_C}")).unwrap();
    let known = known_set(&[HASH_C]);
    let refs = scan_directory_refs(dir.path(), &known).unwrap();
    assert_eq!(refs.len(), 1);
    assert!(refs.contains(HASH_C));
}

#[test]
fn test_fj1304_scan_directory_empty() {
    let dir = tempfile::tempdir().unwrap();
    let known = known_set(&[HASH_A]);
    let refs = scan_directory_refs(dir.path(), &known).unwrap();
    assert!(refs.is_empty());
}

#[test]
fn test_fj1304_scan_directory_not_found() {
    let known = known_set(&[HASH_A]);
    let result = scan_directory_refs(std::path::Path::new("/no/such/dir"), &known);
    assert!(result.is_err());
}

#[test]
fn test_fj1304_scan_file_adjacent_hashes() {
    let content = format!("{HASH_A}{HASH_B}");
    let known = known_set(&[HASH_A, HASH_B]);
    let refs = scan_file_refs(content.as_bytes(), &known);
    assert_eq!(refs.len(), 2);
}
