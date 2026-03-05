//! Tests for FJ-1220: lifecycle.ignore_drift enforcement in drift detection.

use super::*;
use crate::core::types::{LifecycleRules, MachineTarget, Resource, ResourceType};
use std::collections::HashMap;

fn make_file_resource(lifecycle: Option<LifecycleRules>) -> Resource {
    Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single("m1".to_string()),
        state: None,
        depends_on: vec![],
        provider: None,
        packages: vec![],
        version: None,
        path: Some("/etc/test.conf".to_string()),
        content: Some("hello".to_string()),
        source: None,
        target: None,
        owner: None,
        group: None,
        mode: None,
        name: None,
        enabled: None,
        restart_on: vec![],
        triggers: vec![],
        fs_type: None,
        options: None,
        uid: None,
        shell: None,
        home: None,
        groups: vec![],
        ssh_authorized_keys: vec![],
        system_user: false,
        schedule: None,
        command: None,
        image: None,
        ports: vec![],
        environment: vec![],
        volumes: vec![],
        restart: None,
        protocol: None,
        port: None,
        action: None,
        from_addr: None,
        recipe: None,
        inputs: HashMap::new(),
        arch: vec![],
        tags: vec![],
        resource_group: None,
        when: None,
        count: None,
        for_each: None,
        chroot_dir: None,
        namespace_uid: None,
        namespace_gid: None,
        seccomp: false,
        netns: false,
        cpuset: None,
        memory_limit: None,
        overlay_lower: None,
        overlay_upper: None,
        overlay_work: None,
        overlay_merged: None,
        format: None,
        quantization: None,
        checksum: None,
        cache_dir: None,
        gpu_backend: None,
        driver_version: None,
        cuda_version: None,
        rocm_version: None,
        devices: vec![],
        persistence_mode: None,
        compute_mode: None,
        gpu_memory_limit_mb: None,
        output_artifacts: vec![],
        completion_check: None,
        timeout: None,
        working_dir: None,
        task_mode: None,
        task_inputs: vec![],
        stages: vec![],
        cache: false,
        gpu_device: None,
        restart_delay: None,
        pre_apply: None,
        post_apply: None,
        lifecycle,
        store: false,
        sudo: false,
        script: None,
        gather: vec![],
        scatter: vec![],
    }
}

#[test]
fn test_should_ignore_drift_returns_true_with_wildcard() {
    let mut resources = indexmap::IndexMap::new();
    resources.insert(
        "cfg".to_string(),
        make_file_resource(Some(LifecycleRules {
            prevent_destroy: false,
            create_before_destroy: false,
            ignore_drift: vec!["*".to_string()],
        })),
    );

    assert!(should_ignore_drift("cfg", &resources));
}

#[test]
fn test_should_ignore_drift_returns_true_with_specific_fields() {
    let mut resources = indexmap::IndexMap::new();
    resources.insert(
        "cfg".to_string(),
        make_file_resource(Some(LifecycleRules {
            prevent_destroy: false,
            create_before_destroy: false,
            ignore_drift: vec!["content".to_string(), "mode".to_string()],
        })),
    );

    assert!(should_ignore_drift("cfg", &resources));
}

#[test]
fn test_should_ignore_drift_returns_false_without_lifecycle() {
    let mut resources = indexmap::IndexMap::new();
    resources.insert("cfg".to_string(), make_file_resource(None));

    assert!(!should_ignore_drift("cfg", &resources));
}

#[test]
fn test_should_ignore_drift_returns_false_with_empty_ignore_list() {
    let mut resources = indexmap::IndexMap::new();
    resources.insert(
        "cfg".to_string(),
        make_file_resource(Some(LifecycleRules {
            prevent_destroy: false,
            create_before_destroy: false,
            ignore_drift: vec![],
        })),
    );

    assert!(!should_ignore_drift("cfg", &resources));
}

#[test]
fn test_should_ignore_drift_returns_false_for_unknown_resource() {
    let resources = indexmap::IndexMap::new();
    assert!(!should_ignore_drift("nonexistent", &resources));
}
