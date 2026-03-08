use super::*;
use crate::core::types::{Machine, MachineTarget, Resource};
use crate::tripwire::hasher;

#[test]
fn test_fj016_full_drift_skips_non_file_without_live_hash() {
    // Non-file resource without live_hash should be skipped by detect_drift_full
    let mut resources = indexmap::IndexMap::new();
    resources.insert(
        "my-pkg".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::Package,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:abc".to_string(),
            details: std::collections::HashMap::new(), // no live_hash
        },
    );
    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "test".to_string(),
        hostname: "test-box".to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar 0.1.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    };

    // Local machine (127.0.0.1) — no real transport needed since there's no live_hash
    let machine = Machine {
        hostname: "test-box".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    };

    let config_resources = indexmap::IndexMap::new();
    let findings = detect_drift_full(&lock, &machine, &config_resources);
    assert!(
        findings.is_empty(),
        "non-file resources without live_hash should be skipped"
    );
}

#[test]
fn test_fj016_full_drift_skips_non_converged() {
    // Non-converged resources should be skipped regardless of type
    let mut resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "live_hash".to_string(),
        serde_yaml_ng::Value::String("blake3:xxx".to_string()),
    );
    resources.insert(
        "failed-pkg".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::Package,
            status: ResourceStatus::Failed,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:abc".to_string(),
            details,
        },
    );
    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "test".to_string(),
        hostname: "test-box".to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar 0.1.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    };
    let machine = Machine {
        hostname: "test-box".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    };
    let config_resources = indexmap::IndexMap::new();
    let findings = detect_drift_full(&lock, &machine, &config_resources);
    assert!(findings.is_empty(), "failed resources should be skipped");
}

#[test]
fn test_fj016_full_drift_skips_missing_resource_config() {
    // Resource with live_hash but not in config should be skipped
    let mut resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "live_hash".to_string(),
        serde_yaml_ng::Value::String("blake3:xxx".to_string()),
    );
    resources.insert(
        "orphan-pkg".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::Package,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:abc".to_string(),
            details,
        },
    );
    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "test".to_string(),
        hostname: "test-box".to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar 0.1.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    };
    let machine = Machine {
        hostname: "test-box".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    };
    // Empty config — resource not found
    let config_resources = indexmap::IndexMap::new();
    let findings = detect_drift_full(&lock, &machine, &config_resources);
    assert!(
        findings.is_empty(),
        "resources not in config should be skipped"
    );
}

// ── FJ-128: Drift detection edge case tests ──────────────────

fn make_test_machine() -> Machine {
    Machine {
        hostname: "test".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    }
}

fn make_service_resource(name: Option<&str>) -> Resource {
    Resource {
        resource_type: ResourceType::Service,
        machine: MachineTarget::Single("m".to_string()),
        state: Some("present".to_string()),
        depends_on: vec![],
        provider: None,
        packages: vec![],
        version: None,
        path: None,
        content: None,
        source: None,
        target: None,
        owner: None,
        group: None,
        mode: None,
        name: name.map(|s| s.to_string()),
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
        inputs: std::collections::HashMap::new(),
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
        quality_gate: None,
        health_check: None,
        restart_policy: None,
        pre_apply: None,
        post_apply: None,
        lifecycle: None,
        store: false,
        sudo: false,
        script: None,
        gather: vec![],
        scatter: vec![],
        build_machine: None,
    }
}

#[test]
fn test_fj016_detect_drift_full_matching_live_hash() {
    // Non-file resource where live state matches stored live_hash -> no drift
    let mut config_resources = indexmap::IndexMap::new();
    config_resources.insert("test-svc".to_string(), make_service_resource(Some("nginx")));

    // Compute what the state_query_script for this service would produce
    let query = crate::core::codegen::state_query_script(config_resources.get("test-svc").unwrap())
        .unwrap();
    let machine = make_test_machine();
    let output = crate::transport::exec_script(&machine, &query).unwrap();
    let live_hash = hasher::hash_string(&output.stdout);

    let mut lock_resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "live_hash".to_string(),
        serde_yaml_ng::Value::String(live_hash),
    );
    lock_resources.insert(
        "test-svc".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::Service,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:desired".to_string(),
            details,
        },
    );

    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "test".to_string(),
        hostname: "test".to_string(),
        generated_at: "now".to_string(),
        generator: "test".to_string(),
        blake3_version: "1.8".to_string(),
        resources: lock_resources,
    };

    let findings = detect_drift_full(&lock, &machine, &config_resources);
    assert!(
        findings.is_empty(),
        "matching live_hash should show no drift"
    );
}

#[test]
fn test_fj016_detect_drift_full_mismatched_live_hash() {
    // Non-file resource where live state differs -> drift detected
    let mut config_resources = indexmap::IndexMap::new();
    config_resources.insert("test-svc".to_string(), make_service_resource(Some("nginx")));

    let machine = make_test_machine();

    // Use a stale live_hash that won't match current systemctl output
    let mut lock_resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "live_hash".to_string(),
        serde_yaml_ng::Value::String("blake3:stale-from-yesterday".to_string()),
    );
    lock_resources.insert(
        "test-svc".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::Service,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:desired".to_string(),
            details,
        },
    );

    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "test".to_string(),
        hostname: "test".to_string(),
        generated_at: "now".to_string(),
        generator: "test".to_string(),
        blake3_version: "1.8".to_string(),
        resources: lock_resources,
    };

    let findings = detect_drift_full(&lock, &machine, &config_resources);
    assert_eq!(findings.len(), 1, "stale live_hash should detect drift");
    assert_eq!(findings[0].resource_id, "test-svc");
    assert!(findings[0].detail.contains("state changed"));
}

#[test]
fn test_fj016_detect_drift_full_codegen_error_skips() {
    // Resource present in lock + config but codegen fails -> should be skipped
    let mut config_resources = indexmap::IndexMap::new();
    config_resources.insert("broken-res".to_string(), make_service_resource(None));

    let machine = make_test_machine();

    let mut lock_resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "live_hash".to_string(),
        serde_yaml_ng::Value::String("blake3:old".to_string()),
    );
    lock_resources.insert(
        "broken-res".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::Service,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:abc".to_string(),
            details,
        },
    );

    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "test".to_string(),
        hostname: "test".to_string(),
        generated_at: "now".to_string(),
        generator: "test".to_string(),
        blake3_version: "1.8".to_string(),
        resources: lock_resources,
    };

    // This should not panic — codegen may succeed or fail, but drift detection should handle it
    let _findings = detect_drift_full(&lock, &machine, &config_resources);
    // The test verifies no panic occurs, and drift detection gracefully handles the case
}

#[test]
fn test_fj016_detect_drift_full_file_plus_service() {
    // Mixed: file resource (local) + service resource (live_hash) in same lock
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("mixed.txt");
    std::fs::write(&file, "stable").unwrap();
    let file_hash = hasher::hash_file(&file).unwrap();

    let mut config_resources = indexmap::IndexMap::new();
    config_resources.insert("my-svc".to_string(), make_service_resource(Some("nginx")));

    let machine = make_test_machine();

    // Run the real state query to get current live_hash
    let query =
        crate::core::codegen::state_query_script(config_resources.get("my-svc").unwrap()).unwrap();
    let output = crate::transport::exec_script(&machine, &query).unwrap();
    let svc_live_hash = hasher::hash_string(&output.stdout);

    let mut lock_resources = indexmap::IndexMap::new();

    // File resource — no drift
    let mut file_details = std::collections::HashMap::new();
    file_details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::String(file.to_str().unwrap().to_string()),
    );
    file_details.insert(
        "content_hash".to_string(),
        serde_yaml_ng::Value::String(file_hash),
    );
    lock_resources.insert(
        "my-file".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:desired".to_string(),
            details: file_details,
        },
    );

    // Service resource — no drift (live_hash matches)
    let mut svc_details = std::collections::HashMap::new();
    svc_details.insert(
        "live_hash".to_string(),
        serde_yaml_ng::Value::String(svc_live_hash),
    );
    lock_resources.insert(
        "my-svc".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::Service,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:desired".to_string(),
            details: svc_details,
        },
    );

    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "test".to_string(),
        hostname: "test".to_string(),
        generated_at: "now".to_string(),
        generator: "test".to_string(),
        blake3_version: "1.8".to_string(),
        resources: lock_resources,
    };

    let findings = detect_drift_full(&lock, &machine, &config_resources);
    assert!(
        findings.is_empty(),
        "no drift expected when both file and service hashes match"
    );
}
