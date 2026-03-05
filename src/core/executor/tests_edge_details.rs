//! Edge-case tests: build_resource_details variants.

use super::test_fixtures::*;
use super::*;

#[test]
fn test_fj012_build_details_nonexistent_file_no_hash() {
    // content is set but the file doesn't exist → no content_hash
    let resource = Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single("test".to_string()),
        path: Some("/tmp/does-not-exist-forjar-test.txt".to_string()),
        content: Some("ghost".to_string()),
        source: None,
        target: None,
        state: None,
        depends_on: vec![],
        provider: None,
        packages: vec![],
        version: None,
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
    let details = build_resource_details(&resource, &local_machine());
    assert!(
        !details.contains_key("content_hash"),
        "nonexistent file → no hash"
    );
}

#[test]
fn test_fj012_build_details_all_fields() {
    let resource = Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single("test".to_string()),
        path: Some("/etc/app.conf".to_string()),
        owner: Some("app".to_string()),
        group: Some("app".to_string()),
        mode: Some("0600".to_string()),
        name: Some("app-config".to_string()),
        content: None,
        source: None,
        target: None,
        state: None,
        depends_on: vec![],
        provider: None,
        packages: vec![],
        version: None,
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
    let details = build_resource_details(&resource, &local_machine());
    assert_eq!(
        details["path"],
        serde_yaml_ng::Value::String("/etc/app.conf".to_string())
    );
    assert_eq!(
        details["owner"],
        serde_yaml_ng::Value::String("app".to_string())
    );
    assert_eq!(
        details["group"],
        serde_yaml_ng::Value::String("app".to_string())
    );
    assert_eq!(
        details["mode"],
        serde_yaml_ng::Value::String("0600".to_string())
    );
    assert_eq!(
        details["service_name"],
        serde_yaml_ng::Value::String("app-config".to_string())
    );
}

#[test]
fn test_fj012_collect_machines_deduplicates() {
    let yaml = r#"
version: "1.0"
name: dedup
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  a:
    type: file
    machine: m1
    path: /a
  b:
    type: file
    machine: m1
    path: /b
  c:
    type: file
    machine: m1
    path: /c
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let machines = collect_machines(&config);
    assert_eq!(machines.len(), 1, "3 resources on same machine → 1 entry");
    assert_eq!(machines[0], "m1");
}

#[test]
fn test_fj012_collect_machines_preserves_order() {
    let yaml = r#"
version: "1.0"
name: order
machines:
  web:
    hostname: web
    addr: 1.1.1.1
  db:
    hostname: db
    addr: 2.2.2.2
  cache:
    hostname: cache
    addr: 3.3.3.3
resources:
  a:
    type: file
    machine: web
    path: /a
  b:
    type: file
    machine: cache
    path: /b
  c:
    type: file
    machine: db
    path: /c
  d:
    type: file
    machine: web
    path: /d
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let machines = collect_machines(&config);
    assert_eq!(machines, vec!["web", "cache", "db"]);
}

#[test]
fn test_fj012_dry_run_with_machine_filter() {
    let yaml = r#"
version: "1.0"
name: filter-test
machines:
  web:
    hostname: web
    addr: 1.1.1.1
  db:
    hostname: db
    addr: 2.2.2.2
resources:
  web-pkg:
    type: file
    machine: web
    path: /tmp/web
    content: web
  db-pkg:
    type: file
    machine: db
    path: /tmp/db
    content: db
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: dir.path(),
        force: false,
        dry_run: true,
        machine_filter: Some("web"),
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
    // Dry run returns a single result
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].machine, "dry-run");
}

#[test]
fn test_fj012_apply_with_tag_filter() {
    let yaml = r#"
version: "1.0"
name: tag-apply
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  tagged:
    type: file
    machine: local
    path: /tmp/forjar-tagged-test.txt
    content: "tagged content"
    tags: [deploy]
  untagged:
    type: file
    machine: local
    path: /tmp/forjar-untagged-test.txt
    content: "untagged content"
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
        tag_filter: Some("deploy"),
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
    // Only the tagged resource should be applied
    assert_eq!(results[0].resources_converged, 1);
    // The tagged file should exist
    assert!(
        std::path::Path::new("/tmp/forjar-tagged-test.txt").exists(),
        "tagged file should be created"
    );
    // Clean up
    let _ = std::fs::remove_file("/tmp/forjar-tagged-test.txt");
    let _ = std::fs::remove_file("/tmp/forjar-untagged-test.txt");
}

#[test]
fn test_fj012_log_tripwire_enabled() {
    let dir = tempfile::tempdir().unwrap();
    log_tripwire(
        dir.path(),
        "machine1",
        true,
        ProvenanceEvent::ApplyStarted {
            machine: "machine1".to_string(),
            run_id: "test-run".to_string(),
            forjar_version: "0.1.0".to_string(),
            operator: None,
            config_hash: None,
            param_count: None,
        },
    );
    let events = dir.path().join("machine1").join("events.jsonl");
    assert!(events.exists(), "tripwire=true should write event");
}

#[test]
fn test_fj012_log_tripwire_disabled() {
    let dir = tempfile::tempdir().unwrap();
    log_tripwire(
        dir.path(),
        "machine1",
        false,
        ProvenanceEvent::ApplyStarted {
            machine: "machine1".to_string(),
            run_id: "test-run".to_string(),
            forjar_version: "0.1.0".to_string(),
            operator: None,
            config_hash: None,
            param_count: None,
        },
    );
    let events = dir.path().join("machine1").join("events.jsonl");
    assert!(!events.exists(), "tripwire=false should NOT write event");
}
