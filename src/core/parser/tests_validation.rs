//! Resource type validation tests.

use super::*;

#[test]
fn test_fj002_package_no_packages() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: []
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors.iter().any(|e| e.message.contains("no packages")));
}

#[test]
fn test_fj002_file_no_path() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  f:
    type: file
    machine: m1
    content: "hello"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors.iter().any(|e| e.message.contains("no path")));
}

#[test]
fn test_fj035_file_content_and_source_exclusive() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  f:
    type: file
    machine: m1
    path: /etc/config
    content: "inline content"
    source: /local/path/config.txt
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("both content and source")));
}

#[test]
fn test_fj002_service_no_name() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  svc:
    type: service
    machine: m1
    state: running
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors.iter().any(|e| e.message.contains("no name")));
}

#[test]
fn test_fj002_package_no_provider() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  pkg:
    type: package
    machine: m1
    packages: [curl]
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors.iter().any(|e| e.message.contains("no provider")));
}

#[test]
fn test_fj002_mount_no_source_or_path() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  mnt:
    type: mount
    machine: m1
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors.iter().any(|e| e.message.contains("has no source")));
    assert!(errors.iter().any(|e| e.message.contains("has no path")));
}

/// BH-MUT-0001: Kill mutation of `machine_name != "localhost"`.
#[test]
fn test_fj002_user_no_name() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  u:
    type: user
    machine: m1
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("(user) has no name")));
}

#[test]
fn test_fj002_docker_no_name() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  d:
    type: docker
    machine: m1
    image: nginx:latest
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("(docker) has no name")));
}

#[test]
fn test_fj002_docker_no_image() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  d:
    type: docker
    machine: m1
    name: web
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("(docker) has no image")));
}

#[test]
fn test_fj002_cron_no_schedule() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  c:
    type: cron
    machine: m1
    name: job
    command: /bin/true
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("(cron) has no schedule")));
}

#[test]
fn test_fj002_cron_no_command() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  c:
    type: cron
    machine: m1
    name: job
    schedule: "0 * * * *"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("(cron) has no command")));
}

#[test]
fn test_fj002_network_no_port() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  fw:
    type: network
    machine: m1
    action: allow
    protocol: tcp
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("(network) has no port")));
}

#[test]
fn test_fj002_user_valid() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  deploy-user:
    type: user
    machine: m1
    name: deploy
    shell: /bin/bash
    groups: [docker, sudo]
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.is_empty(),
        "unexpected errors: {:?}",
        errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_fj002_docker_valid() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  web:
    type: docker
    machine: m1
    name: web
    image: nginx:latest
    ports: ["8080:80"]
    environment: ["ENV=prod"]
    restart: unless-stopped
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.is_empty(),
        "unexpected errors: {:?}",
        errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_file_invalid_state() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  f:
    type: file
    machine: m1
    path: /tmp/x
    state: bogus
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("invalid state 'bogus'")));
}

#[test]
fn test_file_symlink_requires_target() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  link:
    type: file
    machine: m1
    path: /usr/local/bin/tool
    state: symlink
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("symlink requires a target")));
}

#[test]
fn test_file_valid_states() {
    for state in &["file", "directory", "symlink", "absent"] {
        let yaml = format!(
            r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  f:
    type: file
    machine: m1
    path: /tmp/x
    state: {state}
    target: /tmp/y
"#
        );
        let config = parse_config(&yaml).unwrap();
        let errors = validate_config(&config);
        let state_errors: Vec<_> = errors
            .iter()
            .filter(|e| e.message.contains("invalid state"))
            .collect();
        assert!(state_errors.is_empty(), "state '{state}' should be valid");
    }
}

#[test]
fn test_service_invalid_state() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  svc:
    type: service
    machine: m1
    name: nginx
    state: restarting
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("invalid state 'restarting'")));
}

#[test]
fn test_service_valid_states() {
    for state in &["running", "stopped", "enabled", "disabled"] {
        let yaml = format!(
            r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  svc:
    type: service
    machine: m1
    name: nginx
    state: {state}
"#
        );
        let config = parse_config(&yaml).unwrap();
        let errors = validate_config(&config);
        let state_errors: Vec<_> = errors
            .iter()
            .filter(|e| e.message.contains("invalid state"))
            .collect();
        assert!(state_errors.is_empty(), "state '{state}' should be valid");
    }
}

#[test]
fn test_mount_invalid_state() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  mnt:
    type: mount
    machine: m1
    source: /dev/sda1
    path: /mnt/data
    state: attached
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("invalid state 'attached'")));
}

#[test]
fn test_mount_missing_source_only() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  mnt:
    type: mount
    machine: m1
    path: /mnt/data
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors.iter().any(|e| e.message.contains("has no source")));
    assert!(!errors.iter().any(|e| e.message.contains("has no path")));
}
