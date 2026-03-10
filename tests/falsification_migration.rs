//! FJ-044: Docker → pepita migration falsification tests.
//!
//! Popperian rejection criteria for:
//! - Docker→pepita state mapping, port→netns, volumes/env/restart warnings
//! - Full config migration preserving non-Docker resources
//!
//! Usage: cargo test --test falsification_migration

use forjar::core::migrate::{docker_to_pepita, migrate_config};
use forjar::core::types::{ForjarConfig, MachineTarget, Policy, Resource, ResourceType};
use indexmap::IndexMap;
use std::collections::HashMap;

fn empty_config() -> ForjarConfig {
    ForjarConfig {
        version: "1.0".to_string(),
        name: "test".to_string(),
        description: None,
        params: HashMap::new(),
        machines: IndexMap::new(),
        resources: IndexMap::new(),
        policy: Policy::default(),
        outputs: IndexMap::new(),
        policies: vec![],
        data: IndexMap::new(),
        includes: vec![],
        include_provenance: HashMap::new(),
        checks: IndexMap::new(),
        moved: vec![],
        secrets: Default::default(),
        environments: IndexMap::new(),
        dist: None,
    }
}

fn docker_resource(image: &str) -> Resource {
    let mut r = default_resource();
    r.resource_type = ResourceType::Docker;
    r.image = Some(image.to_string());
    r.name = Some("web".to_string());
    r.state = Some("running".to_string());
    r
}

fn package_resource() -> Resource {
    let mut r = default_resource();
    r.resource_type = ResourceType::Package;
    r.packages = vec!["nginx".to_string()];
    r
}

fn default_resource() -> Resource {
    Resource {
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
        repo: None,
        tag: None,
        asset_pattern: None,
        binary: None,
        install_dir: None,
    }
}

// ============================================================================
// FJ-044: Docker → Pepita Conversion
// ============================================================================

#[test]
fn docker_basic_conversion() {
    let docker = docker_resource("nginx:latest");
    let result = docker_to_pepita("web", &docker);
    assert_eq!(result.resource.resource_type, ResourceType::Pepita);
    assert_eq!(result.resource.state.as_deref(), Some("present"));
    assert!(result.resource.image.is_none());
}

#[test]
fn docker_ports_enable_netns() {
    let mut docker = docker_resource("nginx:latest");
    docker.ports = vec!["8080:80".into()];
    let result = docker_to_pepita("web", &docker);
    assert!(result.resource.netns);
    assert!(result.resource.ports.is_empty());
    assert!(result.warnings.iter().any(|w| w.contains("iptables")));
}

#[test]
fn docker_absent_state() {
    let mut docker = docker_resource("nginx:latest");
    docker.state = Some("absent".into());
    let result = docker_to_pepita("old", &docker);
    assert_eq!(result.resource.state.as_deref(), Some("absent"));
}

#[test]
fn docker_stopped_maps_to_absent() {
    let mut docker = docker_resource("app:v1");
    docker.state = Some("stopped".into());
    let result = docker_to_pepita("app", &docker);
    assert_eq!(result.resource.state.as_deref(), Some("absent"));
    assert!(result.warnings.iter().any(|w| w.contains("stopped")));
}

#[test]
fn docker_default_state_maps_to_present() {
    let mut docker = docker_resource("nginx:latest");
    docker.state = None;
    let result = docker_to_pepita("web", &docker);
    assert_eq!(result.resource.state.as_deref(), Some("present"));
}

#[test]
fn docker_unknown_state_warning() {
    let mut docker = docker_resource("app:v1");
    docker.state = Some("restarting".into());
    let result = docker_to_pepita("app", &docker);
    assert_eq!(result.resource.state.as_deref(), Some("present"));
    assert!(result.warnings.iter().any(|w| w.contains("restarting")));
}

#[test]
fn docker_image_warning() {
    let docker = docker_resource("nginx:latest");
    let result = docker_to_pepita("web", &docker);
    assert!(result.warnings.iter().any(|w| w.contains("nginx:latest")));
    assert!(result.warnings.iter().any(|w| w.contains("overlay_lower")));
}

#[test]
fn docker_volumes_warning() {
    let mut docker = docker_resource("postgres:16");
    docker.volumes = vec!["/data:/var/lib/postgresql".into()];
    let result = docker_to_pepita("db", &docker);
    assert!(result.resource.volumes.is_empty());
    assert!(result.warnings.iter().any(|w| w.contains("volumes")));
}

#[test]
fn docker_environment_warning() {
    let mut docker = docker_resource("app:v1");
    docker.environment = vec!["DB_HOST=localhost".into()];
    let result = docker_to_pepita("app", &docker);
    assert!(result.resource.environment.is_empty());
    assert!(result.warnings.iter().any(|w| w.contains("environment")));
}

#[test]
fn docker_restart_warning() {
    let mut docker = docker_resource("nginx:latest");
    docker.restart = Some("unless-stopped".into());
    let result = docker_to_pepita("web", &docker);
    assert!(result.resource.restart.is_none());
    assert!(result.warnings.iter().any(|w| w.contains("restart")));
}

#[test]
fn docker_preserves_depends_on() {
    let mut docker = docker_resource("app:v1");
    docker.depends_on = vec!["db".into()];
    let result = docker_to_pepita("app", &docker);
    assert_eq!(result.resource.depends_on, vec!["db"]);
}

#[test]
fn docker_preserves_tags() {
    let mut docker = docker_resource("nginx:latest");
    docker.tags = vec!["web".into(), "critical".into()];
    let result = docker_to_pepita("web", &docker);
    assert_eq!(result.resource.tags, vec!["web", "critical"]);
}

// ============================================================================
// FJ-044: Full Config Migration
// ============================================================================

#[test]
fn migrate_config_converts_docker() {
    let mut config = empty_config();
    config
        .resources
        .insert("web".into(), docker_resource("nginx:latest"));
    let mut pkg = package_resource();
    pkg.resource_type = ResourceType::Package;
    config.resources.insert("tools".into(), pkg);

    let (migrated, warnings) = migrate_config(&config);
    assert_eq!(
        migrated.resources["web"].resource_type,
        ResourceType::Pepita
    );
    assert_eq!(
        migrated.resources["tools"].resource_type,
        ResourceType::Package
    );
    assert!(!warnings.is_empty());
}

#[test]
fn migrate_config_no_docker_no_warnings() {
    let mut config = empty_config();
    config.resources.insert("pkg".into(), package_resource());

    let (migrated, warnings) = migrate_config(&config);
    assert!(warnings.is_empty());
    assert_eq!(
        migrated.resources["pkg"].resource_type,
        ResourceType::Package
    );
}

#[test]
fn migrate_config_empty() {
    let config = empty_config();
    let (migrated, warnings) = migrate_config(&config);
    assert!(migrated.resources.is_empty());
    assert!(warnings.is_empty());
}
