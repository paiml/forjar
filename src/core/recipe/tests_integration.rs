//! Integration and edge case tests (FJ-132, FJ-036, resource inputs, metadata).

#![allow(unused_imports)]
use std::collections::HashMap;
use std::path::Path;

use crate::core::types::MachineTarget;

use super::expansion::{
    expand_recipe, load_recipe, parse_recipe, resolve_input_template, resolve_resource_inputs,
};
use super::types::RecipeSource;
use super::validation::validate_inputs;

const RECIPE_YAML: &str = r#"
recipe:
  name: nfs-server
  version: "1.0"
  description: "NFS server recipe"
  inputs:
    export_path:
      type: path
      description: "Path to export"
    network:
      type: string
      default: "192.168.50.0/24"
    port:
      type: int
      default: 2049
      min: 1024
      max: 65535

resources:
  packages:
    type: package
    provider: apt
    packages: [nfs-kernel-server]

  exports:
    type: file
    path: /etc/exports
    content: "{{inputs.export_path}} {{inputs.network}}(rw,sync)"
    depends_on: [packages]

  service:
    type: service
    name: nfs-kernel-server
    state: running
    enabled: true
    restart_on: [exports]
    depends_on: [packages, exports]
"#;

#[test]
fn test_fj019_resolve_resource_inputs_target_and_options() {
    use crate::core::types::{MachineTarget, Resource, ResourceType};

    let resource = Resource {
        resource_type: ResourceType::Mount,
        machine: MachineTarget::Single("m1".to_string()),
        state: None,
        depends_on: vec![],
        provider: None,
        packages: vec![],
        version: None,
        path: Some("/mnt/{{inputs.vol}}".to_string()),
        content: None,
        source: Some("{{inputs.server}}:/data".to_string()),
        target: Some("/mnt/{{inputs.vol}}/sub".to_string()),
        owner: None,
        group: None,
        mode: None,
        name: None,
        enabled: None,
        restart_on: vec![],
        triggers: vec![],
        fs_type: None,
        options: Some("ro,{{inputs.extra}}".to_string()),
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
    };
    let mut inputs = HashMap::new();
    inputs.insert("vol".to_string(), "raid".to_string());
    inputs.insert("server".to_string(), "nas01".to_string());
    inputs.insert("extra".to_string(), "hard".to_string());

    let resolved = resolve_resource_inputs(&resource, &inputs).unwrap();
    assert_eq!(resolved.path.as_deref(), Some("/mnt/raid"));
    assert_eq!(resolved.source.as_deref(), Some("nas01:/data"));
    assert_eq!(resolved.target.as_deref(), Some("/mnt/raid/sub"));
    assert_eq!(resolved.options.as_deref(), Some("ro,hard"));
}

#[test]
fn test_fj019_resolve_resource_inputs_content_field() {
    use crate::core::types::{MachineTarget, Resource, ResourceType};

    let resource = Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single("m1".to_string()),
        state: None,
        depends_on: vec![],
        provider: None,
        packages: vec![],
        version: None,
        path: None,
        content: Some("user={{inputs.user}}".to_string()),
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
        build_machine: None,
    };
    let mut inputs = HashMap::new();
    inputs.insert("user".to_string(), "admin".to_string());
    let resolved = resolve_resource_inputs(&resource, &inputs).unwrap();
    assert_eq!(resolved.content.as_deref(), Some("user=admin"));
}

#[test]
fn test_fj019_recipe_source_debug_clone() {
    let local = RecipeSource::Local {
        path: "recipes/test.yaml".to_string(),
    };
    let cloned = local.clone();
    let _ = format!("{cloned:?}");

    let git = RecipeSource::Git {
        git: "https://github.com/example/recipes.git".to_string(),
        r#ref: Some("main".to_string()),
        path: Some("nfs.yaml".to_string()),
    };
    let cloned = git.clone();
    let _ = format!("{cloned:?}");
}

#[test]
fn test_fj019_recipe_metadata_optional_fields() {
    let yaml = r#"
recipe:
  name: minimal
resources: {}
"#;
    let recipe = parse_recipe(yaml).unwrap();
    assert!(recipe.recipe.version.is_none());
    assert!(recipe.recipe.description.is_none());
    assert!(recipe.recipe.requires.is_empty());
}

#[test]
fn test_fj019_recipe_with_requires() {
    let yaml = r#"
recipe:
  name: app-stack
  requires:
    - recipe: web-server
    - recipe: database
  inputs: {}
resources: {}
"#;
    let recipe = parse_recipe(yaml).unwrap();
    assert_eq!(recipe.recipe.requires.len(), 2);
    assert_eq!(recipe.recipe.requires[0].recipe, "web-server");
    assert_eq!(recipe.recipe.requires[1].recipe, "database");
}

// --- FJ-132: Recipe edge case tests ---

#[test]
fn test_fj132_expand_recipe_namespaces_resource_ids() {
    let recipe = parse_recipe(RECIPE_YAML).unwrap();
    let machine = MachineTarget::Single("test".to_string());
    let mut inputs = HashMap::new();
    inputs.insert(
        "export_path".to_string(),
        serde_yaml_ng::Value::String("/mnt/data".to_string()),
    );
    inputs.insert(
        "network".to_string(),
        serde_yaml_ng::Value::String("10.0.0.0/8".to_string()),
    );
    let expanded = expand_recipe("web", &recipe, &machine, &inputs, &[]).unwrap();
    for key in expanded.keys() {
        assert!(
            key.starts_with("web/"),
            "expanded key '{key}' should be namespaced with 'web/'"
        );
    }
}

#[test]
fn test_fj132_expand_recipe_sets_machine() {
    let recipe = parse_recipe(RECIPE_YAML).unwrap();
    let machine = MachineTarget::Single("prod-web".to_string());
    let mut inputs = HashMap::new();
    inputs.insert(
        "export_path".to_string(),
        serde_yaml_ng::Value::String("/mnt".to_string()),
    );
    inputs.insert(
        "network".to_string(),
        serde_yaml_ng::Value::String("10.0.0.0/8".to_string()),
    );
    let expanded = expand_recipe("stack", &recipe, &machine, &inputs, &[]).unwrap();
    for resource in expanded.values() {
        match &resource.machine {
            MachineTarget::Single(name) => assert_eq!(name, "prod-web"),
            _ => panic!("expected Single machine target"),
        }
    }
}

#[test]
fn test_fj132_expand_namespaces_depends_on() {
    let recipe = parse_recipe(RECIPE_YAML).unwrap();
    let machine = MachineTarget::Single("m".to_string());
    let mut inputs = HashMap::new();
    inputs.insert(
        "export_path".to_string(),
        serde_yaml_ng::Value::String("/mnt".to_string()),
    );
    inputs.insert(
        "network".to_string(),
        serde_yaml_ng::Value::String("10.0.0.0/8".to_string()),
    );
    let expanded = expand_recipe("nfs", &recipe, &machine, &inputs, &[]).unwrap();
    let svc = &expanded["nfs/service"];
    assert!(svc.depends_on.contains(&"nfs/packages".to_string()));
    assert!(svc.depends_on.contains(&"nfs/exports".to_string()));
}

#[test]
fn test_fj132_expand_namespaces_restart_on() {
    let recipe = parse_recipe(RECIPE_YAML).unwrap();
    let machine = MachineTarget::Single("m".to_string());
    let mut inputs = HashMap::new();
    inputs.insert(
        "export_path".to_string(),
        serde_yaml_ng::Value::String("/mnt".to_string()),
    );
    inputs.insert(
        "network".to_string(),
        serde_yaml_ng::Value::String("10.0.0.0/8".to_string()),
    );
    let expanded = expand_recipe("nfs", &recipe, &machine, &inputs, &[]).unwrap();
    let svc = &expanded["nfs/service"];
    assert!(
        svc.restart_on.contains(&"nfs/exports".to_string()),
        "restart_on should be namespaced: {:?}",
        svc.restart_on
    );
}

#[test]
fn test_fj132_resolve_input_preserves_non_template() {
    let result = resolve_input_template("no templates here", &HashMap::new()).unwrap();
    assert_eq!(result, "no templates here");
}

// -- FJ-036 tests --

#[test]
fn test_fj036_expand_recipe_namespaces_resources() {
    let yaml = r#"
recipe:
  name: mystack
  inputs:
    dir:
      type: path
resources:
  install:
    type: package
    provider: apt
    packages: [nginx]
  config:
    type: file
    path: /etc/nginx/nginx.conf
    content: "root {{inputs.dir}}"
    depends_on: [install]
"#;
    let recipe = parse_recipe(yaml).unwrap();
    let machine = MachineTarget::Single("web1".to_string());
    let mut inputs = HashMap::new();
    inputs.insert(
        "dir".to_string(),
        serde_yaml_ng::Value::String("/var/www".to_string()),
    );

    let expanded = expand_recipe("mystack", &recipe, &machine, &inputs, &[]).unwrap();

    assert_eq!(expanded.len(), 2);
    for key in expanded.keys() {
        assert!(key.starts_with("mystack/"));
        assert!(key.contains('/'));
    }
    assert!(expanded.contains_key("mystack/install"));
    assert!(expanded.contains_key("mystack/config"));
}

#[test]
fn test_fj036_expand_recipe_inherits_machine() {
    let recipe = parse_recipe(RECIPE_YAML).unwrap();
    let machine = MachineTarget::Single("staging-box".to_string());
    let mut inputs = HashMap::new();
    inputs.insert(
        "export_path".to_string(),
        serde_yaml_ng::Value::String("/mnt/staging".to_string()),
    );

    let expanded = expand_recipe("nfs", &recipe, &machine, &inputs, &[]).unwrap();

    for (key, resource) in &expanded {
        let machines = resource.machine.to_vec();
        assert_eq!(
            machines,
            vec!["staging-box"],
            "resource '{key}' should inherit machine 'staging-box', got {machines:?}"
        );
    }
}

#[test]
fn test_fj036_load_recipe_file_missing() {
    let result = load_recipe(Path::new("/tmp/does-not-exist/recipe-fj036.yaml"));
    assert!(result.is_err(), "missing file should produce an error");
    let err = result.unwrap_err();
    assert!(
        err.contains("cannot read recipe"),
        "error should mention cannot read recipe: {err}"
    );
}
