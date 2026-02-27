//! FJ-132 (continued), FJ-036 structural tests.

use super::*;

#[test]
fn test_fj132_validate_symlink_no_target() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  bad-link:
    type: file
    machine: m
    path: /usr/local/bin/myapp
    state: symlink
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("symlink requires a target")),
        "symlink without target should error"
    );
}

#[test]
fn test_fj132_validate_unknown_arch() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
    arch: mips64
resources: {}
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.message.contains("unknown arch")),
        "unknown architecture should error"
    );
}

#[test]
fn test_fj132_validate_service_invalid_state() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  svc:
    type: service
    machine: m
    name: nginx
    state: restarted
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("invalid state 'restarted'")),
        "invalid service state should error"
    );
}

#[test]
fn test_fj132_parse_config_invalid_yaml() {
    let result = parse_config("{{{{bad yaml");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("YAML parse error"));
}

// ---- FJ-036 tests ----

#[test]
fn test_fj036_parse_minimal_config() {
    let yaml = r#"
version: "1.0"
name: minimal
machines:
  m1:
    hostname: box
    addr: 10.0.0.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#;
    let config = parse_config(yaml).unwrap();
    assert_eq!(config.version, "1.0");
    assert_eq!(config.name, "minimal");
    assert_eq!(config.machines.len(), 1);
    assert!(config.machines.contains_key("m1"));
    assert_eq!(config.resources.len(), 1);
    assert!(config.resources.contains_key("pkg"));
    let errors = validate_config(&config);
    assert!(
        errors.is_empty(),
        "minimal valid config should have no errors: {:?}",
        errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_fj036_parse_multiple_machines() {
    let yaml = r#"
version: "1.0"
name: multi-machine
machines:
  web:
    hostname: web-01
    addr: 10.0.0.1
  db:
    hostname: db-01
    addr: 10.0.0.2
  cache:
    hostname: cache-01
    addr: 10.0.0.3
resources:
  web-pkg:
    type: package
    machine: web
    provider: apt
    packages: [nginx]
  db-pkg:
    type: package
    machine: db
    provider: apt
    packages: [postgresql]
  cache-pkg:
    type: package
    machine: cache
    provider: apt
    packages: [redis-server]
"#;
    let config = parse_config(yaml).unwrap();
    assert_eq!(config.machines.len(), 3);
    assert!(config.machines.contains_key("web"));
    assert!(config.machines.contains_key("db"));
    assert!(config.machines.contains_key("cache"));
    assert_eq!(config.machines["web"].hostname, "web-01");
    assert_eq!(config.machines["db"].hostname, "db-01");
    assert_eq!(config.machines["cache"].hostname, "cache-01");
    let errors = validate_config(&config);
    assert!(
        errors.is_empty(),
        "multi-machine config should validate: {:?}",
        errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_fj036_validate_duplicate_depends() {
    let yaml = r#"
version: "1.0"
name: self-dep
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  circular:
    type: file
    machine: m1
    path: /etc/circular.conf
    content: "loop"
    depends_on: [circular]
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("depends on itself")),
        "resource depending on itself should produce error, got: {:?}",
        errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_fj036_parse_with_all_resource_types() {
    let yaml = r#"
version: "1.0"
name: all-types
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
  conf:
    type: file
    machine: m1
    path: /etc/app.conf
    content: "key=value"
  svc:
    type: service
    machine: m1
    name: nginx
    state: running
  mnt:
    type: mount
    machine: m1
    source: /dev/sda1
    path: /mnt/data
  deploy-user:
    type: user
    machine: m1
    name: deploy
  web-container:
    type: docker
    machine: m1
    name: web
    image: nginx:latest
  backup-job:
    type: cron
    machine: m1
    name: backup
    schedule: "0 2 * * *"
    command: /usr/bin/backup
  firewall:
    type: network
    machine: m1
    port: "443"
    protocol: tcp
    action: allow
  sandbox:
    type: pepita
    machine: m1
    name: sandbox
    state: present
"#;
    let config = parse_config(yaml).unwrap();
    assert_eq!(config.resources.len(), 9);
    let errors = validate_config(&config);
    assert!(
        errors.is_empty(),
        "config with all 9 resource types should validate: {:?}",
        errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
    assert_eq!(config.resources["pkg"].resource_type, ResourceType::Package);
    assert_eq!(config.resources["conf"].resource_type, ResourceType::File);
    assert_eq!(config.resources["svc"].resource_type, ResourceType::Service);
    assert_eq!(config.resources["mnt"].resource_type, ResourceType::Mount);
    assert_eq!(
        config.resources["deploy-user"].resource_type,
        ResourceType::User
    );
    assert_eq!(
        config.resources["web-container"].resource_type,
        ResourceType::Docker
    );
    assert_eq!(
        config.resources["backup-job"].resource_type,
        ResourceType::Cron
    );
    assert_eq!(
        config.resources["firewall"].resource_type,
        ResourceType::Network
    );
    assert_eq!(
        config.resources["sandbox"].resource_type,
        ResourceType::Pepita
    );
}
