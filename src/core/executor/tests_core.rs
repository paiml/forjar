//! FJ-012: Core executor tests — collect_machines, build_resource_details, apply basics.

use super::test_fixtures::*;
use super::*;

#[test]
fn test_fj012_collect_machines() {
    let config = local_config();
    let machines = collect_machines(&config);
    assert_eq!(machines, vec!["local"]);
}

#[test]
fn test_fj012_collect_machines_multi() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  a:
    hostname: a
    addr: 1.1.1.1
  b:
    hostname: b
    addr: 2.2.2.2
resources:
  r1:
    type: package
    machine: a
    provider: apt
    packages: [x]
  r2:
    type: package
    machine: [a, b]
    provider: apt
    packages: [y]
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let machines = collect_machines(&config);
    assert_eq!(machines, vec!["a", "b"]);
}

#[test]
fn test_fj012_build_resource_details() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.txt");
    std::fs::write(&file_path, "hello").unwrap();
    let r = Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single("m".to_string()),
        state: None,
        depends_on: vec![],
        provider: None,
        packages: vec![],
        version: None,
        path: Some(file_path.to_str().unwrap().to_string()),
        content: Some("hello".to_string()),
        source: None,
        target: None,
        owner: Some("root".to_string()),
        group: Some("root".to_string()),
        mode: Some("0644".to_string()),
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
        build_machine: None,
        repo: None,
        tag: None,
        asset_pattern: None,
        binary: None,
        install_dir: None,
    };
    let details = build_resource_details(&r, &local_machine());
    assert!(details.contains_key("path"));
    assert!(details.contains_key("content_hash"));
    assert!(details.contains_key("owner"));
    assert!(details.contains_key("mode"));
    assert!(details.contains_key("group"));
    // content_hash should match hash_file (not hash_string)
    let expected = hasher::hash_file(&file_path).unwrap();
    let actual = details["content_hash"].as_str().unwrap();
    assert_eq!(
        actual, expected,
        "content_hash must use hash_file for drift consistency"
    );
}

#[test]
fn test_fj012_build_resource_details_service() {
    let r = Resource {
        resource_type: ResourceType::Service,
        machine: MachineTarget::Single("m".to_string()),
        state: None,
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
        name: Some("nginx".to_string()),
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
        build_machine: None,
        repo: None,
        tag: None,
        asset_pattern: None,
        binary: None,
        install_dir: None,
    };
    let details = build_resource_details(&r, &local_machine());
    assert!(details.contains_key("service_name"));
    assert_eq!(
        details["service_name"],
        serde_yaml_ng::Value::String("nginx".to_string())
    );
}

#[test]
fn test_fj012_dry_run() {
    let config = local_config();
    let dir = tempfile::tempdir().unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: dir.path(),
        force: false,
        dry_run: true,
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
        refresh: false,
        force_tag: None,
    };
    let results = apply(&cfg).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].machine, "dry-run");
}

#[test]
fn test_fj012_apply_local_file() {
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
        refresh: false,
        force_tag: None,
    };
    let results = apply(&cfg).unwrap();
    assert_eq!(results.len(), 1);

    // Verify lock was saved
    let lock = state::load_lock(dir.path(), "local").unwrap();
    assert!(lock.is_some());

    // Verify event log exists
    let events_path = dir.path().join("local").join("events.jsonl");
    assert!(events_path.exists());

    // Clean up
    let _ = std::fs::remove_file("/tmp/forjar-test-executor.txt");
}

#[test]
fn test_fj012_apply_idempotent() {
    let config = local_config();
    let dir = tempfile::tempdir().unwrap();

    // First apply
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
        refresh: false,
        force_tag: None,
    };
    let r1 = apply(&cfg).unwrap();
    assert_eq!(r1[0].resources_converged, 1);

    // Second apply — should be unchanged
    let cfg2 = ApplyConfig {
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
        refresh: false,
        force_tag: None,
    };
    let r2 = apply(&cfg2).unwrap();
    assert_eq!(r2[0].resources_unchanged, 1);
    assert_eq!(r2[0].resources_converged, 0);

    let _ = std::fs::remove_file("/tmp/forjar-test-executor.txt");
}

#[test]
fn test_fj012_force_reapply() {
    let config = local_config();
    let dir = tempfile::tempdir().unwrap();

    // First apply
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
        refresh: false,
        force_tag: None,
    };
    apply(&cfg).unwrap();

    // Force re-apply
    let cfg2 = ApplyConfig {
        config: &config,
        state_dir: dir.path(),
        force: true,
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
        refresh: false,
        force_tag: None,
    };
    let r2 = apply(&cfg2).unwrap();
    assert_eq!(r2[0].resources_converged, 1);

    let _ = std::fs::remove_file("/tmp/forjar-test-executor.txt");
}
