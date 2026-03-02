#![allow(unused_imports)]
use super::tests_helpers::make_base_resource;
use super::*;
use std::collections::HashMap;

#[test]
fn test_fj132_determine_action_no_lock_creates() {
    let resource = make_base_resource(ResourceType::File);
    let locks = std::collections::HashMap::new();
    let action = determine_action("my-file", &resource, "web", &locks);
    assert_eq!(action, PlanAction::Create);
}

#[test]
fn test_fj132_determine_action_converged_same_hash_noop() {
    let mut resource = make_base_resource(ResourceType::File);
    resource.path = Some("/etc/test.conf".to_string());
    resource.content = Some("hello".to_string());
    let desired = hash_desired_state(&resource);
    let mut locks = std::collections::HashMap::new();
    let mut lock_resources = indexmap::IndexMap::new();
    lock_resources.insert(
        "my-file".to_string(),
        ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: desired,
            details: std::collections::HashMap::new(),
        },
    );
    locks.insert(
        "web".to_string(),
        StateLock {
            schema: "1.0".to_string(),
            machine: "web".to_string(),
            hostname: "web".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.8".to_string(),
            resources: lock_resources,
        },
    );
    let action = determine_action("my-file", &resource, "web", &locks);
    assert_eq!(action, PlanAction::NoOp);
}

#[test]
fn test_fj132_determine_action_hash_changed_updates() {
    let mut resource = make_base_resource(ResourceType::File);
    resource.path = Some("/etc/test.conf".to_string());
    resource.content = Some("new content".to_string());
    let mut locks = std::collections::HashMap::new();
    let mut lock_resources = indexmap::IndexMap::new();
    lock_resources.insert(
        "my-file".to_string(),
        ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:old_hash_value".to_string(),
            details: std::collections::HashMap::new(),
        },
    );
    locks.insert(
        "web".to_string(),
        StateLock {
            schema: "1.0".to_string(),
            machine: "web".to_string(),
            hostname: "web".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.8".to_string(),
            resources: lock_resources,
        },
    );
    let action = determine_action("my-file", &resource, "web", &locks);
    assert_eq!(action, PlanAction::Update);
}

#[test]
fn test_fj132_determine_action_absent_with_lock_destroys() {
    let mut resource = make_base_resource(ResourceType::File);
    resource.state = Some("absent".to_string());
    let mut locks = std::collections::HashMap::new();
    let mut lock_resources = indexmap::IndexMap::new();
    lock_resources.insert(
        "old-file".to_string(),
        ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:abc".to_string(),
            details: std::collections::HashMap::new(),
        },
    );
    locks.insert(
        "web".to_string(),
        StateLock {
            schema: "1.0".to_string(),
            machine: "web".to_string(),
            hostname: "web".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.8".to_string(),
            resources: lock_resources,
        },
    );
    let action = determine_action("old-file", &resource, "web", &locks);
    assert_eq!(action, PlanAction::Destroy);
}

#[test]
fn test_fj132_determine_action_absent_no_lock_noop() {
    let mut resource = make_base_resource(ResourceType::File);
    resource.state = Some("absent".to_string());
    let locks = std::collections::HashMap::new();
    let action = determine_action("old-file", &resource, "web", &locks);
    assert_eq!(action, PlanAction::NoOp);
}

#[test]
fn test_fj132_determine_action_failed_retries() {
    let resource = make_base_resource(ResourceType::Package);
    let mut locks = std::collections::HashMap::new();
    let mut lock_resources = indexmap::IndexMap::new();
    lock_resources.insert(
        "my-pkg".to_string(),
        ResourceLock {
            resource_type: ResourceType::Package,
            status: ResourceStatus::Failed,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:abc".to_string(),
            details: std::collections::HashMap::new(),
        },
    );
    locks.insert(
        "web".to_string(),
        StateLock {
            schema: "1.0".to_string(),
            machine: "web".to_string(),
            hostname: "web".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.8".to_string(),
            resources: lock_resources,
        },
    );
    let action = determine_action("my-pkg", &resource, "web", &locks);
    assert_eq!(
        action,
        PlanAction::Update,
        "failed resources should be retried"
    );
}
