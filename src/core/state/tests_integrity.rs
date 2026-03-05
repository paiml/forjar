//! Tests for FJ-1270: State integrity verification.

use super::*;
use integrity::{has_errors, verify_state_integrity, write_b3_sidecar, IntegrityResult};
use tempfile::TempDir;

/// Valid state with matching sidecars passes verification.
#[test]
fn valid_state_passes() {
    let tmp = TempDir::new().expect("tempdir");
    let state_dir = tmp.path();

    // Create a global lock (which writes sidecar via save_global_lock)
    let lock = new_global_lock("test");
    save_global_lock(state_dir, &lock).expect("save global lock");

    let results = verify_state_integrity(state_dir);
    assert!(!has_errors(&results));
    assert!(results.iter().any(|r| matches!(r, IntegrityResult::Ok)));
}

/// Per-machine lock with sidecar also passes.
#[test]
fn machine_lock_passes() {
    let tmp = TempDir::new().expect("tempdir");
    let state_dir = tmp.path();

    let lock = new_lock("web", "web.example.com");
    save_lock(state_dir, &lock).expect("save lock");

    let results = verify_state_integrity(state_dir);
    assert!(!has_errors(&results));
}

/// Truncated/corrupted YAML is detected.
#[test]
fn truncated_yaml_detected() {
    let tmp = TempDir::new().expect("tempdir");
    let state_dir = tmp.path();
    let lock_path = state_dir.join("forjar.lock.yaml");

    std::fs::write(
        &lock_path,
        "schema: \"1.0\"\nname: \"bad\"\n: {\ninvalid yaml{{{{",
    )
    .expect("write");

    let results = verify_state_integrity(state_dir);
    assert!(has_errors(&results));
    assert!(results
        .iter()
        .any(|r| matches!(r, IntegrityResult::InvalidYaml(..))));
}

/// Tampered file detected via hash mismatch.
#[test]
fn tampered_hash_detected() {
    let tmp = TempDir::new().expect("tempdir");
    let state_dir = tmp.path();

    // Write valid lock with sidecar
    let lock = new_global_lock("test");
    save_global_lock(state_dir, &lock).expect("save");

    // Tamper with the lock file
    let lock_path = state_dir.join("forjar.lock.yaml");
    let mut content = std::fs::read_to_string(&lock_path).expect("read");
    content.push_str("\nhacked: true\n");
    std::fs::write(&lock_path, content).expect("overwrite");

    let results = verify_state_integrity(state_dir);
    assert!(has_errors(&results));
    assert!(results
        .iter()
        .any(|r| matches!(r, IntegrityResult::HashMismatch { .. })));
}

/// Missing .b3 sidecar is a warning, not an error.
#[test]
fn missing_b3_is_warning() {
    let tmp = TempDir::new().expect("tempdir");
    let state_dir = tmp.path();

    // Write lock file without sidecar
    let lock_path = state_dir.join("forjar.lock.yaml");
    std::fs::create_dir_all(state_dir).expect("mkdir");
    let lock = new_global_lock("test");
    let yaml = serde_yaml_ng::to_string(&lock).expect("serialize");
    std::fs::write(&lock_path, yaml).expect("write");
    // Explicitly remove sidecar if it exists
    let sidecar = lock_path.with_extension("lock.yaml.b3");
    let _ = std::fs::remove_file(&sidecar);

    let results = verify_state_integrity(state_dir);
    assert!(!has_errors(&results)); // warnings are not errors
    assert!(results
        .iter()
        .any(|r| matches!(r, IntegrityResult::MissingSidecar(..))));
}

/// write_b3_sidecar creates a file with correct hash.
#[test]
fn write_b3_sidecar_creates_correct_hash() {
    let tmp = TempDir::new().expect("tempdir");
    let file_path = tmp.path().join("test.yaml");
    let content = "hello: world\n";
    std::fs::write(&file_path, content).expect("write");

    write_b3_sidecar(&file_path).expect("write sidecar");

    let sidecar_path = tmp.path().join("test.yaml.b3");
    assert!(sidecar_path.exists());

    let stored_hash = std::fs::read_to_string(&sidecar_path).expect("read sidecar");
    let expected_hash = blake3::hash(content.as_bytes()).to_hex().to_string();
    assert_eq!(stored_hash, expected_hash);
}

/// Empty state directory passes with no results.
#[test]
fn empty_state_dir_passes() {
    let tmp = TempDir::new().expect("tempdir");
    let results = verify_state_integrity(tmp.path());
    assert!(results.is_empty());
    assert!(!has_errors(&results));
}
