//! Tests for FJ-1325/FJ-1326: GC roots and mark-and-sweep.

use super::gc::{collect_roots, mark_and_sweep};
use super::meta::{new_meta, write_meta};
use std::collections::BTreeSet;
use std::path::Path;

const HASH_A: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HASH_C: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

fn setup_store_entry(store_dir: &Path, hash: &str, references: &[&str]) {
    let hex = hash.strip_prefix("blake3:").unwrap();
    let entry_dir = store_dir.join(hex);
    std::fs::create_dir_all(&entry_dir).unwrap();
    let mut meta = new_meta(hash, "blake3:recipe", &[], "x86_64", "test");
    meta.references = references.iter().map(|r| r.to_string()).collect();
    write_meta(&entry_dir, &meta).unwrap();
}

#[test]
fn test_fj1325_collect_roots_profiles() {
    let roots = collect_roots(&[HASH_A.to_string(), HASH_B.to_string()], &[], None);
    assert_eq!(roots.len(), 2);
    assert!(roots.contains(HASH_A));
    assert!(roots.contains(HASH_B));
}

#[test]
fn test_fj1325_collect_roots_lockfile() {
    let roots = collect_roots(&[], &[HASH_A.to_string()], None);
    assert_eq!(roots.len(), 1);
    assert!(roots.contains(HASH_A));
}

#[test]
fn test_fj1325_collect_roots_deduplicates() {
    let roots = collect_roots(&[HASH_A.to_string()], &[HASH_A.to_string()], None);
    assert_eq!(roots.len(), 1);
}

#[test]
fn test_fj1325_collect_roots_gc_dir() {
    let dir = tempfile::tempdir().unwrap();
    let gc_dir = dir.path().join(".gc-roots");
    std::fs::create_dir(&gc_dir).unwrap();
    let hex = HASH_A.strip_prefix("blake3:").unwrap();
    let target = format!("/var/forjar/store/{hex}/content");
    std::os::unix::fs::symlink(&target, gc_dir.join("my-root")).unwrap();

    let roots = collect_roots(&[], &[], Some(&gc_dir));
    assert_eq!(roots.len(), 1);
    assert!(roots.contains(HASH_A));
}

#[test]
fn test_fj1325_collect_roots_empty() {
    let roots = collect_roots(&[], &[], None);
    assert!(roots.is_empty());
}

#[test]
fn test_fj1326_mark_sweep_all_live() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("store");
    std::fs::create_dir(&store).unwrap();
    setup_store_entry(&store, HASH_A, &[]);
    setup_store_entry(&store, HASH_B, &[]);

    let roots: BTreeSet<String> = [HASH_A, HASH_B].iter().map(|s| s.to_string()).collect();
    let report = mark_and_sweep(&roots, &store).unwrap();
    assert_eq!(report.live.len(), 2);
    assert!(report.dead.is_empty());
    assert_eq!(report.total, 2);
}

#[test]
fn test_fj1326_mark_sweep_dead_entry() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("store");
    std::fs::create_dir(&store).unwrap();
    setup_store_entry(&store, HASH_A, &[]);
    setup_store_entry(&store, HASH_B, &[]); // B is not a root

    let roots: BTreeSet<String> = [HASH_A].iter().map(|s| s.to_string()).collect();
    let report = mark_and_sweep(&roots, &store).unwrap();
    assert_eq!(report.live.len(), 1);
    assert_eq!(report.dead.len(), 1);
    assert!(report.dead.contains(HASH_B));
}

#[test]
fn test_fj1326_mark_sweep_follows_references() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("store");
    std::fs::create_dir(&store).unwrap();
    setup_store_entry(&store, HASH_A, &[HASH_B]); // A references B
    setup_store_entry(&store, HASH_B, &[HASH_C]); // B references C
    setup_store_entry(&store, HASH_C, &[]);

    let roots: BTreeSet<String> = [HASH_A].iter().map(|s| s.to_string()).collect();
    let report = mark_and_sweep(&roots, &store).unwrap();
    // All three are live via reference chain
    assert_eq!(report.live.len(), 3);
    assert!(report.dead.is_empty());
}

#[test]
fn test_fj1326_mark_sweep_empty_store() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("store");
    std::fs::create_dir(&store).unwrap();

    let roots = BTreeSet::new();
    let report = mark_and_sweep(&roots, &store).unwrap();
    assert!(report.live.is_empty());
    assert!(report.dead.is_empty());
    assert_eq!(report.total, 0);
}

#[test]
fn test_fj1326_mark_sweep_store_not_found() {
    let roots = BTreeSet::new();
    let result = mark_and_sweep(&roots, Path::new("/no/such/store"));
    assert!(result.is_err());
}

#[test]
fn test_fj1326_mark_sweep_missing_meta_still_works() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("store");
    std::fs::create_dir(&store).unwrap();
    // Create entry directory without meta.yaml
    let hex = HASH_A.strip_prefix("blake3:").unwrap();
    std::fs::create_dir(store.join(hex)).unwrap();

    let roots: BTreeSet<String> = [HASH_A].iter().map(|s| s.to_string()).collect();
    let report = mark_and_sweep(&roots, &store).unwrap();
    // A is live (it's a root), just no references to follow
    assert_eq!(report.live.len(), 1);
}

#[test]
fn test_fj1325_gc_root_from_symlink_integration() {
    // Integration test: symlink target → collected root → mark live
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("store");
    std::fs::create_dir(&store).unwrap();
    setup_store_entry(&store, HASH_A, &[]);

    let gc_dir = dir.path().join(".gc-roots");
    std::fs::create_dir(&gc_dir).unwrap();
    let hex_a = HASH_A.strip_prefix("blake3:").unwrap();
    let target = format!("/var/forjar/store/{hex_a}/content");
    std::os::unix::fs::symlink(&target, gc_dir.join("root")).unwrap();

    let roots = collect_roots(&[], &[], Some(&gc_dir));
    let report = mark_and_sweep(&roots, &store).unwrap();
    assert!(report.live.contains(HASH_A));
}
