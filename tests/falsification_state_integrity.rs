//! FJ-013/1270/266: State management, integrity, and process locking falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-013: Lock file load/save/atomic write, global lock, new_lock
//!   - lock_file_path derivation
//!   - save_lock / load_lock roundtrip
//!   - save_global_lock / load_global_lock roundtrip
//!   - new_lock fields
//!   - new_global_lock fields
//!   - load_lock returns None for missing file
//!   - save_apply_report / load_apply_report roundtrip
//! - FJ-1270: State integrity via BLAKE3 sidecar
//!   - write_b3_sidecar creates sidecar file
//!   - verify_state_integrity: all pass, missing sidecar, hash mismatch
//!   - has_errors predicate
//! - FJ-266: Process locking
//!   - acquire/release process lock roundtrip
//!   - force_unlock
//!
//! Usage: cargo test --test falsification_state_integrity

use forjar::core::state;
use forjar::core::state::integrity;
use forjar::core::types::{ApplyResult, ResourceReport, StateLock};

// ============================================================================
// Helpers
// ============================================================================

fn make_state_dir() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    (dir, state_dir)
}

fn sample_lock(machine: &str) -> StateLock {
    state::new_lock(machine, &format!("{machine}-01"))
}

// ============================================================================
// FJ-013: lock_file_path
// ============================================================================

#[test]
fn lock_file_path_derivation() {
    let path = state::lock_file_path(std::path::Path::new("/state"), "web");
    assert_eq!(path, std::path::PathBuf::from("/state/web/state.lock.yaml"));
}

#[test]
fn lock_file_path_nested_machine() {
    let path = state::lock_file_path(std::path::Path::new("/data/state"), "db-primary");
    assert_eq!(
        path,
        std::path::PathBuf::from("/data/state/db-primary/state.lock.yaml")
    );
}

// ============================================================================
// FJ-013: new_lock
// ============================================================================

#[test]
fn new_lock_fields() {
    let lock = state::new_lock("web", "web-01");
    assert_eq!(lock.machine, "web");
    assert_eq!(lock.hostname, "web-01");
    assert_eq!(lock.schema, "1.0");
    assert!(!lock.generated_at.is_empty());
    assert!(lock.generator.starts_with("forjar"));
    assert!(lock.resources.is_empty());
}

// ============================================================================
// FJ-013: save_lock / load_lock roundtrip
// ============================================================================

#[test]
fn save_load_lock_roundtrip() {
    let (_dir, state_dir) = make_state_dir();
    let lock = sample_lock("web");

    state::save_lock(&state_dir, &lock).unwrap();
    let loaded = state::load_lock(&state_dir, "web").unwrap();

    assert!(loaded.is_some());
    let loaded = loaded.unwrap();
    assert_eq!(loaded.machine, "web");
    assert_eq!(loaded.hostname, "web-01");
    assert_eq!(loaded.schema, "1.0");
}

#[test]
fn load_lock_missing_returns_none() {
    let (_dir, state_dir) = make_state_dir();
    let loaded = state::load_lock(&state_dir, "nonexistent").unwrap();
    assert!(loaded.is_none());
}

#[test]
fn save_lock_creates_machine_dir() {
    let (_dir, state_dir) = make_state_dir();
    let lock = sample_lock("new-machine");

    state::save_lock(&state_dir, &lock).unwrap();
    let machine_dir = state_dir.join("new-machine");
    assert!(machine_dir.exists());
    assert!(machine_dir.join("state.lock.yaml").exists());
}

// ============================================================================
// FJ-013: global lock
// ============================================================================

#[test]
fn new_global_lock_fields() {
    let lock = state::new_global_lock("my-infra");
    assert_eq!(lock.name, "my-infra");
    assert_eq!(lock.schema, "1.0");
    assert!(!lock.last_apply.is_empty());
    assert!(lock.generator.starts_with("forjar"));
    assert!(lock.machines.is_empty());
    assert!(lock.outputs.is_empty());
}

#[test]
fn global_lock_path_derivation() {
    let path = state::global_lock_path(std::path::Path::new("/state"));
    assert_eq!(path, std::path::PathBuf::from("/state/forjar.lock.yaml"));
}

#[test]
fn save_load_global_lock_roundtrip() {
    let (_dir, state_dir) = make_state_dir();
    let lock = state::new_global_lock("test-infra");

    state::save_global_lock(&state_dir, &lock).unwrap();
    let loaded = state::load_global_lock(&state_dir).unwrap();

    assert!(loaded.is_some());
    let loaded = loaded.unwrap();
    assert_eq!(loaded.name, "test-infra");
    assert_eq!(loaded.schema, "1.0");
}

#[test]
fn load_global_lock_missing_returns_none() {
    let (_dir, state_dir) = make_state_dir();
    let loaded = state::load_global_lock(&state_dir).unwrap();
    assert!(loaded.is_none());
}

// ============================================================================
// FJ-013: update_global_lock
// ============================================================================

#[test]
fn update_global_lock_creates_machine_entries() {
    let (_dir, state_dir) = make_state_dir();
    let results = vec![("web".to_string(), 5, 5, 0), ("db".to_string(), 3, 2, 1)];
    state::update_global_lock(&state_dir, "my-infra", &results).unwrap();

    let lock = state::load_global_lock(&state_dir).unwrap().unwrap();
    assert_eq!(lock.name, "my-infra");
    assert_eq!(lock.machines.len(), 2);
    assert_eq!(lock.machines["web"].resources, 5);
    assert_eq!(lock.machines["web"].converged, 5);
    assert_eq!(lock.machines["db"].failed, 1);
}

// ============================================================================
// FJ-013: save/load apply report
// ============================================================================

#[test]
fn save_load_apply_report() {
    let (_dir, state_dir) = make_state_dir();
    let report = ApplyResult {
        machine: "web".into(),
        resources_converged: 1,
        resources_unchanged: 0,
        resources_failed: 0,
        total_duration: std::time::Duration::from_millis(500),
        resource_reports: vec![ResourceReport {
            resource_id: "pkg-nginx".into(),
            resource_type: "package".into(),
            status: "converged".into(),
            duration_seconds: 0.5,
            exit_code: Some(0),
            hash: Some("abc123".into()),
            error: None,
        }],
    };
    state::save_apply_report(&state_dir, &report).unwrap();

    let loaded = state::load_apply_report(&state_dir, "web").unwrap();
    assert!(loaded.is_some());
    let content = loaded.unwrap();
    assert!(content.contains("pkg-nginx"));
    assert!(content.contains("converged"));
}

#[test]
fn load_apply_report_missing() {
    let (_dir, state_dir) = make_state_dir();
    let loaded = state::load_apply_report(&state_dir, "nonexistent").unwrap();
    assert!(loaded.is_none());
}

// ============================================================================
// FJ-1270: BLAKE3 sidecar — write and verify
// ============================================================================

#[test]
fn write_b3_sidecar_creates_file() {
    let (_dir, state_dir) = make_state_dir();
    let lock = sample_lock("web");
    state::save_lock(&state_dir, &lock).unwrap();

    let lock_path = state::lock_file_path(&state_dir, "web");
    // save_lock already writes sidecar, but verify it exists
    let sidecar = std::path::PathBuf::from(format!("{}.b3", lock_path.display()));
    assert!(sidecar.exists(), "BLAKE3 sidecar should exist");

    let hash_content = std::fs::read_to_string(&sidecar).unwrap();
    assert_eq!(hash_content.len(), 64, "BLAKE3 hash should be 64 hex chars");
}

#[test]
fn verify_integrity_all_pass() {
    let (_dir, state_dir) = make_state_dir();
    let lock = sample_lock("web");
    state::save_lock(&state_dir, &lock).unwrap();

    let results = integrity::verify_state_integrity(&state_dir);
    assert!(
        !integrity::has_errors(&results),
        "integrity should pass: {results:?}"
    );
}

#[test]
fn verify_integrity_hash_mismatch() {
    let (_dir, state_dir) = make_state_dir();
    let lock = sample_lock("tampered");
    state::save_lock(&state_dir, &lock).unwrap();

    // Tamper with the lock file
    let lock_path = state::lock_file_path(&state_dir, "tampered");
    let mut content = std::fs::read_to_string(&lock_path).unwrap();
    content.push_str("\n# tampered\n");
    std::fs::write(&lock_path, content).unwrap();

    let results = integrity::verify_state_integrity(&state_dir);
    assert!(
        integrity::has_errors(&results),
        "integrity should detect tampered file"
    );
}

#[test]
fn verify_integrity_missing_sidecar() {
    let (_dir, state_dir) = make_state_dir();

    // Manually create lock file without sidecar
    let machine_dir = state_dir.join("no-sidecar");
    std::fs::create_dir_all(&machine_dir).unwrap();
    std::fs::write(
        machine_dir.join("state.lock.yaml"),
        "schema: \"1.0\"\nmachine: no-sidecar\nhostname: h1\ngenerated_at: now\ngenerator: test\nblake3_version: \"1.8\"\nresources: {}\n",
    )
    .unwrap();

    let results = integrity::verify_state_integrity(&state_dir);
    let has_missing = results
        .iter()
        .any(|r| matches!(r, integrity::IntegrityResult::MissingSidecar(_)));
    assert!(has_missing, "should detect missing sidecar");
    // Missing sidecar is a warning, not an error
    assert!(
        !integrity::has_errors(&results),
        "missing sidecar should not be a hard error"
    );
}

#[test]
fn verify_integrity_empty_state_dir() {
    let (_dir, state_dir) = make_state_dir();
    let results = integrity::verify_state_integrity(&state_dir);
    assert!(results.is_empty());
    assert!(!integrity::has_errors(&results));
}

#[test]
fn has_errors_false_for_ok() {
    let results = vec![integrity::IntegrityResult::Ok];
    assert!(!integrity::has_errors(&results));
}

// ============================================================================
// FJ-266: Process locking
// ============================================================================

#[test]
fn acquire_release_process_lock() {
    let (_dir, state_dir) = make_state_dir();
    state::acquire_process_lock(&state_dir).unwrap();

    // Lock file should exist
    let lock_file = state_dir.join(".forjar.lock");
    assert!(lock_file.exists());
    let content = std::fs::read_to_string(&lock_file).unwrap();
    assert!(content.contains("pid:"));

    state::release_process_lock(&state_dir);
    assert!(!lock_file.exists());
}

#[test]
fn force_unlock_removes_lock() {
    let (_dir, state_dir) = make_state_dir();
    state::acquire_process_lock(&state_dir).unwrap();
    state::force_unlock(&state_dir).unwrap();

    let lock_file = state_dir.join(".forjar.lock");
    assert!(!lock_file.exists());
}

#[test]
fn force_unlock_no_lock_file() {
    let (_dir, state_dir) = make_state_dir();
    // Should succeed even if no lock file
    state::force_unlock(&state_dir).unwrap();
}

// ============================================================================
// FJ-013: resolve_outputs
// ============================================================================

#[test]
fn resolve_outputs_empty() {
    let config = forjar::core::types::ForjarConfig {
        version: "1.0".into(),
        name: "test".into(),
        ..Default::default()
    };
    let resolved = state::resolve_outputs(&config);
    assert!(resolved.is_empty());
}

#[test]
fn resolve_outputs_with_literal() {
    let mut config = forjar::core::types::ForjarConfig {
        version: "1.0".into(),
        name: "test".into(),
        ..Default::default()
    };
    config.outputs.insert(
        "url".into(),
        forjar::core::types::OutputValue {
            value: "https://example.com".into(),
            description: None,
        },
    );
    let resolved = state::resolve_outputs(&config);
    assert_eq!(resolved["url"], "https://example.com");
}

// ============================================================================
// FJ-013: persist_outputs
// ============================================================================

#[test]
fn persist_outputs_non_ephemeral() {
    let (_dir, state_dir) = make_state_dir();
    let mut outputs = indexmap::IndexMap::new();
    outputs.insert("url".into(), "https://example.com".into());

    state::persist_outputs(&state_dir, "test", &outputs, false).unwrap();

    let lock = state::load_global_lock(&state_dir).unwrap().unwrap();
    assert_eq!(lock.outputs["url"], "https://example.com");
}

#[test]
fn persist_outputs_ephemeral_redacts() {
    let (_dir, state_dir) = make_state_dir();
    let mut outputs = indexmap::IndexMap::new();
    outputs.insert("secret_key".into(), "super-secret-value".into());

    state::persist_outputs(&state_dir, "test", &outputs, true).unwrap();

    let lock = state::load_global_lock(&state_dir).unwrap().unwrap();
    // Ephemeral mode replaces values with BLAKE3 hashes
    assert_ne!(lock.outputs["secret_key"], "super-secret-value");
    // Should be a BLAKE3 hash (64 hex chars) or redacted format
    assert!(lock.outputs["secret_key"].len() >= 10);
}
