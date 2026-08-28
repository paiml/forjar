//! GH-236: unit tests for store verification.

use super::content::content_hash;
use super::meta::{new_meta, seal_output, write_meta, Addressing};
use super::verify::{verify_entry, verify_store, EntryStatus};
use std::path::Path;

/// Build a sealed entry holding `bytes`, and return its directory.
fn sealed_entry(store: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let entry = store.join(name);
    std::fs::create_dir_all(entry.join("content")).unwrap();
    std::fs::write(entry.join("content/out.bin"), bytes).unwrap();
    let mut meta = new_meta(
        &format!("blake3:{name}"),
        "blake3:recipe",
        &[],
        "x86_64",
        "apt",
    );
    seal_output(&entry, &mut meta, Addressing::Content).unwrap();
    write_meta(&entry, &meta).unwrap();
    entry
}

#[test]
fn a_sealed_entry_that_was_not_touched_verifies() {
    let dir = tempfile::tempdir().unwrap();
    let entry = sealed_entry(dir.path(), "a".repeat(64).as_str(), b"bytes");
    assert_eq!(verify_entry(&entry).status, EntryStatus::Ok);
}

#[test]
fn an_entry_with_no_meta_is_malformed_not_ok() {
    // `write_meta` is part of every entry's creation, so an entry without it
    // was never finished. Reporting that as `Ok` would be the same class of
    // lie the whole issue is about.
    let dir = tempfile::tempdir().unwrap();
    let entry = dir.path().join("b".repeat(64));
    std::fs::create_dir_all(entry.join("content")).unwrap();
    std::fs::write(entry.join("content/out.bin"), b"orphan").unwrap();
    assert!(matches!(
        verify_entry(&entry).status,
        EntryStatus::Malformed(_)
    ));
}

#[test]
fn a_sealed_entry_whose_content_vanished_is_malformed() {
    let dir = tempfile::tempdir().unwrap();
    let entry = sealed_entry(dir.path(), "c".repeat(64).as_str(), b"bytes");
    std::fs::remove_dir_all(entry.join("content")).unwrap();
    assert!(matches!(
        verify_entry(&entry).status,
        EntryStatus::Malformed(_)
    ));
}

#[test]
fn verify_store_skips_gc_roots_and_plain_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".gc-roots")).unwrap();
    std::fs::write(dir.path().join("README"), "not an entry").unwrap();
    sealed_entry(dir.path(), "d".repeat(64).as_str(), b"bytes");

    let verdicts = verify_store(dir.path()).unwrap();
    assert_eq!(verdicts.len(), 1, "{verdicts:?}");
    assert_eq!(verdicts[0].status, EntryStatus::Ok);
}

#[test]
fn a_missing_store_dir_is_an_empty_store_not_an_error() {
    // GH-239 precedent: the store is created by the first import, not by
    // asking about it. `forjar store verify` on a fresh machine must not fail.
    let dir = tempfile::tempdir().unwrap();
    let absent = dir.path().join("never-created");
    assert_eq!(verify_store(&absent).unwrap(), Vec::new());
}

#[test]
fn a_corrupted_entry_is_a_failure_and_an_unsealed_one_is_not() {
    let dir = tempfile::tempdir().unwrap();

    let corrupt = sealed_entry(dir.path(), "e".repeat(64).as_str(), b"GOOD");
    std::fs::write(corrupt.join("content/out.bin"), b"BAD!").unwrap();

    let legacy = dir.path().join("f".repeat(64));
    std::fs::create_dir_all(legacy.join("content")).unwrap();
    std::fs::write(legacy.join("content/out.bin"), b"legacy").unwrap();
    let meta = new_meta("blake3:f", "blake3:recipe", &[], "x86_64", "apt");
    write_meta(&legacy, &meta).unwrap();

    assert!(verify_entry(&corrupt).is_failure());
    assert_eq!(verify_entry(&legacy).status, EntryStatus::Unsealed);
    assert!(
        !verify_entry(&legacy).is_failure(),
        "a store written by an older forjar must not turn a CI gate red"
    );
}

#[test]
fn content_hash_refuses_a_directory_that_is_not_there() {
    let dir = tempfile::tempdir().unwrap();
    assert!(content_hash(&dir.path().join("content")).is_err());
}
