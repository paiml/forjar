//! Edge-case tests: apply variants (duration, timeout, arch filter, force, lock, tripwire).

use super::test_fixtures::*;
use super::*;

#[test]
fn test_fj012_apply_result_duration_positive() {
    let config = local_config();
    let dir = tempfile::tempdir().unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: dir.path(),
        force: false,
        dry_run: false,
        machine_filter: None,
        resource_filter: None,
        tag_filter: None,
        group_filter: None,
        timeout_secs: None,
        force_unlock: false,
        progress: false,
        retry: 0,
        parallel: None,
        resource_timeout: None,
        rollback_on_failure: false,
        max_parallel: None,
        trace: false,
        run_id: None,
    };
    let results = apply(&cfg).unwrap();
    for r in &results {
        assert!(r.total_duration.as_secs_f64() >= 0.0);
    }
    // Clean up the test file
    let _ = std::fs::remove_file("/tmp/forjar-test-executor.txt");
}

#[test]
fn test_fj012_build_resource_details_empty() {
    // Resource with no path, no content, no name → empty details
    let r = Resource {
        resource_type: ResourceType::Package,
        machine: MachineTarget::Single("m".to_string()),
        state: None,
        depends_on: vec![],
        provider: Some("apt".to_string()),
        packages: vec!["curl".to_string()],
        version: None,
        path: None,
        content: None,
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
    };
    let details = build_resource_details(&r, &local_machine());
    assert!(
        details.is_empty(),
        "package resource with no path/content/name should have empty details"
    );
}

#[test]
fn test_fj012_build_resource_details_path_only() {
    // File resource with path but no content → path in details but no content_hash
    let r = Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single("m".to_string()),
        state: Some("present".to_string()),
        depends_on: vec![],
        provider: None,
        packages: vec![],
        version: None,
        path: Some("/tmp/forjar-test-path-only.txt".to_string()),
        content: None, // no content
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
    };
    let details = build_resource_details(&r, &local_machine());
    assert!(details.contains_key("path"));
    assert!(
        !details.contains_key("content_hash"),
        "no content means no content_hash"
    );
}

#[test]
fn test_fj012_apply_with_timeout() {
    // Apply with explicit timeout_secs — verifies the timeout parameter threads through
    let config = local_config();
    let dir = tempfile::tempdir().unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: dir.path(),
        force: false,
        dry_run: false,
        machine_filter: None,
        resource_filter: None,
        tag_filter: None,
        group_filter: None,
        timeout_secs: Some(30),
        force_unlock: false,
        progress: false,
        retry: 0,
        parallel: None,
        resource_timeout: None,
        rollback_on_failure: false,
        max_parallel: None,
        trace: false,
        run_id: None,
    };
    let results = apply(&cfg).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].resources_converged, 1);
    let _ = std::fs::remove_file("/tmp/forjar-test-executor.txt");
}

#[test]
fn test_fj012_apply_arch_filter_skip() {
    // Resource with arch=[aarch64] on x86_64 machine → should be skipped
    let yaml = r#"
version: "1.0"
name: arch-skip-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
    arch: x86_64
resources:
  arm-file:
    type: file
    machine: local
    path: /tmp/forjar-test-arch-skip.txt
    content: "arm only"
    arch: [aarch64]
policy:
  lock_file: true
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: dir.path(),
        force: false,
        dry_run: false,
        machine_filter: None,
        resource_filter: None,
        tag_filter: None,
        group_filter: None,
        timeout_secs: None,
        force_unlock: false,
        progress: false,
        retry: 0,
        parallel: None,
        resource_timeout: None,
        rollback_on_failure: false,
        max_parallel: None,
        trace: false,
        run_id: None,
    };
    let results = apply(&cfg).unwrap();
    // Resource should be skipped due to arch mismatch
    assert_eq!(results[0].resources_converged, 0);
    assert_eq!(results[0].resources_unchanged, 0);
    assert!(
        !std::path::Path::new("/tmp/forjar-test-arch-skip.txt").exists(),
        "arch-filtered resource should not create file"
    );
}
