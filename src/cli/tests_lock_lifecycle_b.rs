//! Coverage tests for lock_lifecycle.rs — compress, archive, snapshot, defrag.

use super::lock_lifecycle::*;
use crate::core::state;

// ── cmd_lock_compress ───────────────────────────────────────────────

#[test]
fn compress_no_machines() {
    let dir = tempfile::tempdir().unwrap();
    let result = cmd_lock_compress(dir.path(), false);
    assert!(result.is_ok());
}

#[test]
fn compress_with_lock() {
    let dir = tempfile::tempdir().unwrap();
    let lock = state::new_lock("web", "web-host");
    state::save_lock(dir.path(), &lock).unwrap();

    let result = cmd_lock_compress(dir.path(), false);
    assert!(result.is_ok());
}

#[test]
fn compress_json_output() {
    let dir = tempfile::tempdir().unwrap();
    let lock = state::new_lock("web", "web-host");
    state::save_lock(dir.path(), &lock).unwrap();

    let result = cmd_lock_compress(dir.path(), true);
    assert!(result.is_ok());
}

#[test]
fn compress_with_comments_saves_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let m_dir = dir.path().join("web");
    std::fs::create_dir_all(&m_dir).unwrap();
    let lock_path = m_dir.join("state.lock.yaml");
    std::fs::write(
        &lock_path,
        "# This is a comment\nschema: '1.0'\nmachine: web\nhostname: web\n\n# Another comment\ngenerated_at: '2026-01-01'\ngenerator: forjar\nblake3_version: '1.8'\nresources: {}\n",
    )
    .unwrap();

    let result = cmd_lock_compress(dir.path(), false);
    assert!(result.is_ok());
    // Minified file should be created
    assert!(dir.path().join("web.lock.yaml.min").exists());
}

// ── cmd_lock_archive ────────────────────────────────────────────────

#[test]
fn archive_no_machines() {
    let dir = tempfile::tempdir().unwrap();
    let result = cmd_lock_archive(dir.path(), false);
    assert!(result.is_ok());
}

#[test]
fn archive_no_events() {
    let dir = tempfile::tempdir().unwrap();
    let lock = state::new_lock("web", "host");
    state::save_lock(dir.path(), &lock).unwrap();

    let result = cmd_lock_archive(dir.path(), false);
    assert!(result.is_ok());
}

#[test]
fn archive_with_events() {
    let dir = tempfile::tempdir().unwrap();
    let lock = state::new_lock("web", "host");
    state::save_lock(dir.path(), &lock).unwrap();

    // Create event log
    std::fs::write(
        dir.path().join("web.events.jsonl"),
        "{\"ts\":\"2026-03-08\",\"event\":{\"type\":\"test\"}}\n",
    )
    .unwrap();

    let result = cmd_lock_archive(dir.path(), false);
    assert!(result.is_ok());
    // Archive directory should exist
    assert!(dir.path().join("archive").exists());
}

#[test]
fn archive_json_output() {
    let dir = tempfile::tempdir().unwrap();
    let lock = state::new_lock("web", "host");
    state::save_lock(dir.path(), &lock).unwrap();

    let result = cmd_lock_archive(dir.path(), true);
    assert!(result.is_ok());
}

// ── cmd_lock_snapshot ───────────────────────────────────────────────

#[test]
fn snapshot_no_machines() {
    let dir = tempfile::tempdir().unwrap();
    let result = cmd_lock_snapshot(dir.path(), false);
    assert!(result.is_ok());
}

#[test]
fn snapshot_with_locks() {
    let dir = tempfile::tempdir().unwrap();
    let lock = state::new_lock("web", "host");
    state::save_lock(dir.path(), &lock).unwrap();

    let result = cmd_lock_snapshot(dir.path(), false);
    assert!(result.is_ok());
    // Snapshot dir should exist
    assert!(dir.path().join("snapshots").exists());
}

#[test]
fn snapshot_json_output() {
    let dir = tempfile::tempdir().unwrap();
    let lock = state::new_lock("web", "host");
    state::save_lock(dir.path(), &lock).unwrap();

    let result = cmd_lock_snapshot(dir.path(), true);
    assert!(result.is_ok());
}

#[test]
fn snapshot_multiple_machines() {
    let dir = tempfile::tempdir().unwrap();
    for name in &["web", "db", "cache"] {
        let lock = state::new_lock(name, &format!("{name}-host"));
        state::save_lock(dir.path(), &lock).unwrap();
    }

    let result = cmd_lock_snapshot(dir.path(), false);
    assert!(result.is_ok());
}

// ── cmd_lock_defrag ─────────────────────────────────────────────────

#[test]
fn defrag_no_machines() {
    let dir = tempfile::tempdir().unwrap();
    let result = cmd_lock_defrag(dir.path(), false);
    assert!(result.is_ok());
}

#[test]
fn defrag_with_lock() {
    let dir = tempfile::tempdir().unwrap();
    let lock = state::new_lock("web", "host");
    state::save_lock(dir.path(), &lock).unwrap();

    let result = cmd_lock_defrag(dir.path(), false);
    assert!(result.is_ok());
}

#[test]
fn defrag_json_output() {
    let dir = tempfile::tempdir().unwrap();
    let lock = state::new_lock("web", "host");
    state::save_lock(dir.path(), &lock).unwrap();

    let result = cmd_lock_defrag(dir.path(), true);
    assert!(result.is_ok());
}

#[test]
fn defrag_sorts_resources_alphabetically() {
    let dir = tempfile::tempdir().unwrap();
    let m_dir = dir.path().join("web");
    std::fs::create_dir_all(&m_dir).unwrap();

    // Create lock with resources in reverse order
    let mut lock = state::new_lock("web", "host");
    lock.resources.insert(
        "z-resource".to_string(),
        crate::core::types::ResourceLock {
            resource_type: crate::core::types::ResourceType::File,
            status: crate::core::types::ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:z".to_string(),
            details: std::collections::HashMap::new(),
        },
    );
    lock.resources.insert(
        "a-resource".to_string(),
        crate::core::types::ResourceLock {
            resource_type: crate::core::types::ResourceType::Package,
            status: crate::core::types::ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:a".to_string(),
            details: std::collections::HashMap::new(),
        },
    );
    state::save_lock(dir.path(), &lock).unwrap();

    cmd_lock_defrag(dir.path(), false).unwrap();

    // Load back and verify sorted
    let reloaded = state::load_lock(dir.path(), "web").unwrap().unwrap();
    let keys: Vec<&String> = reloaded.resources.keys().collect();
    assert!(keys.windows(2).all(|w| w[0] <= w[1]), "should be sorted");
}
