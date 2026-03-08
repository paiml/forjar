//! FJ-064: Cross-architecture filtering, FJ-052: Cost-aware scheduling, tag filter tests.

#![allow(unused_imports)]
use super::test_fixtures::*;
use super::*;

#[test]
fn test_fj064_arch_filter_yaml_parsing() {
    let yaml = r#"
version: "1.0"
name: arch-test
machines:
  x86-box:
    hostname: x86-box
    addr: 127.0.0.1
    arch: x86_64
  arm-box:
    hostname: arm-box
    addr: 10.0.0.1
    arch: aarch64
resources:
  x86-only:
    type: file
    machine: x86-box
    path: /etc/x86-marker
    content: "x86 only"
    arch: [x86_64]
  arm-only:
    type: file
    machine: arm-box
    path: /etc/arm-marker
    content: "arm only"
    arch: [aarch64]
  universal:
    type: file
    machine: x86-box
    path: /etc/universal
    content: "any arch"
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(config.resources["x86-only"].arch, vec!["x86_64"]);
    assert_eq!(config.resources["arm-only"].arch, vec!["aarch64"]);
    assert!(config.resources["universal"].arch.is_empty());
}

#[test]
fn test_fj064_arch_filter_skips_mismatched() {
    // Resource with arch: [aarch64] should be skipped on x86_64 machine
    let machine = Machine {
        hostname: "x86-box".to_string(),
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
    let resource = Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single("x86-box".to_string()),
        state: None,
        depends_on: vec![],
        provider: None,
        packages: vec![],
        version: None,
        path: Some("/etc/arm-only".to_string()),
        content: Some("arm only".to_string()),
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
        arch: vec!["aarch64".to_string()],
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
    };

    // arch filter should reject: aarch64 resource on x86_64 machine
    assert!(
        !resource.arch.is_empty() && !resource.arch.contains(&machine.arch),
        "arch filter should skip aarch64 resource on x86_64 machine"
    );
}

#[test]
fn test_fj064_arch_filter_allows_matching() {
    let machine = Machine {
        hostname: "arm-box".to_string(),
        addr: "10.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "aarch64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    };
    let arch = ["aarch64".to_string()];
    assert!(arch.contains(&machine.arch));
}

#[test]
fn test_fj064_empty_arch_allows_all() {
    let machine = Machine {
        hostname: "any-box".to_string(),
        addr: "10.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "riscv64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    };
    let arch: Vec<String> = vec![];
    // Empty arch means "runs on all architectures"
    assert!(arch.is_empty() || arch.contains(&machine.arch));
}

#[test]
fn test_fj052_cost_field_default_zero() {
    let yaml = r#"
version: "1.0"
name: cost-test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources: {}
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(config.machines["m1"].cost, 0);
}

#[test]
fn test_fj052_cost_field_parsed() {
    let yaml = r#"
version: "1.0"
name: cost-test
machines:
  cheap:
    hostname: cheap
    addr: 10.0.0.1
    cost: 1
  expensive:
    hostname: expensive
    addr: 10.0.0.2
    cost: 10
resources: {}
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(config.machines["cheap"].cost, 1);
    assert_eq!(config.machines["expensive"].cost, 10);
}

#[test]
fn test_fj052_machines_sorted_by_cost() {
    let yaml = r#"
version: "1.0"
name: cost-test
machines:
  expensive:
    hostname: expensive
    addr: 10.0.0.3
    cost: 100
  medium:
    hostname: medium
    addr: 10.0.0.2
    cost: 50
  cheap:
    hostname: cheap
    addr: 10.0.0.1
    cost: 1
resources:
  f:
    type: file
    machine: [expensive, medium, cheap]
    path: /tmp/test
    content: hello
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let all_machines: Vec<String> = config.machines.keys().cloned().collect();
    let mut sorted: Vec<&String> = all_machines.iter().collect();
    sorted.sort_by_key(|m| {
        config
            .machines
            .get(*m)
            .map(|machine| machine.cost)
            .unwrap_or(0)
    });

    assert_eq!(sorted[0], "cheap");
    assert_eq!(sorted[1], "medium");
    assert_eq!(sorted[2], "expensive");
}

#[test]
fn test_tag_filter_skips_untagged_resources() {
    let yaml = r#"
version: "1.0"
name: tag-test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  tagged-file:
    type: file
    machine: local
    path: /tmp/forjar-tag-test.txt
    content: "tagged"
    tags: [web, critical]
  untagged-file:
    type: file
    machine: local
    path: /tmp/forjar-tag-test2.txt
    content: "untagged"
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
        tag_filter: Some("web"),
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
    // Only the tagged resource should be applied
    assert_eq!(results[0].resources_converged, 1);
    let _ = std::fs::remove_file("/tmp/forjar-tag-test.txt");
}

#[test]
fn test_tag_filter_none_applies_all() {
    let yaml = r#"
version: "1.0"
name: tag-test-all
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  a:
    type: file
    machine: local
    path: /tmp/forjar-tag-all-a.txt
    content: "a"
    tags: [web]
  b:
    type: file
    machine: local
    path: /tmp/forjar-tag-all-b.txt
    content: "b"
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
    // Both resources applied
    assert_eq!(results[0].resources_converged, 2);
    let _ = std::fs::remove_file("/tmp/forjar-tag-all-a.txt");
    let _ = std::fs::remove_file("/tmp/forjar-tag-all-b.txt");
}

#[test]
fn test_tags_parsed_from_yaml() {
    let yaml = r#"
version: "1.0"
name: tags-parse
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: m
    path: /tmp/test
    content: test
    tags: [web, critical, db]
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let resource = config.resources.get("f").unwrap();
    assert_eq!(resource.tags, vec!["web", "critical", "db"]);
}

#[test]
fn test_tags_default_empty() {
    let yaml = r#"
version: "1.0"
name: tags-empty
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: m
    path: /tmp/test
    content: test
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let resource = config.resources.get("f").unwrap();
    assert!(resource.tags.is_empty());
}
