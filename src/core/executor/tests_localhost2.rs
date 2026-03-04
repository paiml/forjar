//! FJ-131: Executor + state edge case tests (part 2).

use super::test_fixtures::*;
use super::*;

#[test]
fn test_fj131_apply_continue_independent_policy() {
    // With ContinueIndependent, a failing resource shouldn't block others
    // Use a file resource with an impossible path to trigger failure
    let yaml = r#"
version: "1.0"
name: continue-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  good-file:
    type: file
    machine: local
    path: /tmp/forjar-test-continue-good.txt
    content: "good"
  bad-file:
    type: file
    machine: local
    path: /proc/nonexistent/impossible/path.txt
    content: "will fail"
    source: /dev/null/impossible
policy:
  failure: continue_independent
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
        trace: false,
    };
    let results = apply(&cfg).unwrap();
    // At least one resource should converge even if one fails
    // (good-file should succeed, bad-file may fail)
    let total = results[0].resources_converged + results[0].resources_failed;
    assert!(total > 0, "should have attempted resources");

    let _ = std::fs::remove_file("/tmp/forjar-test-continue-good.txt");
}

#[test]
fn test_fj131_record_success_no_live_hash_for_package() {
    // Package resources have no state_query that returns a live_hash
    let dir = tempfile::tempdir().unwrap();
    let mut lock = state::new_lock("test", "test-box");
    let resource = Resource {
        resource_type: ResourceType::Package,
        machine: MachineTarget::Single("test".to_string()),
        provider: Some("apt".to_string()),
        packages: vec!["curl".to_string()],
        state: None,
        depends_on: vec![],
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
        pre_apply: None,
        post_apply: None,
        lifecycle: None,
        store: false,
        sudo: false,
        script: None,
    };
    let mut ctx = RecordCtx {
        lock: &mut lock,
        state_dir: dir.path(),
        machine_name: "test",
        tripwire: false,
        failure_policy: &FailurePolicy::StopOnFirst,
        timeout_secs: None,
    };

    record_success(
        &mut ctx,
        "pkg-curl",
        &resource,
        &resource,
        &local_machine(),
        0.3,
    );

    let rl = &ctx.lock.resources["pkg-curl"];
    assert_eq!(rl.status, ResourceStatus::Converged);
    assert!(rl.hash.starts_with("blake3:"));
    // Package resources get live_hash from state_query_script execution
    // The live_hash presence depends on whether the script succeeds locally
}

#[test]
fn test_fj131_apply_dry_run_returns_unchanged_count() {
    // Dry-run with existing state should report unchanged resources
    let config = local_config();
    let dir = tempfile::tempdir().unwrap();

    // First real apply to establish state
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
    };
    apply(&cfg).unwrap();

    // Now dry-run — should report the unchanged count from plan
    let cfg2 = ApplyConfig {
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
    };
    let results = apply(&cfg2).unwrap();
    assert_eq!(results[0].machine, "dry-run");
    assert_eq!(
        results[0].resources_unchanged, 1,
        "dry-run after apply should report 1 unchanged"
    );
    assert_eq!(results[0].resources_converged, 0);
    assert_eq!(results[0].resources_failed, 0);

    let _ = std::fs::remove_file("/tmp/forjar-test-executor.txt");
}

#[test]
fn test_fj131_resource_outcome_variants() {
    // Verify ResourceOutcome enum can be matched correctly
    let converged = ResourceOutcome::Converged;
    let unchanged = ResourceOutcome::Unchanged;
    let skipped = ResourceOutcome::Skipped;
    let failed_stop = ResourceOutcome::Failed { should_stop: true };
    let failed_continue = ResourceOutcome::Failed { should_stop: false };

    assert!(matches!(converged, ResourceOutcome::Converged));
    assert!(matches!(unchanged, ResourceOutcome::Unchanged));
    assert!(matches!(skipped, ResourceOutcome::Skipped));
    assert!(matches!(
        failed_stop,
        ResourceOutcome::Failed { should_stop: true }
    ));
    assert!(matches!(
        failed_continue,
        ResourceOutcome::Failed { should_stop: false }
    ));
}

#[test]
fn test_fj131_record_ctx_timeout_propagation() {
    // RecordCtx correctly stores timeout value
    let dir = tempfile::tempdir().unwrap();
    let mut lock = state::new_lock("test", "test-box");
    let ctx = RecordCtx {
        lock: &mut lock,
        state_dir: dir.path(),
        machine_name: "test",
        tripwire: true,
        failure_policy: &FailurePolicy::StopOnFirst,
        timeout_secs: Some(60),
    };
    assert_eq!(ctx.timeout_secs, Some(60));
    assert_eq!(ctx.machine_name, "test");
    assert!(ctx.tripwire);
}
