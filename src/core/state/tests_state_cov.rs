//! Coverage tests for state/mod.rs — parse_lock_pid, walk_yaml_files,
//! walk_age_files, save/load_apply_report, new_lock, new_global_lock,
//! force_unlock.

use super::*;

// ── parse_lock_pid ──────────────────────────────────────────────

#[test]
fn parse_lock_pid_valid() {
    let content = "pid: 12345\nstarted_at: 2026-03-08T12:00:00Z\n";
    assert_eq!(parse_lock_pid(content), Some(12345));
}

#[test]
fn parse_lock_pid_multiline_first() {
    let content = "started_at: 2026-03-08T12:00:00Z\npid: 99999\n";
    assert_eq!(parse_lock_pid(content), Some(99999));
}

#[test]
fn parse_lock_pid_no_pid_line() {
    let content = "started_at: 2026-03-08T12:00:00Z\n";
    assert_eq!(parse_lock_pid(content), None);
}

#[test]
fn parse_lock_pid_malformed() {
    let content = "pid: not_a_number\n";
    assert_eq!(parse_lock_pid(content), None);
}

#[test]
fn parse_lock_pid_empty() {
    assert_eq!(parse_lock_pid(""), None);
}

#[test]
fn parse_lock_pid_with_spaces() {
    let content = "pid:   42  \n";
    assert_eq!(parse_lock_pid(content), Some(42));
}

// ── walk_yaml_files ─────────────────────────────────────────────

#[test]
fn walk_yaml_files_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let files = walk_yaml_files(dir.path());
    assert!(files.is_empty());
}

#[test]
fn walk_yaml_files_finds_yaml() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("state.lock.yaml"), "data").unwrap();
    std::fs::write(dir.path().join("other.txt"), "data").unwrap();
    let files = walk_yaml_files(dir.path());
    assert_eq!(files.len(), 1);
    assert!(files[0].to_string_lossy().contains("state.lock.yaml"));
}

#[test]
fn walk_yaml_files_recursive() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("web");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("state.lock.yaml"), "data").unwrap();
    std::fs::write(dir.path().join("global.yaml"), "data").unwrap();
    let files = walk_yaml_files(dir.path());
    assert_eq!(files.len(), 2);
}

#[test]
fn walk_yaml_files_nonexistent() {
    let files = walk_yaml_files(std::path::Path::new("/tmp/forjar-nonexistent-xyz"));
    assert!(files.is_empty());
}

// ── walk_age_files ──────────────────────────────────────────────

#[test]
fn walk_age_files_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let files = walk_age_files(dir.path());
    assert!(files.is_empty());
}

#[test]
fn walk_age_files_finds_age() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("state.lock.yaml.age"), "encrypted").unwrap();
    std::fs::write(dir.path().join("state.lock.yaml"), "plain").unwrap();
    let files = walk_age_files(dir.path());
    assert_eq!(files.len(), 1);
    assert!(files[0].to_string_lossy().ends_with(".yaml.age"));
}

#[test]
fn walk_age_files_recursive() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("web");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("state.lock.yaml.age"), "enc").unwrap();
    let files = walk_age_files(dir.path());
    assert_eq!(files.len(), 1);
}

// ── save_apply_report / load_apply_report ───────────────────────

#[test]
fn save_load_apply_report_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let result = crate::core::types::ApplyResult {
        machine: "web".to_string(),
        resources_converged: 2,
        resources_unchanged: 0,
        resources_failed: 1,
        total_duration: std::time::Duration::from_millis(1500),
        resource_reports: vec![],
    };
    save_apply_report(dir.path(), &result).unwrap();
    let loaded = load_apply_report(dir.path(), "web").unwrap();
    assert!(loaded.is_some());
    let content = loaded.unwrap();
    assert!(content.contains("web"));
    assert!(content.contains("resources_converged: 2"));
}

#[test]
fn load_apply_report_nonexistent() {
    let dir = tempfile::tempdir().unwrap();
    let result = load_apply_report(dir.path(), "ghost").unwrap();
    assert!(result.is_none());
}

// ── new_lock ────────────────────────────────────────────────────

#[test]
fn new_lock_fields() {
    let lock = new_lock("web", "web.example.com");
    assert_eq!(lock.schema, "1.0");
    assert_eq!(lock.machine, "web");
    assert_eq!(lock.hostname, "web.example.com");
    assert!(!lock.generated_at.is_empty());
    assert!(lock.generator.contains("forjar"));
    assert_eq!(lock.blake3_version, "1.8");
    assert!(lock.resources.is_empty());
}

// ── new_global_lock ─────────────────────────────────────────────

#[test]
fn new_global_lock_fields() {
    let lock = new_global_lock("my-stack");
    assert_eq!(lock.schema, "1.0");
    assert_eq!(lock.name, "my-stack");
    assert!(!lock.last_apply.is_empty());
    assert!(lock.generator.contains("forjar"));
    assert!(lock.machines.is_empty());
    assert!(lock.outputs.is_empty());
}

// ── force_unlock ────────────────────────────────────────────────

#[test]
fn force_unlock_no_lock_file() {
    let dir = tempfile::tempdir().unwrap();
    assert!(force_unlock(dir.path()).is_ok());
}

#[test]
fn force_unlock_removes_lock() {
    let dir = tempfile::tempdir().unwrap();
    let lock_path = process_lock_path(dir.path());
    std::fs::write(&lock_path, "pid: 1\n").unwrap();
    assert!(lock_path.exists());
    force_unlock(dir.path()).unwrap();
    assert!(!lock_path.exists());
}

// ── lock_file_path / global_lock_path ───────────────────────────

#[test]
fn lock_file_path_format() {
    let p = lock_file_path(std::path::Path::new("/state"), "web");
    assert_eq!(p.to_string_lossy(), "/state/web/state.lock.yaml");
}

#[test]
fn global_lock_path_format() {
    let p = global_lock_path(std::path::Path::new("/state"));
    assert_eq!(p.to_string_lossy(), "/state/forjar.lock.yaml");
}

// ── process_lock_path ───────────────────────────────────────────

#[test]
fn process_lock_path_format() {
    let p = process_lock_path(std::path::Path::new("/state"));
    assert_eq!(p.to_string_lossy(), "/state/.forjar.lock");
}

// ── acquire_process_lock / release_process_lock ─────────────────

#[test]
fn acquire_release_process_lock() {
    let dir = tempfile::tempdir().unwrap();
    acquire_process_lock(dir.path()).unwrap();
    let lock_path = process_lock_path(dir.path());
    assert!(lock_path.exists());
    let content = std::fs::read_to_string(&lock_path).unwrap();
    assert!(content.contains("pid:"));
    release_process_lock(dir.path());
    assert!(!lock_path.exists());
}

// ── update_global_lock ───────────────────────────────────────────

#[test]
fn update_global_lock_creates_new() {
    let dir = tempfile::tempdir().unwrap();
    let results = vec![
        ("web".to_string(), 3usize, 2usize, 1usize),
        ("db".to_string(), 2, 2, 0),
    ];
    update_global_lock(dir.path(), "my-stack", &results).unwrap();
    let lock = load_global_lock(dir.path()).unwrap().unwrap();
    assert_eq!(lock.name, "my-stack");
    assert_eq!(lock.machines.len(), 2);
    assert_eq!(lock.machines["web"].resources, 3);
    assert_eq!(lock.machines["web"].converged, 2);
    assert_eq!(lock.machines["web"].failed, 1);
}

#[test]
fn update_global_lock_updates_existing() {
    let dir = tempfile::tempdir().unwrap();
    let results1 = vec![("web".to_string(), 3usize, 3usize, 0usize)];
    update_global_lock(dir.path(), "my-stack", &results1).unwrap();
    let results2 = vec![("web".to_string(), 4, 3, 1)];
    update_global_lock(dir.path(), "my-stack", &results2).unwrap();
    let lock = load_global_lock(dir.path()).unwrap().unwrap();
    assert_eq!(lock.machines["web"].resources, 4);
    assert_eq!(lock.machines["web"].failed, 1);
}

// ── save_lock / load_lock ───────────────────────────────────────

#[test]
fn save_load_lock_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let lock = new_lock("web", "web.example.com");
    save_lock(dir.path(), &lock).unwrap();
    let loaded = load_lock(dir.path(), "web").unwrap().unwrap();
    assert_eq!(loaded.machine, "web");
    assert_eq!(loaded.hostname, "web.example.com");
}

#[test]
fn load_lock_nonexistent() {
    let dir = tempfile::tempdir().unwrap();
    let result = load_lock(dir.path(), "ghost").unwrap();
    assert!(result.is_none());
}

// ── save_global_lock / load_global_lock ─────────────────────────

#[test]
fn save_load_global_lock_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let lock = new_global_lock("test-stack");
    save_global_lock(dir.path(), &lock).unwrap();
    let loaded = load_global_lock(dir.path()).unwrap().unwrap();
    assert_eq!(loaded.name, "test-stack");
}

#[test]
fn load_global_lock_nonexistent() {
    let dir = tempfile::tempdir().unwrap();
    let result = load_global_lock(dir.path()).unwrap();
    assert!(result.is_none());
}

// ── persist_outputs ─────────────────────────────────────────────

#[test]
fn persist_outputs_creates_lock() {
    let dir = tempfile::tempdir().unwrap();
    let mut outputs = indexmap::IndexMap::new();
    outputs.insert("db_url".to_string(), "postgres://localhost/db".to_string());
    persist_outputs(dir.path(), "my-stack", &outputs, false).unwrap();
    let lock = load_global_lock(dir.path()).unwrap().unwrap();
    assert_eq!(lock.outputs["db_url"], "postgres://localhost/db");
}

// ── resolve_outputs ─────────────────────────────────────────────

#[test]
fn resolve_outputs_empty() {
    let yaml = "version: '1.0'\nname: test\nmachines: {}\nresources: {}\n";
    let config: crate::core::types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let resolved = resolve_outputs(&config);
    assert!(resolved.is_empty());
}

#[test]
fn acquire_stale_lock_removed() {
    let dir = tempfile::tempdir().unwrap();
    // Write a lock with PID 1 (init, always running) — but we use 999999999 which doesn't exist
    let lock_path = process_lock_path(dir.path());
    std::fs::write(&lock_path, "pid: 999999999\n").unwrap();
    // Should succeed because PID 999999999 is not running
    acquire_process_lock(dir.path()).unwrap();
    release_process_lock(dir.path());
}
