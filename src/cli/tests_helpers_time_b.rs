//! Additional coverage tests for time helpers and diff command.

use super::diff_cmd::*;
use super::helpers_time::*;
use crate::core::{state, types};
use std::collections::HashMap;

// ── estimate_hours_between ─────────────────────────────────────────

#[test]
fn estimate_hours_normal() {
    let h = estimate_hours_between("2026-03-01T10:00:00Z", "2026-03-01T12:30:00Z");
    // 2.5 hours
    assert!((h - 2.5).abs() < 0.01);
}

#[test]
fn estimate_hours_same_time() {
    let h = estimate_hours_between("2026-03-01T10:00:00Z", "2026-03-01T10:00:00Z");
    assert!((h - 0.0).abs() < 0.001);
}

#[test]
fn estimate_hours_short_string() {
    // Strings shorter than 19 chars → default 1.0
    let h = estimate_hours_between("short", "also-short");
    assert!((h - 1.0).abs() < 0.001);
}

#[test]
fn estimate_hours_unparseable() {
    let h = estimate_hours_between("xxxx-xx-xxTxx:xx:xxZ", "yyyy-yy-yyTyy:yy:yyZ");
    assert!((h - 1.0).abs() < 0.001);
}

#[test]
fn estimate_hours_different_days() {
    // Day 02 vs Day 01, same time → 24 hours from day difference
    let h = estimate_hours_between("2026-03-01T00:00:00Z", "2026-03-02T00:00:00Z");
    assert!((h - 24.0).abs() < 0.01);
}

// ── parse_duration_secs error paths ────────────────────────────────

#[test]
fn parse_duration_secs_too_short() {
    let result = parse_duration_secs("1");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("invalid duration"));
}

#[test]
fn parse_duration_secs_empty() {
    let result = parse_duration_secs("");
    assert!(result.is_err());
}

#[test]
fn parse_duration_secs_bad_number() {
    let result = parse_duration_secs("abch");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("invalid duration number"));
}

#[test]
fn parse_duration_secs_unknown_unit() {
    let result = parse_duration_secs("10x");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown duration unit"));
}

#[test]
fn parse_duration_secs_all_units() {
    assert_eq!(parse_duration_secs("10s").unwrap(), 10);
    assert_eq!(parse_duration_secs("10m").unwrap(), 600);
    assert_eq!(parse_duration_secs("10h").unwrap(), 36000);
    assert_eq!(parse_duration_secs("10d").unwrap(), 864000);
}

// ── parse_duration_string error paths ──────────────────────────────

#[test]
fn parse_duration_string_empty() {
    let result = parse_duration_string("");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("empty duration string"));
}

#[test]
fn parse_duration_string_unknown_unit() {
    let result = parse_duration_string("10y");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown duration unit"));
}

#[test]
fn parse_duration_string_bad_number() {
    let result = parse_duration_string("xxd");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("invalid duration"));
}

// ── chrono_now_compact ─────────────────────────────────────────────

#[test]
fn chrono_now_compact_returns_numeric() {
    let ts = chrono_now_compact();
    // Should be all digits (unix timestamp string)
    assert!(ts.chars().all(|c| c.is_ascii_digit()));
    assert!(!ts.is_empty());
}

// ── cmd_diff with JSON output ──────────────────────────────────────

#[test]
fn cmd_diff_json_with_changes() {
    let from = tempfile::tempdir().unwrap();
    let to = tempfile::tempdir().unwrap();

    // "from" has one resource
    let lock_from = types::StateLock {
        schema: "1.0".to_string(),
        machine: "web".to_string(),
        hostname: "web".to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar 1.0.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources: {
            let mut r = indexmap::IndexMap::new();
            r.insert(
                "pkg-a".to_string(),
                types::ResourceLock {
                    resource_type: types::ResourceType::Package,
                    status: types::ResourceStatus::Converged,
                    applied_at: None,
                    duration_seconds: None,
                    hash: "blake3:old".to_string(),
                    details: HashMap::new(),
                },
            );
            r
        },
    };
    state::save_lock(from.path(), &lock_from).unwrap();

    // "to" has changed hash and new resource
    let mut lock_to = lock_from.clone();
    lock_to
        .resources
        .get_mut("pkg-a")
        .unwrap()
        .hash = "blake3:new".to_string();
    lock_to.resources.insert(
        "file-b".to_string(),
        types::ResourceLock {
            resource_type: types::ResourceType::File,
            status: types::ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:added".to_string(),
            details: HashMap::new(),
        },
    );
    state::save_lock(to.path(), &lock_to).unwrap();

    // JSON output
    let result = cmd_diff(from.path(), to.path(), None, None, true);
    assert!(result.is_ok());
}

#[test]
fn cmd_diff_with_resource_filter() {
    let from = tempfile::tempdir().unwrap();
    let to = tempfile::tempdir().unwrap();

    let lock = types::StateLock {
        schema: "1.0".to_string(),
        machine: "web".to_string(),
        hostname: "web".to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar 1.0.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources: {
            let mut r = indexmap::IndexMap::new();
            r.insert(
                "pkg-a".to_string(),
                types::ResourceLock {
                    resource_type: types::ResourceType::Package,
                    status: types::ResourceStatus::Converged,
                    applied_at: None,
                    duration_seconds: None,
                    hash: "blake3:aaa".to_string(),
                    details: HashMap::new(),
                },
            );
            r
        },
    };
    state::save_lock(from.path(), &lock).unwrap();

    // Remove resource in "to"
    let mut lock_to = lock.clone();
    lock_to.resources.clear();
    state::save_lock(to.path(), &lock_to).unwrap();

    // Filter to specific resource
    let result = cmd_diff(from.path(), to.path(), None, Some("pkg-a"), false);
    assert!(result.is_ok());
}

#[test]
fn cmd_diff_removed_resource_text() {
    let from = tempfile::tempdir().unwrap();
    let to = tempfile::tempdir().unwrap();

    let lock = types::StateLock {
        schema: "1.0".to_string(),
        machine: "web".to_string(),
        hostname: "web".to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar 1.0.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources: {
            let mut r = indexmap::IndexMap::new();
            r.insert(
                "old-pkg".to_string(),
                types::ResourceLock {
                    resource_type: types::ResourceType::Package,
                    status: types::ResourceStatus::Converged,
                    applied_at: None,
                    duration_seconds: None,
                    hash: "blake3:x".to_string(),
                    details: HashMap::new(),
                },
            );
            r
        },
    };
    state::save_lock(from.path(), &lock).unwrap();

    // "to" has no resources — removal
    let mut lock_to = lock.clone();
    lock_to.resources.clear();
    state::save_lock(to.path(), &lock_to).unwrap();

    let result = cmd_diff(from.path(), to.path(), None, None, false);
    assert!(result.is_ok());
}

// ── cmd_env_diff ───────────────────────────────────────────────────

#[test]
fn cmd_env_diff_identical() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path();
    let env1 = state_dir.join("staging");
    let env2 = state_dir.join("production");
    std::fs::create_dir_all(&env1).unwrap();
    std::fs::create_dir_all(&env2).unwrap();

    // No machines/locks — environments are identical
    let result = cmd_env_diff("staging", "production", state_dir, false);
    assert!(result.is_ok());
}

#[test]
fn cmd_env_diff_json_identical() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path();
    std::fs::create_dir_all(state_dir.join("a")).unwrap();
    std::fs::create_dir_all(state_dir.join("b")).unwrap();

    let result = cmd_env_diff("a", "b", state_dir, true);
    assert!(result.is_ok());
}

#[test]
fn cmd_env_diff_missing_env1() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("prod")).unwrap();

    let result = cmd_env_diff("staging", "prod", dir.path(), false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn cmd_env_diff_missing_env2() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("staging")).unwrap();

    let result = cmd_env_diff("staging", "prod", dir.path(), false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn cmd_env_diff_with_drift() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path();

    // Create env1 with a machine lock
    let env1_dir = state_dir.join("staging");
    let env1_m1 = env1_dir.join("web");
    std::fs::create_dir_all(&env1_m1).unwrap();
    let lock1 = types::StateLock {
        schema: "1.0".to_string(),
        machine: "web".to_string(),
        hostname: "web".to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar 1.0.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources: {
            let mut r = indexmap::IndexMap::new();
            r.insert(
                "pkg".to_string(),
                types::ResourceLock {
                    resource_type: types::ResourceType::Package,
                    status: types::ResourceStatus::Converged,
                    applied_at: None,
                    duration_seconds: None,
                    hash: "blake3:staging-hash".to_string(),
                    details: HashMap::new(),
                },
            );
            r
        },
    };
    state::save_lock(&env1_dir, &lock1).unwrap();

    // Create env2 with different hash
    let env2_dir = state_dir.join("production");
    let env2_m1 = env2_dir.join("web");
    std::fs::create_dir_all(&env2_m1).unwrap();
    let mut lock2 = lock1.clone();
    lock2.resources.get_mut("pkg").unwrap().hash = "blake3:prod-hash".to_string();
    state::save_lock(&env2_dir, &lock2).unwrap();

    // Text output — should show drift
    let result = cmd_env_diff("staging", "production", state_dir, false);
    assert!(result.is_ok());

    // JSON output — should show drift
    let result = cmd_env_diff("staging", "production", state_dir, true);
    assert!(result.is_ok());
}
