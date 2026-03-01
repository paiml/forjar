//! FJ-131: Executor + state edge case tests (part 1).

use super::*;
use super::test_fixtures::*;

// ── FJ-131: Executor + state edge case tests ──────────────────

#[test]
fn test_fj131_collect_machines_with_localhost() {
    // Resources targeting "localhost" (implicit machine) appear in collect output
    let yaml = r#"
version: "1.0"
name: localhost-test
machines:
  web:
    hostname: web
    addr: 1.1.1.1
resources:
  local-file:
    type: file
    machine: localhost
    path: /tmp/test
    content: "x"
  web-file:
    type: file
    machine: web
    path: /tmp/test2
    content: "y"
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let machines = collect_machines(&config);
    assert!(machines.contains(&"localhost".to_string()));
    assert!(machines.contains(&"web".to_string()));
    assert_eq!(machines.len(), 2);
}

#[test]
fn test_fj131_apply_empty_resources() {
    // Config with machines but no resources → empty results
    let yaml = r#"
version: "1.0"
name: empty-resources
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources: {}
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
    };
    let results = apply(&cfg).unwrap();
    // No resources → no machines collected → empty results
    assert!(results.is_empty());
}

#[test]
fn test_fj131_apply_machine_filter_no_match() {
    // Machine filter doesn't match any collected machine → empty results
    let config = local_config();
    let dir = tempfile::tempdir().unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: dir.path(),
        force: false,
        dry_run: false,
        machine_filter: Some("nonexistent-machine"),
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
    };
    let results = apply(&cfg).unwrap();
    assert!(
        results.is_empty(),
        "machine filter that matches nothing should yield no results"
    );
}

#[test]
fn test_fj131_record_failure_docker_resource() {
    let dir = tempfile::tempdir().unwrap();
    let mut lock = state::new_lock("test", "test-box");
    let mut ctx = RecordCtx {
        lock: &mut lock,
        state_dir: dir.path(),
        machine_name: "test",
        tripwire: true,
        failure_policy: &FailurePolicy::StopOnFirst,
        timeout_secs: None,
    };

    let should_stop = record_failure(
        &mut ctx,
        "my-container",
        &ResourceType::Docker,
        3.0,
        "image pull failed",
    );

    assert!(should_stop);
    let rl = &ctx.lock.resources["my-container"];
    assert_eq!(rl.status, ResourceStatus::Failed);
    assert_eq!(rl.resource_type, ResourceType::Docker);
    assert_eq!(rl.hash, "");
    assert!(rl.duration_seconds.unwrap() > 2.0);
}

#[test]
fn test_fj131_record_failure_mount_continue() {
    let dir = tempfile::tempdir().unwrap();
    let mut lock = state::new_lock("test", "test-box");
    let mut ctx = RecordCtx {
        lock: &mut lock,
        state_dir: dir.path(),
        machine_name: "test",
        tripwire: false,
        failure_policy: &FailurePolicy::ContinueIndependent,
        timeout_secs: None,
    };

    let should_stop = record_failure(
        &mut ctx,
        "nfs-share",
        &ResourceType::Mount,
        0.8,
        "mount: permission denied",
    );

    assert!(!should_stop);
    assert_eq!(
        ctx.lock.resources["nfs-share"].resource_type,
        ResourceType::Mount
    );
}

#[test]
fn test_fj131_build_details_group_only() {
    // Resource with group but no owner/mode → only group in details
    let r = Resource {
        resource_type: ResourceType::File,
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
        group: Some("www-data".to_string()),
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
        pre_apply: None,
        post_apply: None,
        lifecycle: None,
    };
    let details = build_resource_details(&r, &local_machine());
    assert_eq!(
        details["group"],
        serde_yaml_ng::Value::String("www-data".to_string())
    );
    assert!(!details.contains_key("owner"), "owner not set");
    assert!(!details.contains_key("mode"), "mode not set");
    assert!(!details.contains_key("path"), "path not set");
}

#[test]
fn test_fj131_collect_machines_multiple_target() {
    // MachineTarget::Multiple collects all targets
    let yaml = r#"
version: "1.0"
name: multi-target
machines:
  a:
    hostname: a
    addr: 1.1.1.1
  b:
    hostname: b
    addr: 2.2.2.2
  c:
    hostname: c
    addr: 3.3.3.3
resources:
  r:
    type: file
    machine: [a, b, c]
    path: /tmp/test
    content: test
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let machines = collect_machines(&config);
    assert_eq!(machines.len(), 3);
    assert!(machines.contains(&"a".to_string()));
    assert!(machines.contains(&"b".to_string()));
    assert!(machines.contains(&"c".to_string()));
}

#[test]
fn test_fj131_apply_localhost_implicit_machine() {
    // Resources on "localhost" work without defining localhost in machines block
    let yaml = r#"
version: "1.0"
name: localhost-apply
machines: {}
resources:
  local-file:
    type: file
    machine: localhost
    path: /tmp/forjar-test-localhost-implicit.txt
    content: "localhost works"
policy:
  lock_file: true
  tripwire: false
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
    };
    let results = apply(&cfg).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].machine, "localhost");
    assert_eq!(results[0].resources_converged, 1);

    let content = std::fs::read_to_string("/tmp/forjar-test-localhost-implicit.txt").unwrap();
    assert_eq!(content.trim(), "localhost works");

    // Lock should reference localhost
    let lock = state::load_lock(dir.path(), "localhost").unwrap().unwrap();
    assert!(lock.resources.contains_key("local-file"));

    let _ = std::fs::remove_file("/tmp/forjar-test-localhost-implicit.txt");
}

