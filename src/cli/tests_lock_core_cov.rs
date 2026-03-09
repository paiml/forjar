//! Coverage tests for lock_core.rs — collect_verify_mismatches, validate_single_lock.

use crate::core::types::*;
use indexmap::IndexMap;
use std::collections::HashMap;

fn make_lock(machine: &str, resources: IndexMap<String, ResourceLock>) -> StateLock {
    StateLock {
        schema: "1".to_string(),
        machine: machine.to_string(),
        hostname: "localhost".to_string(),
        generated_at: "2026-03-08T12:00:00Z".to_string(),
        generator: "forjar 1.0.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    }
}

fn make_rl(hash: &str) -> ResourceLock {
    ResourceLock {
        resource_type: ResourceType::File,
        status: ResourceStatus::Converged,
        applied_at: Some("2026-03-08T12:00:00Z".to_string()),
        duration_seconds: Some(0.01),
        hash: hash.to_string(),
        details: HashMap::new(),
    }
}

// ── collect_verify_mismatches ───────────────────────────────────

#[test]
fn verify_matching_hashes() {
    let mut res = IndexMap::new();
    res.insert("nginx".to_string(), make_rl("blake3:abc123def456"));
    let new_lock = make_lock("web", res.clone());
    let existing = make_lock("web", res);
    let mut mismatches = Vec::new();
    super::lock_core::collect_verify_mismatches("web", &new_lock, &existing, &mut mismatches);
    assert!(mismatches.is_empty());
}

#[test]
fn verify_hash_mismatch() {
    let mut new_res = IndexMap::new();
    new_res.insert("nginx".to_string(), make_rl("blake3:new_hash_value"));
    let mut old_res = IndexMap::new();
    old_res.insert("nginx".to_string(), make_rl("blake3:old_hash_value"));
    let new_lock = make_lock("web", new_res);
    let existing = make_lock("web", old_res);
    let mut mismatches = Vec::new();
    super::lock_core::collect_verify_mismatches("web", &new_lock, &existing, &mut mismatches);
    assert_eq!(mismatches.len(), 1);
    assert!(mismatches[0].contains("hash mismatch"));
}

#[test]
fn verify_new_resource_not_in_lock() {
    let mut new_res = IndexMap::new();
    new_res.insert("nginx".to_string(), make_rl("blake3:abc"));
    new_res.insert("redis".to_string(), make_rl("blake3:def"));
    let mut old_res = IndexMap::new();
    old_res.insert("nginx".to_string(), make_rl("blake3:abc"));
    let new_lock = make_lock("web", new_res);
    let existing = make_lock("web", old_res);
    let mut mismatches = Vec::new();
    super::lock_core::collect_verify_mismatches("web", &new_lock, &existing, &mut mismatches);
    assert!(mismatches.iter().any(|m| m.contains("not in lock")));
}

#[test]
fn verify_resource_removed_from_config() {
    let mut new_res = IndexMap::new();
    new_res.insert("nginx".to_string(), make_rl("blake3:abc"));
    let mut old_res = IndexMap::new();
    old_res.insert("nginx".to_string(), make_rl("blake3:abc"));
    old_res.insert("redis".to_string(), make_rl("blake3:def"));
    let new_lock = make_lock("web", new_res);
    let existing = make_lock("web", old_res);
    let mut mismatches = Vec::new();
    super::lock_core::collect_verify_mismatches("web", &new_lock, &existing, &mut mismatches);
    assert!(mismatches
        .iter()
        .any(|m| m.contains("in lock but not in config")));
}

// ── validate_single_lock ────────────────────────────────────────

#[test]
fn validate_lock_valid_schema_1() {
    let lock = make_lock("web", IndexMap::new());
    let issues = super::lock_core::validate_single_lock("web", &lock);
    assert!(issues.is_empty());
}

#[test]
fn validate_lock_valid_schema_1_0() {
    let mut lock = make_lock("web", IndexMap::new());
    lock.schema = "1.0".to_string();
    let issues = super::lock_core::validate_single_lock("web", &lock);
    assert!(issues.is_empty());
}

#[test]
fn validate_lock_invalid_schema() {
    let mut lock = make_lock("web", IndexMap::new());
    lock.schema = "2.0".to_string();
    let issues = super::lock_core::validate_single_lock("web", &lock);
    assert!(issues.iter().any(|(_, msg)| msg.contains("schema")));
}

#[test]
fn validate_lock_empty_hash() {
    let mut res = IndexMap::new();
    res.insert("broken".to_string(), make_rl(""));
    let lock = make_lock("web", res);
    let issues = super::lock_core::validate_single_lock("web", &lock);
    assert!(issues.iter().any(|(_, msg)| msg.contains("empty hash")));
}

#[test]
fn validate_lock_multiple_issues() {
    let mut res = IndexMap::new();
    res.insert("r1".to_string(), make_rl(""));
    res.insert("r2".to_string(), make_rl(""));
    let mut lock = make_lock("web", res);
    lock.schema = "99".to_string();
    let issues = super::lock_core::validate_single_lock("web", &lock);
    assert!(issues.len() >= 3); // 1 schema + 2 empty hash
}
