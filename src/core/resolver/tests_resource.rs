//! Resource template tests.

use super::resource::resolve_resource_templates;
use super::template::resolve_template;
use super::*;
use std::collections::HashMap;

#[test]
fn test_fj003_resolve_all_fields() {
    let mut params = HashMap::new();
    params.insert(
        "dir".to_string(),
        serde_yaml_ng::Value::String("/data".to_string()),
    );
    let mut machines = indexmap::IndexMap::new();
    machines.insert(
        "m1".to_string(),
        Machine {
            hostname: "m1-box".to_string(),
            addr: "10.0.0.1".to_string(),
            user: "deploy".to_string(),
            arch: "aarch64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
            allowed_operators: vec![],
        },
    );

    let resource = Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single("m1".to_string()),
        state: None,
        depends_on: vec![],
        provider: None,
        packages: vec![],
        version: None,
        path: Some("{{params.dir}}/config".to_string()),
        content: Some("host={{machine.m1.hostname}}".to_string()),
        source: Some("{{machine.m1.addr}}:/src".to_string()),
        target: Some("{{params.dir}}/link".to_string()),
        owner: Some("{{machine.m1.user}}".to_string()),
        group: None,
        mode: None,
        name: Some("{{machine.m1.hostname}}-svc".to_string()),
        enabled: None,
        restart_on: vec![],
        triggers: vec![],
        fs_type: None,
        options: Some("{{machine.m1.arch}}".to_string()),
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

    let resolved = resolve_resource_templates(&resource, &params, &machines).unwrap();
    assert_eq!(resolved.path.as_deref(), Some("/data/config"));
    assert_eq!(resolved.content.as_deref(), Some("host=m1-box"));
    assert_eq!(resolved.source.as_deref(), Some("10.0.0.1:/src"));
    assert_eq!(resolved.target.as_deref(), Some("/data/link"));
    assert_eq!(resolved.owner.as_deref(), Some("deploy"));
    assert_eq!(resolved.name.as_deref(), Some("m1-box-svc"));
    assert_eq!(resolved.options.as_deref(), Some("aarch64"));
}

#[test]
fn test_fj003_resolve_machine_fields() {
    let params = HashMap::new();
    let mut machines = indexmap::IndexMap::new();
    machines.insert(
        "srv".to_string(),
        Machine {
            hostname: "srv-01".to_string(),
            addr: "192.168.1.1".to_string(),
            user: "admin".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
            allowed_operators: vec![],
        },
    );

    // Test all machine field branches
    assert_eq!(
        resolve_template("{{machine.srv.hostname}}", &params, &machines).unwrap(),
        "srv-01"
    );
    assert_eq!(
        resolve_template("{{machine.srv.user}}", &params, &machines).unwrap(),
        "admin"
    );
    assert_eq!(
        resolve_template("{{machine.srv.arch}}", &params, &machines).unwrap(),
        "x86_64"
    );

    // Unknown field
    let err = resolve_template("{{machine.srv.bogus}}", &params, &machines);
    assert!(err.is_err());
    assert!(err.unwrap_err().contains("unknown machine field"));

    // Invalid machine ref format
    let err = resolve_template("{{machine.srv}}", &params, &machines);
    assert!(err.is_err());
    assert!(err.unwrap_err().contains("invalid machine ref"));
}

#[test]
fn test_fj003_resolve_resource_templates_group_and_mode() {
    let mut params = HashMap::new();
    params.insert(
        "grp".to_string(),
        serde_yaml_ng::Value::String("www-data".to_string()),
    );
    params.insert(
        "perm".to_string(),
        serde_yaml_ng::Value::String("0644".to_string()),
    );
    let machines = indexmap::IndexMap::new();

    let resource = Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single("m1".to_string()),
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
        group: Some("{{params.grp}}".to_string()),
        mode: Some("{{params.perm}}".to_string()),
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

    let resolved = resolve_resource_templates(&resource, &params, &machines).unwrap();
    assert_eq!(resolved.group.as_deref(), Some("www-data"));
    assert_eq!(resolved.mode.as_deref(), Some("0644"));
}
