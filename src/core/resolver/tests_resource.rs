//! Resource template tests.

use super::resource::resolve_resource_templates;
use super::template::resolve_template;
use super::tests_helpers::{make_base_resource, test_params};
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
        pre_apply: None,
        post_apply: None,
        lifecycle: None,
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
        pre_apply: None,
        post_apply: None,
        lifecycle: None,
    };

    let resolved = resolve_resource_templates(&resource, &params, &machines).unwrap();
    assert_eq!(resolved.group.as_deref(), Some("www-data"));
    assert_eq!(resolved.mode.as_deref(), Some("0644"));
}


#[test]
fn test_resolve_port_template() {
    let params = HashMap::from([(
        "port".to_string(),
        serde_yaml_ng::Value::String("8080".to_string()),
    )]);
    let machines = indexmap::IndexMap::new();
    let mut r = make_base_resource();
    r.port = Some("{{params.port}}".to_string());
    let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
    assert_eq!(resolved.port.as_deref(), Some("8080"));
}

#[test]
fn test_resolve_command_template() {
    let params = HashMap::from([(
        "host".to_string(),
        serde_yaml_ng::Value::String("localhost".to_string()),
    )]);
    let machines = indexmap::IndexMap::new();
    let mut r = make_base_resource();
    r.command = Some("curl http://{{params.host}}/health".to_string());
    let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
    assert_eq!(
        resolved.command.as_deref(),
        Some("curl http://localhost/health")
    );
}

#[test]
fn test_resolve_image_template() {
    let params = HashMap::from([(
        "tag".to_string(),
        serde_yaml_ng::Value::String("v2.1".to_string()),
    )]);
    let machines = indexmap::IndexMap::new();
    let mut r = make_base_resource();
    r.image = Some("myapp:{{params.tag}}".to_string());
    let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
    assert_eq!(resolved.image.as_deref(), Some("myapp:v2.1"));
}

#[test]
fn test_resolve_ports_list_template() {
    let params = HashMap::from([(
        "port".to_string(),
        serde_yaml_ng::Value::String("9090".to_string()),
    )]);
    let machines = indexmap::IndexMap::new();
    let mut r = make_base_resource();
    r.ports = vec!["{{params.port}}:8080".to_string(), "443:443".to_string()];
    let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
    assert_eq!(resolved.ports, vec!["9090:8080", "443:443"]);
}

#[test]
fn test_resolve_environment_list_template() {
    let params = HashMap::from([(
        "env".to_string(),
        serde_yaml_ng::Value::String("prod".to_string()),
    )]);
    let machines = indexmap::IndexMap::new();
    let mut r = make_base_resource();
    r.environment = vec!["APP_ENV={{params.env}}".to_string()];
    let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
    assert_eq!(resolved.environment, vec!["APP_ENV=prod"]);
}

#[test]
fn test_resolve_volumes_list_template() {
    let params = HashMap::from([(
        "data".to_string(),
        serde_yaml_ng::Value::String("/data/app".to_string()),
    )]);
    let machines = indexmap::IndexMap::new();
    let mut r = make_base_resource();
    r.volumes = vec!["{{params.data}}:/app/data:ro".to_string()];
    let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
    assert_eq!(resolved.volumes, vec!["/data/app:/app/data:ro"]);
}

#[test]
fn test_resolve_packages_list_template() {
    let params = HashMap::from([(
        "pkg".to_string(),
        serde_yaml_ng::Value::String("htop".to_string()),
    )]);
    let machines = indexmap::IndexMap::new();
    let mut r = make_base_resource();
    r.packages = vec!["curl".to_string(), "{{params.pkg}}".to_string()];
    let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
    assert_eq!(resolved.packages, vec!["curl", "htop"]);
}


#[test]
fn test_fj131_resolve_source_field() {
    let params = test_params();
    let machines = indexmap::IndexMap::new();
    let mut r = make_base_resource();
    r.source = Some("/data/{{params.val}}/file.txt".to_string());
    let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
    assert_eq!(resolved.source.as_deref(), Some("/data/resolved/file.txt"));
}

#[test]
fn test_fj131_resolve_target_field() {
    let params = test_params();
    let machines = indexmap::IndexMap::new();
    let mut r = make_base_resource();
    r.target = Some("/opt/{{params.val}}/bin".to_string());
    let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
    assert_eq!(resolved.target.as_deref(), Some("/opt/resolved/bin"));
}

#[test]
fn test_fj131_resolve_options_field() {
    let params = test_params();
    let machines = indexmap::IndexMap::new();
    let mut r = make_base_resource();
    r.options = Some("rw,{{params.val}}".to_string());
    let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
    assert_eq!(resolved.options.as_deref(), Some("rw,resolved"));
}

#[test]
fn test_fj131_resolve_protocol_field() {
    let params = HashMap::from([(
        "proto".to_string(),
        serde_yaml_ng::Value::String("tcp".to_string()),
    )]);
    let machines = indexmap::IndexMap::new();
    let mut r = make_base_resource();
    r.protocol = Some("{{params.proto}}".to_string());
    let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
    assert_eq!(resolved.protocol.as_deref(), Some("tcp"));
}

#[test]
fn test_fj131_resolve_action_field() {
    let params = HashMap::from([(
        "act".to_string(),
        serde_yaml_ng::Value::String("allow".to_string()),
    )]);
    let machines = indexmap::IndexMap::new();
    let mut r = make_base_resource();
    r.action = Some("{{params.act}}".to_string());
    let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
    assert_eq!(resolved.action.as_deref(), Some("allow"));
}

#[test]
fn test_fj131_resolve_from_addr_field() {
    let params = HashMap::from([(
        "cidr".to_string(),
        serde_yaml_ng::Value::String("10.0.0.0/24".to_string()),
    )]);
    let machines = indexmap::IndexMap::new();
    let mut r = make_base_resource();
    r.from_addr = Some("{{params.cidr}}".to_string());
    let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
    assert_eq!(resolved.from_addr.as_deref(), Some("10.0.0.0/24"));
}

#[test]
fn test_fj131_resolve_shell_field() {
    let params = test_params();
    let machines = indexmap::IndexMap::new();
    let mut r = make_base_resource();
    r.shell = Some("/bin/{{params.val}}".to_string());
    let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
    assert_eq!(resolved.shell.as_deref(), Some("/bin/resolved"));
}

#[test]
fn test_fj131_resolve_home_field() {
    let params = HashMap::from([(
        "user".to_string(),
        serde_yaml_ng::Value::String("deploy".to_string()),
    )]);
    let machines = indexmap::IndexMap::new();
    let mut r = make_base_resource();
    r.home = Some("/home/{{params.user}}".to_string());
    let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
    assert_eq!(resolved.home.as_deref(), Some("/home/deploy"));
}

#[test]
fn test_fj131_resolve_restart_field() {
    let params = HashMap::from([(
        "policy".to_string(),
        serde_yaml_ng::Value::String("unless-stopped".to_string()),
    )]);
    let machines = indexmap::IndexMap::new();
    let mut r = make_base_resource();
    r.restart = Some("{{params.policy}}".to_string());
    let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
    assert_eq!(resolved.restart.as_deref(), Some("unless-stopped"));
}

#[test]
fn test_fj131_resolve_version_field() {
    let params = HashMap::from([(
        "ver".to_string(),
        serde_yaml_ng::Value::String("2.1.0".to_string()),
    )]);
    let machines = indexmap::IndexMap::new();
    let mut r = make_base_resource();
    r.version = Some("{{params.ver}}".to_string());
    let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
    assert_eq!(resolved.version.as_deref(), Some("2.1.0"));
}


#[test]
fn test_fj132_resolve_resource_templates_list_fields() {
    let mut params = HashMap::new();
    params.insert(
        "port".to_string(),
        serde_yaml_ng::Value::String("8080".to_string()),
    );
    let machines = indexmap::IndexMap::new();
    let mut resource = make_base_resource();
    resource.resource_type = ResourceType::Docker;
    resource.ports = vec!["{{params.port}}:{{params.port}}".to_string()];
    resource.environment = vec!["PORT={{params.port}}".to_string()];
    let resolved = resolve_resource_templates(&resource, &params, &machines).unwrap();
    assert_eq!(resolved.ports, vec!["8080:8080"]);
    assert_eq!(resolved.environment, vec!["PORT=8080"]);
}

