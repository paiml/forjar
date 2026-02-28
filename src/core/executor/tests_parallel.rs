//! FJ-034: Parallel machine execution tests, record failure/success edge cases, proptest.

use super::*;
use super::test_fixtures::*;
use proptest::prelude::*;

#[test]
fn test_fj012_record_failure_stop_on_first() {
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
        "failing-pkg",
        &ResourceType::Package,
        0.5,
        "exit code 1: not found",
    );

    assert!(should_stop, "StopOnFirst should return true");
    let rl = &ctx.lock.resources["failing-pkg"];
    assert_eq!(rl.status, ResourceStatus::Failed);
    assert_eq!(rl.hash, "");
    assert!(rl.duration_seconds.unwrap() > 0.0);
}

#[test]
fn test_fj012_record_failure_continue() {
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
        "failing-pkg",
        &ResourceType::Package,
        1.0,
        "exit code 2: error",
    );

    assert!(!should_stop, "Continue policy should return false");
    assert_eq!(
        ctx.lock.resources["failing-pkg"].status,
        ResourceStatus::Failed
    );
}

#[test]
fn test_fj012_record_failure_with_tripwire_logging() {
    let dir = tempfile::tempdir().unwrap();
    let mut lock = state::new_lock("test", "test-box");
    let mut ctx = RecordCtx {
        lock: &mut lock,
        state_dir: dir.path(),
        machine_name: "test",
        tripwire: true,
        failure_policy: &FailurePolicy::ContinueIndependent,
        timeout_secs: None,
    };

    record_failure(
        &mut ctx,
        "broken-svc",
        &ResourceType::Service,
        2.0,
        "transport error: connection refused",
    );

    // Verify event log was written
    let events_path = dir.path().join("test").join("events.jsonl");
    assert!(events_path.exists(), "tripwire event log should be written");
    let content = std::fs::read_to_string(&events_path).unwrap();
    assert!(content.contains("broken-svc"));
    assert!(content.contains("resource_failed"));
}

#[test]
fn test_fj012_record_success_writes_lock_and_event() {
    let dir = tempfile::tempdir().unwrap();
    let managed_file = dir.path().join("managed.txt");
    std::fs::write(&managed_file, "test content").unwrap();
    let mut lock = state::new_lock("test", "test-box");
    let resource = Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single("test".to_string()),
        state: None,
        depends_on: vec![],
        provider: None,
        packages: vec![],
        version: None,
        path: Some(managed_file.to_str().unwrap().to_string()),
        content: Some("test content".to_string()),
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
        pre_apply: None,
        post_apply: None,
    };
    let machine = Machine {
        hostname: "localhost".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    let mut ctx = RecordCtx {
        lock: &mut lock,
        state_dir: dir.path(),
        machine_name: "test",
        tripwire: true,
        failure_policy: &FailurePolicy::StopOnFirst,
        timeout_secs: None,
    };

    record_success(&mut ctx, "test-file", &resource, &resource, &machine, 0.1);

    let rl = &ctx.lock.resources["test-file"];
    assert_eq!(rl.status, ResourceStatus::Converged);
    assert!(rl.hash.starts_with("blake3:"));
    assert!(rl.details.contains_key("path"));
    assert!(rl.details.contains_key("content_hash"));

    // Verify event log
    let events_path = dir.path().join("test").join("events.jsonl");
    assert!(events_path.exists());
    let content = std::fs::read_to_string(&events_path).unwrap();
    assert!(content.contains("resource_converged"));
}

#[test]
fn test_fj012_resource_filter() {
    let config = local_config();
    let dir = tempfile::tempdir().unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: dir.path(),
        force: false,
        dry_run: false,
        machine_filter: None,
        resource_filter: Some("nonexistent-resource"),
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
    // Resource filter doesn't match — everything skipped
    assert_eq!(results[0].resources_converged, 0);
    assert_eq!(results[0].resources_unchanged, 0);
}

#[test]
fn test_fj034_parallel_multi_machine() {
    let yaml = r#"
version: "1.0"
name: parallel-test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
  m2:
    hostname: m2
    addr: 127.0.0.1
resources:
  f1:
    type: file
    machine: m1
    path: /tmp/forjar-test-parallel-m1.txt
    content: "m1"
  f2:
    type: file
    machine: m2
    path: /tmp/forjar-test-parallel-m2.txt
    content: "m2"
policy:
  parallel_machines: true
  lock_file: true
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    assert!(config.policy.parallel_machines);

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
    assert_eq!(results.len(), 2);

    // Both machines should converge
    let total_converged: u32 = results.iter().map(|r| r.resources_converged).sum();
    assert_eq!(total_converged, 2, "both files should converge");

    // Verify files exist
    assert!(
        std::path::Path::new("/tmp/forjar-test-parallel-m1.txt").exists(),
        "m1 file should exist"
    );
    assert!(
        std::path::Path::new("/tmp/forjar-test-parallel-m2.txt").exists(),
        "m2 file should exist"
    );

    // Idempotency with parallel
    let r2 = apply(&cfg).unwrap();
    let total_unchanged: u32 = r2.iter().map(|r| r.resources_unchanged).sum();
    assert_eq!(total_unchanged, 2, "both files should be unchanged");

    // Verify locks saved for both machines
    assert!(state::load_lock(dir.path(), "m1").unwrap().is_some());
    assert!(state::load_lock(dir.path(), "m2").unwrap().is_some());

    let _ = std::fs::remove_file("/tmp/forjar-test-parallel-m1.txt");
    let _ = std::fs::remove_file("/tmp/forjar-test-parallel-m2.txt");
}

#[test]
fn test_fj034_single_machine_skips_parallel() {
    // Even with parallel_machines=true, single machine stays sequential
    let yaml = r#"
version: "1.0"
name: single-machine
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: m1
    path: /tmp/forjar-test-single-parallel.txt
    content: "single"
policy:
  parallel_machines: true
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
    assert_eq!(results[0].resources_converged, 1);

    let _ = std::fs::remove_file("/tmp/forjar-test-single-parallel.txt");
}

proptest! {
    /// FALSIFY-ES-002: Jidoka StopOnFirst returns should_stop=true.
    #[test]
    fn falsify_es_002_jidoka_stop_on_first(error in ".{1,50}") {
        let dir = tempfile::tempdir().unwrap();
        let mut lock = state::new_lock("test", "test-box");
        let mut ctx = RecordCtx {
            lock: &mut lock,
            state_dir: dir.path(),
            machine_name: "test",
            tripwire: false,
            failure_policy: &FailurePolicy::StopOnFirst,
            timeout_secs: None,
        };
        let should_stop = record_failure(
            &mut ctx, "res", &ResourceType::Package, 0.1, &error,
        );
        prop_assert!(should_stop, "StopOnFirst must return true");
    }

    /// FALSIFY-ES-003: ContinueIndependent returns should_stop=false.
    #[test]
    fn falsify_es_003_jidoka_continue(error in ".{1,50}") {
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
            &mut ctx, "res", &ResourceType::Package, 0.1, &error,
        );
        prop_assert!(!should_stop, "ContinueIndependent must return false");
    }
}

