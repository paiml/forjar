//! Additional coverage tests for FJ-1386 generation helpers.

use super::generation::{
    cmd_generation_diff, collect_generation_numbers, count_lock_resources, create_generation,
    current_generation, gc_generations, generations_dir, list_generations, load_gen_locks,
    lock_to_tuples, read_created_at, read_gen_info,
};
use crate::core::types::{GenerationMeta, ResourceLock, ResourceStatus, ResourceType, StateLock};
use std::collections::HashMap;

// ── Pure function tests ────────────────────────────────────────────

#[test]
fn lock_to_tuples_some_lock() {
    let mut resources = indexmap::IndexMap::new();
    resources.insert(
        "pkg-a".to_string(),
        ResourceLock {
            resource_type: ResourceType::Package,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:aaa".to_string(),
            details: HashMap::new(),
        },
    );
    resources.insert(
        "file-b".to_string(),
        ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:bbb".to_string(),
            details: HashMap::new(),
        },
    );
    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "m1".to_string(),
        hostname: "host".to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar 1.0.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    };
    let tuples = lock_to_tuples(Some(&lock));
    assert_eq!(tuples.len(), 2);
    assert!(tuples.iter().any(|(id, _, _)| id == "pkg-a"));
    assert!(tuples.iter().any(|(id, _, hash)| id == "file-b" && hash == "blake3:bbb"));
}

#[test]
fn lock_to_tuples_none() {
    let tuples = lock_to_tuples(None);
    assert!(tuples.is_empty());
}

#[test]
fn lock_to_tuples_empty_resources() {
    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "m1".to_string(),
        hostname: "host".to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar 1.0.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources: indexmap::IndexMap::new(),
    };
    let tuples = lock_to_tuples(Some(&lock));
    assert!(tuples.is_empty());
}

// ── read_created_at tests ──────────────────────────────────────────

#[test]
fn read_created_at_valid() {
    let dir = tempfile::tempdir().unwrap();
    let meta_path = dir.path().join(".generation.yaml");
    std::fs::write(&meta_path, "created_at: 2026-03-01T10:00:00Z\naction: apply\n").unwrap();
    let created = read_created_at(&meta_path);
    assert_eq!(created, "2026-03-01T10:00:00Z");
}

#[test]
fn read_created_at_quoted() {
    let dir = tempfile::tempdir().unwrap();
    let meta_path = dir.path().join(".generation.yaml");
    std::fs::write(&meta_path, "created_at: \"2026-03-01T10:00:00Z\"\n").unwrap();
    let created = read_created_at(&meta_path);
    assert_eq!(created, "2026-03-01T10:00:00Z");
}

#[test]
fn read_created_at_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let meta_path = dir.path().join("nonexistent.yaml");
    let created = read_created_at(&meta_path);
    assert_eq!(created, "unknown");
}

#[test]
fn read_created_at_no_field() {
    let dir = tempfile::tempdir().unwrap();
    let meta_path = dir.path().join(".generation.yaml");
    std::fs::write(&meta_path, "action: apply\n").unwrap();
    let created = read_created_at(&meta_path);
    assert_eq!(created, "unknown");
}

// ── count_lock_resources tests ─────────────────────────────────────

#[test]
fn count_lock_resources_with_locks() {
    let dir = tempfile::tempdir().unwrap();
    let gen_path = dir.path();

    // Create a machine dir with a valid lock
    let m1 = gen_path.join("m1");
    std::fs::create_dir_all(&m1).unwrap();
    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "m1".to_string(),
        hostname: "host".to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar 1.0.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources: {
            let mut r = indexmap::IndexMap::new();
            r.insert(
                "pkg".to_string(),
                ResourceLock {
                    resource_type: ResourceType::Package,
                    status: ResourceStatus::Converged,
                    applied_at: None,
                    duration_seconds: None,
                    hash: "blake3:x".to_string(),
                    details: HashMap::new(),
                },
            );
            r.insert(
                "file".to_string(),
                ResourceLock {
                    resource_type: ResourceType::File,
                    status: ResourceStatus::Converged,
                    applied_at: None,
                    duration_seconds: None,
                    hash: "blake3:y".to_string(),
                    details: HashMap::new(),
                },
            );
            r
        },
    };
    let yaml = serde_yaml_ng::to_string(&lock).unwrap();
    std::fs::write(m1.join("state.lock.yaml"), &yaml).unwrap();

    let count = count_lock_resources(gen_path);
    assert_eq!(count, 2);
}

#[test]
fn count_lock_resources_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let count = count_lock_resources(dir.path());
    assert_eq!(count, 0);
}

#[test]
fn count_lock_resources_no_lock_files() {
    let dir = tempfile::tempdir().unwrap();
    let m1 = dir.path().join("m1");
    std::fs::create_dir_all(&m1).unwrap();
    std::fs::write(m1.join("other.txt"), "data").unwrap();
    let count = count_lock_resources(dir.path());
    assert_eq!(count, 0);
}

// ── read_gen_info tests ────────────────────────────────────────────

#[test]
fn read_gen_info_valid_meta() {
    let dir = tempfile::tempdir().unwrap();
    let gen_dir = dir.path();
    let gen0 = gen_dir.join("0");
    std::fs::create_dir_all(&gen0).unwrap();

    let meta = GenerationMeta::new(0, "2026-03-01T10:00:00Z".to_string());
    let yaml = meta.to_yaml().unwrap();
    std::fs::write(gen0.join(".generation.yaml"), &yaml).unwrap();

    let info = read_gen_info(gen_dir, 0);
    assert_eq!(info.num, 0);
    assert_eq!(info.created_at, "2026-03-01T10:00:00Z");
    assert_eq!(info.action, "apply");
}

#[test]
fn read_gen_info_missing_meta() {
    let dir = tempfile::tempdir().unwrap();
    let gen_dir = dir.path();
    let gen0 = gen_dir.join("0");
    std::fs::create_dir_all(&gen0).unwrap();

    let info = read_gen_info(gen_dir, 0);
    assert_eq!(info.num, 0);
    assert_eq!(info.action, "apply");
    assert_eq!(info.changes, 0);
    assert_eq!(info.resource_count, 0);
}

#[test]
fn read_gen_info_corrupted_meta() {
    let dir = tempfile::tempdir().unwrap();
    let gen_dir = dir.path();
    let gen0 = gen_dir.join("0");
    std::fs::create_dir_all(&gen0).unwrap();
    std::fs::write(gen0.join(".generation.yaml"), "{{broken yaml").unwrap();

    let info = read_gen_info(gen_dir, 0);
    assert_eq!(info.num, 0);
    assert_eq!(info.changes, 0);
}

// ── load_gen_locks tests ───────────────────────────────────────────

#[test]
fn load_gen_locks_with_machines() {
    let dir = tempfile::tempdir().unwrap();
    let gen_dir = dir.path();

    let m1 = gen_dir.join("m1");
    std::fs::create_dir_all(&m1).unwrap();
    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "m1".to_string(),
        hostname: "host".to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar 1.0.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources: indexmap::IndexMap::new(),
    };
    let yaml = serde_yaml_ng::to_string(&lock).unwrap();
    std::fs::write(m1.join("state.lock.yaml"), &yaml).unwrap();

    let locks = load_gen_locks(gen_dir);
    assert_eq!(locks.len(), 1);
    assert!(locks.contains_key("m1"));
}

#[test]
fn load_gen_locks_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let locks = load_gen_locks(dir.path());
    assert!(locks.is_empty());
}

#[test]
fn load_gen_locks_skips_dotfiles() {
    let dir = tempfile::tempdir().unwrap();
    let hidden = dir.path().join(".generation.yaml");
    std::fs::write(&hidden, "metadata").unwrap();
    let locks = load_gen_locks(dir.path());
    assert!(locks.is_empty());
}

#[test]
fn load_gen_locks_skips_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("some-file.txt"), "data").unwrap();
    let locks = load_gen_locks(dir.path());
    assert!(locks.is_empty());
}

// ── collect_generation_numbers tests ───────────────────────────────

#[test]
fn collect_generation_numbers_basic() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("0")).unwrap();
    std::fs::create_dir(dir.path().join("1")).unwrap();
    std::fs::create_dir(dir.path().join("2")).unwrap();
    std::fs::write(dir.path().join("current"), "link").unwrap(); // non-numeric

    let nums = collect_generation_numbers(dir.path()).unwrap();
    assert_eq!(nums.len(), 3);
    assert!(nums.contains(&0));
    assert!(nums.contains(&1));
    assert!(nums.contains(&2));
}

// ── gc_generations verbose test ────────────────────────────────────

#[test]
fn gc_generations_verbose() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    let machine_dir = state_dir.join("m1");
    std::fs::create_dir_all(&machine_dir).unwrap();
    std::fs::write(machine_dir.join("state.lock.yaml"), "schema: '1.0'\nresources: {}").unwrap();

    for _ in 0..4 {
        create_generation(&state_dir, None).unwrap();
    }

    // Verbose gc
    gc_generations(&state_dir, 2, true);

    let gen_dir = state_dir.join("generations");
    assert!(!gen_dir.join("0").exists());
    assert!(!gen_dir.join("1").exists());
    assert!(gen_dir.join("2").exists());
    assert!(gen_dir.join("3").exists());
}

#[test]
fn gc_generations_no_dir() {
    let dir = tempfile::tempdir().unwrap();
    // Should not panic on nonexistent generations dir
    gc_generations(dir.path(), 2, false);
}

// ── cmd_generation_diff tests ──────────────────────────────────────

fn setup_with_generations(dir: &std::path::Path) -> std::path::PathBuf {
    let state_dir = dir.join("state");
    let machine_dir = state_dir.join("m1");
    std::fs::create_dir_all(&machine_dir).unwrap();

    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "m1".to_string(),
        hostname: "host".to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar 1.0.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources: {
            let mut r = indexmap::IndexMap::new();
            r.insert(
                "pkg-a".to_string(),
                ResourceLock {
                    resource_type: ResourceType::Package,
                    status: ResourceStatus::Converged,
                    applied_at: None,
                    duration_seconds: None,
                    hash: "blake3:v1".to_string(),
                    details: HashMap::new(),
                },
            );
            r
        },
    };
    let yaml = serde_yaml_ng::to_string(&lock).unwrap();
    std::fs::write(machine_dir.join("state.lock.yaml"), &yaml).unwrap();
    create_generation(&state_dir, None).unwrap();

    // Modify lock for gen 1
    let mut lock2 = lock;
    lock2
        .resources
        .get_mut("pkg-a")
        .unwrap()
        .hash = "blake3:v2".to_string();
    lock2.resources.insert(
        "file-b".to_string(),
        ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:new".to_string(),
            details: HashMap::new(),
        },
    );
    let yaml2 = serde_yaml_ng::to_string(&lock2).unwrap();
    std::fs::write(state_dir.join("m1").join("state.lock.yaml"), &yaml2).unwrap();
    create_generation(&state_dir, None).unwrap();

    state_dir
}

#[test]
fn cmd_generation_diff_text() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = setup_with_generations(dir.path());
    let result = cmd_generation_diff(&state_dir, 0, 1, false);
    assert!(result.is_ok());
}

#[test]
fn cmd_generation_diff_json() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = setup_with_generations(dir.path());
    let result = cmd_generation_diff(&state_dir, 0, 1, true);
    assert!(result.is_ok());
}

#[test]
fn cmd_generation_diff_same_gen() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = setup_with_generations(dir.path());
    let result = cmd_generation_diff(&state_dir, 0, 0, false);
    assert!(result.is_ok());
}

#[test]
fn cmd_generation_diff_from_missing() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = setup_with_generations(dir.path());
    let result = cmd_generation_diff(&state_dir, 99, 0, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn cmd_generation_diff_to_missing() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = setup_with_generations(dir.path());
    let result = cmd_generation_diff(&state_dir, 0, 99, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

// ── current_generation edge cases ──────────────────────────────────

#[test]
fn current_generation_no_symlink() {
    let dir = tempfile::tempdir().unwrap();
    let result = current_generation(dir.path());
    assert!(result.is_none());
}

// ── generations_dir ────────────────────────────────────────────────

#[test]
fn generations_dir_path() {
    let path = generations_dir(std::path::Path::new("/var/state"));
    assert_eq!(path, std::path::PathBuf::from("/var/state/generations"));
}

// ── print_generations_text/json via list_generations ────────────────

#[test]
fn list_generations_json_with_data() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = setup_with_generations(dir.path());
    let result = list_generations(&state_dir, true);
    assert!(result.is_ok());
}

#[test]
fn list_generations_text_with_data() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = setup_with_generations(dir.path());
    let result = list_generations(&state_dir, false);
    assert!(result.is_ok());
}
