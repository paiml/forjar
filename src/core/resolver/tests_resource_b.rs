//! Resource template tests.

use super::resource::resolve_resource_templates;
use super::tests_helpers::{make_base_resource, test_params};
use super::*;
use std::collections::HashMap;

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

// ── PMAT-039: GPU fields must resolve templates ──

#[test]
fn test_pmat039_resolve_driver_version() {
    let params = HashMap::from([(
        "drv".to_string(),
        serde_yaml_ng::Value::String("550".to_string()),
    )]);
    let machines = indexmap::IndexMap::new();
    let mut r = make_base_resource();
    r.driver_version = Some("{{params.drv}}".to_string());
    let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
    assert_eq!(resolved.driver_version.as_deref(), Some("550"));
}

#[test]
fn test_pmat039_resolve_cuda_version() {
    let params = HashMap::from([(
        "cuda".to_string(),
        serde_yaml_ng::Value::String("12.4".to_string()),
    )]);
    let machines = indexmap::IndexMap::new();
    let mut r = make_base_resource();
    r.cuda_version = Some("{{params.cuda}}".to_string());
    let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
    assert_eq!(resolved.cuda_version.as_deref(), Some("12.4"));
}

#[test]
fn test_pmat039_resolve_rocm_version() {
    let params = HashMap::from([(
        "rocm".to_string(),
        serde_yaml_ng::Value::String("6.0".to_string()),
    )]);
    let machines = indexmap::IndexMap::new();
    let mut r = make_base_resource();
    r.rocm_version = Some("{{params.rocm}}".to_string());
    let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
    assert_eq!(resolved.rocm_version.as_deref(), Some("6.0"));
}

#[test]
fn test_pmat039_resolve_gpu_backend() {
    let params = HashMap::from([(
        "backend".to_string(),
        serde_yaml_ng::Value::String("rocm".to_string()),
    )]);
    let machines = indexmap::IndexMap::new();
    let mut r = make_base_resource();
    r.gpu_backend = Some("{{params.backend}}".to_string());
    let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
    assert_eq!(resolved.gpu_backend.as_deref(), Some("rocm"));
}

#[test]
fn test_pmat039_resolve_compute_mode() {
    let params = HashMap::from([(
        "mode".to_string(),
        serde_yaml_ng::Value::String("exclusive_process".to_string()),
    )]);
    let machines = indexmap::IndexMap::new();
    let mut r = make_base_resource();
    r.compute_mode = Some("{{params.mode}}".to_string());
    let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
    assert_eq!(resolved.compute_mode.as_deref(), Some("exclusive_process"));
}
