//! Tests for FJ-1220 lifecycle enforcement + FJ-1210 moved blocks in planner.

use super::*;
use super::tests_helpers::make_base_resource;
use crate::core::types::{
    LifecycleRules, MovedEntry, ResourceLock, ResourceStatus, ResourceType, StateLock,
};
use std::collections::HashMap;

fn test_lock(machine: &str) -> StateLock {
    StateLock {
        schema: "1".to_string(),
        machine: machine.to_string(),
        hostname: machine.to_string(),
        generated_at: String::new(),
        generator: "test".to_string(),
        blake3_version: "1".to_string(),
        resources: indexmap::IndexMap::new(),
    }
}

fn test_rl(rt: ResourceType, hash: &str) -> ResourceLock {
    ResourceLock {
        resource_type: rt,
        hash: hash.to_string(),
        status: ResourceStatus::Converged,
        applied_at: None,
        duration_seconds: None,
        details: HashMap::new(),
    }
}

// ============================================================================
// prevent_destroy tests
// ============================================================================

#[test]
fn test_prevent_destroy_blocks_destroy_action() {
    let mut resource = make_base_resource(ResourceType::File);
    resource.state = Some("absent".to_string());
    resource.lifecycle = Some(LifecycleRules {
        prevent_destroy: true,
        create_before_destroy: false,
        ignore_drift: vec![],
    });

    let mut lock = test_lock("m1");
    lock.resources.insert("protected-file".to_string(), test_rl(ResourceType::File, "abc123"));

    let mut locks = HashMap::new();
    locks.insert("m1".to_string(), lock);

    let action = determine_action("protected-file", &resource, "m1", &locks);
    assert_eq!(action, PlanAction::NoOp, "prevent_destroy should block Destroy → NoOp");
}

#[test]
fn test_destroy_allowed_without_prevent_destroy() {
    let mut resource = make_base_resource(ResourceType::File);
    resource.state = Some("absent".to_string());

    let mut lock = test_lock("m1");
    lock.resources.insert("normal-file".to_string(), test_rl(ResourceType::File, "abc123"));

    let mut locks = HashMap::new();
    locks.insert("m1".to_string(), lock);

    let action = determine_action("normal-file", &resource, "m1", &locks);
    assert_eq!(action, PlanAction::Destroy, "without prevent_destroy, Destroy should proceed");
}

#[test]
fn test_prevent_destroy_false_allows_destroy() {
    let mut resource = make_base_resource(ResourceType::File);
    resource.state = Some("absent".to_string());
    resource.lifecycle = Some(LifecycleRules {
        prevent_destroy: false,
        create_before_destroy: false,
        ignore_drift: vec![],
    });

    let mut lock = test_lock("m1");
    lock.resources.insert("removable".to_string(), test_rl(ResourceType::File, "abc123"));

    let mut locks = HashMap::new();
    locks.insert("m1".to_string(), lock);

    let action = determine_action("removable", &resource, "m1", &locks);
    assert_eq!(action, PlanAction::Destroy, "prevent_destroy=false should allow Destroy");
}

// ============================================================================
// moved blocks tests
// ============================================================================

#[test]
fn test_apply_moved_blocks_renames_resource_in_lock() {
    let mut lock = test_lock("m1");
    lock.resources.insert("old-config".to_string(), test_rl(ResourceType::File, "hash123"));

    let mut locks = HashMap::new();
    locks.insert("m1".to_string(), lock);

    let moved = vec![MovedEntry {
        from: "old-config".to_string(),
        to: "new-config".to_string(),
    }];

    let result = apply_moved_blocks(&moved, &locks);
    let m1_lock = result.get("m1").unwrap();

    assert!(!m1_lock.resources.contains_key("old-config"), "old key should be removed");
    assert!(m1_lock.resources.contains_key("new-config"), "new key should exist");

    let rl = m1_lock.resources.get("new-config").unwrap();
    assert_eq!(rl.hash, "hash123", "hash should be preserved across rename");
    assert_eq!(rl.status, ResourceStatus::Converged, "status should be preserved");
}

#[test]
fn test_apply_moved_blocks_no_op_when_empty() {
    let lock = test_lock("m1");
    let mut locks = HashMap::new();
    locks.insert("m1".to_string(), lock);

    let moved = vec![];
    let result = apply_moved_blocks(&moved, &locks);

    assert_eq!(result.len(), 1, "should return same number of machines");
}

#[test]
fn test_apply_moved_blocks_no_op_when_source_missing() {
    let mut lock = test_lock("m1");
    lock.resources.insert("existing-config".to_string(), test_rl(ResourceType::File, "hash456"));

    let mut locks = HashMap::new();
    locks.insert("m1".to_string(), lock);

    let moved = vec![MovedEntry {
        from: "nonexistent".to_string(),
        to: "renamed".to_string(),
    }];

    let result = apply_moved_blocks(&moved, &locks);
    let m1_lock = result.get("m1").unwrap();

    assert!(m1_lock.resources.contains_key("existing-config"), "existing should be untouched");
    assert!(!m1_lock.resources.contains_key("renamed"), "rename target shouldn't appear");
}

#[test]
fn test_apply_moved_blocks_multiple_renames() {
    let mut lock = test_lock("m1");
    lock.resources.insert("alpha".to_string(), test_rl(ResourceType::File, "h1"));
    lock.resources.insert("beta".to_string(), test_rl(ResourceType::Package, "h2"));

    let mut locks = HashMap::new();
    locks.insert("m1".to_string(), lock);

    let moved = vec![
        MovedEntry { from: "alpha".to_string(), to: "alpha-v2".to_string() },
        MovedEntry { from: "beta".to_string(), to: "beta-renamed".to_string() },
    ];

    let result = apply_moved_blocks(&moved, &locks);
    let m1 = result.get("m1").unwrap();

    assert!(m1.resources.contains_key("alpha-v2"));
    assert!(m1.resources.contains_key("beta-renamed"));
    assert!(!m1.resources.contains_key("alpha"));
    assert!(!m1.resources.contains_key("beta"));
}
