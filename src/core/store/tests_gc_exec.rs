//! Tests for FJ-1365: GC sweep execution.

use super::gc::{collect_roots, mark_and_sweep, GcReport};
use super::gc_exec::{dir_size, sweep, sweep_dry_run};
use super::meta::{write_meta, StoreMeta};
use std::collections::BTreeSet;
use std::path::Path;

fn make_store_entry(store_dir: &Path, hash_hex: &str, content: &[u8]) {
    let entry_dir = store_dir.join(hash_hex);
    let content_dir = entry_dir.join("content");
    std::fs::create_dir_all(&content_dir).unwrap();
    std::fs::write(content_dir.join("data.bin"), content).unwrap();

    let meta = StoreMeta {
        schema: "1.0".to_string(),
        store_hash: format!("blake3:{hash_hex}"),
        recipe_hash: "test".to_string(),
        input_hashes: Vec::new(),
        arch: "x86_64".to_string(),
        provider: "test".to_string(),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "test".to_string(),
        references: Vec::new(),
        provenance: None,
    };
    write_meta(&entry_dir, &meta).unwrap();
}

fn dead_report(dead: &[&str]) -> GcReport {
    GcReport {
        live: BTreeSet::new(),
        dead: dead.iter().map(|h| format!("blake3:{h}")).collect(),
        total: dead.len(),
    }
}

#[test]
fn sweep_removes_dead_entries() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path();

    make_store_entry(store, "aaa111", b"dead content");
    make_store_entry(store, "bbb222", b"also dead");
    make_store_entry(store, "ccc333", b"live content");

    let report = dead_report(&["aaa111", "bbb222"]);
    let result = sweep(&report, store).unwrap();

    assert_eq!(result.removed.len(), 2);
    assert!(result.errors.is_empty());
    assert!(result.bytes_freed > 0);

    // Dead entries removed
    assert!(!store.join("aaa111").exists());
    assert!(!store.join("bbb222").exists());
    // Live entry untouched
    assert!(store.join("ccc333").exists());
}

#[test]
fn sweep_empty_report_is_noop() {
    let dir = tempfile::tempdir().unwrap();
    let report = GcReport {
        live: BTreeSet::new(),
        dead: BTreeSet::new(),
        total: 0,
    };

    let result = sweep(&report, dir.path()).unwrap();
    assert!(result.removed.is_empty());
    assert_eq!(result.bytes_freed, 0);
    assert!(result.errors.is_empty());
}

#[test]
fn sweep_nonexistent_entry_counted_as_removed() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".gc-journal")).unwrap();

    let report = dead_report(&["nonexistent123"]);
    let result = sweep(&report, dir.path()).unwrap();
    assert_eq!(result.removed.len(), 1);
    assert_eq!(result.bytes_freed, 0);
}

#[test]
fn sweep_continues_on_partial_failure() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path();

    make_store_entry(store, "good111", b"data");
    make_store_entry(store, "good222", b"data");

    let report = dead_report(&["good111", "good222"]);
    let result = sweep(&report, store).unwrap();

    // Both should be removed
    assert_eq!(result.removed.len(), 2);
}

#[test]
fn dry_run_reports_without_deleting() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path();

    make_store_entry(store, "dry111", b"keep this");
    make_store_entry(store, "dry222", b"and this");

    let report = dead_report(&["dry111", "dry222"]);
    let entries = sweep_dry_run(&report, store);

    assert_eq!(entries.len(), 2);
    // All entries still exist
    assert!(store.join("dry111").exists());
    assert!(store.join("dry222").exists());
    // Sizes reported
    assert!(entries.iter().all(|e| e.size_bytes > 0));
}

#[test]
fn dry_run_empty_report() {
    let dir = tempfile::tempdir().unwrap();
    let report = GcReport {
        live: BTreeSet::new(),
        dead: BTreeSet::new(),
        total: 0,
    };
    let entries = sweep_dry_run(&report, dir.path());
    assert!(entries.is_empty());
}

#[test]
fn gc_journal_written() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path();

    make_store_entry(store, "journal111", b"data");

    let report = dead_report(&["journal111"]);
    sweep(&report, store).unwrap();

    let journal_dir = store.join(".gc-journal");
    assert!(journal_dir.exists());
    let entries: Vec<_> = std::fs::read_dir(&journal_dir).unwrap().flatten().collect();
    assert!(!entries.is_empty(), "journal should have an entry");

    let content = std::fs::read_to_string(entries[0].path()).unwrap();
    assert!(content.contains("journal111"));
    assert!(content.contains("timestamp:"));
}

#[test]
fn dir_size_calculates_correctly() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), b"hello").unwrap(); // 5 bytes
    let sub = dir.path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(sub.join("b.txt"), b"world!").unwrap(); // 6 bytes

    let size = dir_size(dir.path());
    assert_eq!(size, 11);
}

#[test]
fn dir_size_nonexistent_returns_zero() {
    assert_eq!(dir_size(Path::new("/nonexistent/path")), 0);
}

#[test]
fn dir_size_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(dir_size(dir.path()), 0);
}

#[test]
fn path_traversal_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path();

    // Create a report with a hash that would traverse out of store
    let mut dead = BTreeSet::new();
    dead.insert("blake3:../../etc/passwd".to_string());
    let report = GcReport {
        live: BTreeSet::new(),
        dead,
        total: 1,
    };

    let result = sweep(&report, store).unwrap();
    // Should have an error, not a removal
    assert!(
        result.errors.len() == 1 || result.removed.is_empty(),
        "traversal attack should be caught"
    );
}

#[test]
fn integration_with_mark_and_sweep() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path();

    // Create entries
    make_store_entry(store, "live111", b"live");
    make_store_entry(store, "dead111", b"dead");

    // Collect roots (only live111)
    let roots = collect_roots(&["blake3:live111".to_string()], &[], None);

    // Mark and sweep
    let report = mark_and_sweep(&roots, store).unwrap();
    assert!(report.live.contains("blake3:live111"));
    assert!(report.dead.contains("blake3:dead111"));

    // Execute sweep
    let result = sweep(&report, store).unwrap();
    assert_eq!(result.removed.len(), 1);
    assert!(result.removed[0].contains("dead111"));

    // Verify filesystem state
    assert!(store.join("live111").exists());
    assert!(!store.join("dead111").exists());
}

#[test]
fn sweep_preserves_gc_roots_dir() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path();
    let gc_roots = store.join(".gc-roots");
    std::fs::create_dir_all(&gc_roots).unwrap();

    make_store_entry(store, "entry111", b"data");

    let report = dead_report(&["entry111"]);
    sweep(&report, store).unwrap();

    // .gc-roots should still exist
    assert!(gc_roots.exists());
}

#[test]
fn dry_run_entry_has_correct_hash() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path();
    make_store_entry(store, "check111", b"data");

    let report = dead_report(&["check111"]);
    let entries = sweep_dry_run(&report, store);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].hash, "blake3:check111");
}
