//! Tests for config, machine, and container types (FJ-001, FJ-131, FJ-132).

use super::*;

#[test]
fn test_fj001_config_parse() {
    let yaml = r#"
version: "1.0"
name: test-infra
params:
  raid_path: /mnt/raid
machines:
  lambda:
    hostname: lambda-box
    addr: 192.168.1.1
    user: noah
    arch: x86_64
    roles: [gpu-compute]
resources:
  test-pkg:
    type: package
    machine: lambda
    provider: apt
    packages: [curl, wget]
policy:
  failure: stop_on_first
  tripwire: true
  lock_file: true
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(config.version, "1.0");
    assert_eq!(config.name, "test-infra");
    assert_eq!(config.machines.len(), 1);
    assert_eq!(config.machines["lambda"].hostname, "lambda-box");
    assert_eq!(config.resources.len(), 1);
    assert_eq!(
        config.resources["test-pkg"].resource_type,
        ResourceType::Package
    );
}

#[test]
fn test_fj001_machine_defaults() {
    let yaml = r#"
hostname: test
addr: 1.2.3.4
"#;
    let m: Machine = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(m.user, "root");
    assert_eq!(m.arch, "x86_64");
    assert!(m.roles.is_empty());
    assert!(m.transport.is_none());
    assert!(m.container.is_none());
}

#[test]
fn test_fj001_container_config_defaults() {
    let yaml = r#"
runtime: docker
image: ubuntu:22.04
"#;
    let c: ContainerConfig = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(c.runtime, "docker");
    assert_eq!(c.image.as_deref(), Some("ubuntu:22.04"));
    assert!(c.name.is_none());
    assert!(c.ephemeral);
    assert!(!c.privileged);
    assert!(c.init);
}

#[test]
fn test_fj001_container_machine_parse() {
    let yaml = r#"
hostname: test-box
addr: container
transport: container
container:
  runtime: docker
  image: ubuntu:22.04
  name: forjar-test
  ephemeral: true
  privileged: false
  init: true
"#;
    let m: Machine = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(m.transport.as_deref(), Some("container"));
    assert!(m.is_container_transport());
    let c = m.container.unwrap();
    assert_eq!(c.runtime, "docker");
    assert_eq!(c.image.as_deref(), Some("ubuntu:22.04"));
    assert_eq!(c.name.as_deref(), Some("forjar-test"));
}

#[test]
fn test_fj001_machine_container_name_derived() {
    let m = Machine {
        hostname: "test-box".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "docker".to_string(),
            image: Some("ubuntu:22.04".to_string()),
            name: None,
            ephemeral: true,
            privileged: false,
            init: true,
        }),
        pepita: None,
        cost: 0,
    };
    assert_eq!(m.container_name(), "forjar-test-box");
}

#[test]
fn test_fj001_is_container_transport() {
    // Explicit transport field
    let m1 = Machine {
        hostname: "t".to_string(),
        addr: "1.2.3.4".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: None,
        pepita: None,
        cost: 0,
    };
    assert!(m1.is_container_transport());

    // Sentinel addr
    let m2 = Machine {
        hostname: "t".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    assert!(m2.is_container_transport());

    // Normal machine
    let m3 = Machine {
        hostname: "t".to_string(),
        addr: "10.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    assert!(!m3.is_container_transport());
}

#[test]
fn test_fj001_policy_defaults() {
    let p = Policy::default();
    assert_eq!(p.failure, FailurePolicy::StopOnFirst);
    assert!(p.tripwire);
    assert!(p.lock_file);
    assert!(!p.parallel_machines);
}

#[test]
fn test_fj001_multi_machine_resource() {
    let yaml = r#"
version: "1.0"
name: multi
machines:
  a:
    hostname: a
    addr: 1.1.1.1
  b:
    hostname: b
    addr: 2.2.2.2
resources:
  tools:
    type: package
    machine: [a, b]
    provider: cargo
    packages: [batuta]
policy:
  failure: stop_on_first
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let targets = config.resources["tools"].machine.to_vec();
    assert_eq!(targets, vec!["a", "b"]);
}

#[test]
fn test_fj131_policy_with_hooks() {
    let yaml = r#"
failure: continue_independent
tripwire: false
lock_file: false
parallel_machines: true
pre_apply: "echo pre"
post_apply: "echo post"
"#;
    let p: Policy = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(p.failure, FailurePolicy::ContinueIndependent);
    assert!(!p.tripwire);
    assert!(!p.lock_file);
    assert!(p.parallel_machines);
    assert_eq!(p.pre_apply.as_deref(), Some("echo pre"));
    assert_eq!(p.post_apply.as_deref(), Some("echo post"));
}

#[test]
fn test_fj131_container_config_default_runtime() {
    // When runtime is omitted, should default to "docker"
    let yaml = r#"
image: alpine:3.19
"#;
    let c: ContainerConfig = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(c.runtime, "docker");
    assert!(c.ephemeral);
    assert!(c.init);
}

#[test]
fn test_fj131_container_config_podman_non_ephemeral() {
    let yaml = r#"
runtime: podman
name: my-container
ephemeral: false
privileged: true
init: false
"#;
    let c: ContainerConfig = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(c.runtime, "podman");
    assert!(c.image.is_none());
    assert_eq!(c.name.as_deref(), Some("my-container"));
    assert!(!c.ephemeral);
    assert!(c.privileged);
    assert!(!c.init);
}

#[test]
fn test_fj131_machine_cost_default_zero() {
    let yaml = r#"
hostname: m
addr: 1.2.3.4
"#;
    let m: Machine = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(m.cost, 0);
}

#[test]
fn test_fj131_machine_cost_explicit() {
    let yaml = r#"
hostname: gpu
addr: 10.0.0.1
cost: 100
"#;
    let m: Machine = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(m.cost, 100);
}

#[test]
fn test_fj131_machine_ssh_key() {
    let yaml = r#"
hostname: remote
addr: 10.0.0.5
ssh_key: ~/.ssh/deploy_ed25519
"#;
    let m: Machine = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(m.ssh_key.as_deref(), Some("~/.ssh/deploy_ed25519"));
}

#[test]
fn test_fj131_machine_container_name_no_container_block() {
    // container_name() on machine without container block falls back to hostname
    let m = Machine {
        hostname: "bare-metal".to_string(),
        addr: "10.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    assert_eq!(m.container_name(), "forjar-bare-metal");
}

#[test]
fn test_fj131_config_minimal_defaults() {
    // Minimal config with all defaults
    let yaml = r#"
version: "1.0"
name: minimal
resources:
  f:
    type: file
    path: /tmp/test
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    assert!(config.description.is_none());
    assert!(config.params.is_empty());
    assert!(config.machines.is_empty());
    assert!(config.policy.tripwire); // default true
    assert!(config.policy.lock_file); // default true
}

#[test]
fn test_fj131_machine_roles_parse() {
    let yaml = r#"
hostname: gpu-01
addr: 10.0.0.5
roles: [gpu-compute, training, inference]
"#;
    let m: Machine = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(m.roles.len(), 3);
    assert_eq!(m.roles[0], "gpu-compute");
}

#[test]
fn test_fj132_container_config_ephemeral_default_true() {
    let yaml = r#"
runtime: docker
image: ubuntu:22.04
"#;
    let c: ContainerConfig = serde_yaml_ng::from_str(yaml).unwrap();
    assert!(c.ephemeral, "ephemeral should default to true");
    assert!(c.init, "init should default to true");
    assert!(!c.privileged, "privileged should default to false");
}

#[test]
fn test_fj132_machine_is_container_transport() {
    let m = Machine {
        hostname: "box".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: None,
        pepita: None,
        cost: 0,
    };
    assert!(m.is_container_transport());
}

#[test]
fn test_fj132_machine_is_not_container_transport() {
    let m = Machine {
        hostname: "web".to_string(),
        addr: "10.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    assert!(!m.is_container_transport());
}

#[test]
fn test_fj132_machine_container_name_explicit() {
    let m = Machine {
        hostname: "box".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "docker".to_string(),
            image: Some("ubuntu:22.04".to_string()),
            name: Some("my-custom-name".to_string()),
            ephemeral: true,
            privileged: false,
            init: true,
        }),
        pepita: None,
        cost: 0,
    };
    assert_eq!(m.container_name(), "my-custom-name");
}

#[test]
fn test_fj132_machine_container_name_derived() {
    let m = Machine {
        hostname: "test-box".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "docker".to_string(),
            image: Some("ubuntu:22.04".to_string()),
            name: None,
            ephemeral: true,
            privileged: false,
            init: true,
        }),
        pepita: None,
        cost: 0,
    };
    assert_eq!(m.container_name(), "forjar-test-box");
}

#[test]
fn test_fj132_policy_defaults() {
    let policy = Policy::default();
    assert!(matches!(policy.failure, FailurePolicy::StopOnFirst));
    assert!(policy.tripwire);
    assert!(policy.lock_file);
    assert!(!policy.parallel_machines);
}
