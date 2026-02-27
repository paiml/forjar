//! Remaining tests: container validation, edge cases, recipe expansion integration tests.

use super::*;

#[test]
fn test_fj002_container_transport_requires_container_block() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
resources: {}
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("no 'container' block")));
}

#[test]
fn test_fj002_container_ephemeral_requires_image() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      runtime: docker
      ephemeral: true
resources: {}
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("ephemeral but has no container image")));
}

#[test]
fn test_fj002_container_invalid_runtime() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      runtime: lxc
      image: ubuntu:22.04
resources: {}
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("must be 'docker' or 'podman'")));
}

#[test]
fn test_fj002_container_valid_config() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      runtime: docker
      image: ubuntu:22.04
      ephemeral: true
resources: {}
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
fn test_fj002_container_podman_valid() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      runtime: podman
      image: ubuntu:22.04
resources: {}
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
fn test_network_invalid_protocol() {
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
    port: "22"
    protocol: sctp
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("invalid protocol 'sctp'")));
}

#[test]
fn test_network_invalid_action() {
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
    port: "80"
    action: block
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("invalid action 'block'")));
}

#[test]
fn test_docker_invalid_state() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  db:
    type: docker
    machine: m1
    name: postgres
    image: postgres:16
    state: paused
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("invalid state 'paused'")));
}

#[test]
fn test_cron_schedule_must_have_5_fields() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  job:
    type: cron
    machine: m1
    name: bad-job
    schedule: "0 2 * *"
    command: /usr/bin/backup
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("must have exactly 5 fields")));
}

#[test]
fn test_cron_valid_schedule() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  job:
    type: cron
    machine: m1
    name: good-job
    schedule: "0 2 * * *"
    command: /usr/bin/backup
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        !errors.iter().any(|e| e.message.contains("5 fields")),
        "valid 5-field schedule should pass"
    );
}

#[test]
fn test_cron_invalid_state() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  job:
    type: cron
    machine: m1
    name: bad
    schedule: "* * * * *"
    command: echo hi
    state: disabled
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("invalid state 'disabled'")));
}

#[test]
fn test_cron_absent_skips_schedule_and_command() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  old-job:
    type: cron
    machine: m1
    name: old-job
    state: absent
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        !errors.iter().any(|e| e.message.contains("no schedule")),
        "absent cron should not require schedule"
    );
    assert!(
        !errors.iter().any(|e| e.message.contains("no command")),
        "absent cron should not require command"
    );
}

#[test]
fn test_cron_schedule_too_many_fields() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  job:
    type: cron
    machine: m1
    name: bad-job
    schedule: "0 2 * * * *"
    command: echo hi
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("must have exactly 5 fields")));
}

#[test]
fn test_fj002_docker_absent_skips_image_requirement() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  old-container:
    type: docker
    machine: m1
    name: old-container
    state: absent
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        !errors.iter().any(|e| e.message.contains("no image")),
        "docker state=absent should not require image"
    );
}

#[test]
fn test_fj002_docker_running_requires_image() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  no-image:
    type: docker
    machine: m1
    name: no-image
    state: running
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.message.contains("no image")),
        "docker state=running must require image"
    );
}

#[test]
fn test_fj002_mount_both_missing_gives_two_errors() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  bad-mount:
    type: mount
    machine: m1
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let mount_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("bad-mount"))
        .collect();
    assert!(
        mount_errors.iter().any(|e| e.message.contains("no source")),
        "should report missing source"
    );
    assert!(
        mount_errors.iter().any(|e| e.message.contains("no path")),
        "should report missing path"
    );
    assert!(
        mount_errors.len() >= 2,
        "mount with both missing should produce >=2 errors"
    );
}

#[test]
fn test_fj002_network_reject_is_valid_action() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  fw-rule:
    type: network
    machine: m1
    port: 443
    protocol: tcp
    action: reject
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        !errors.iter().any(|e| e.message.contains("invalid action")),
        "'reject' should be a valid network action"
    );
}

#[test]
fn test_fj002_network_invalid_protocol() {
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
    port: 80
    protocol: icmp
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("invalid protocol")));
}

#[test]
fn test_fj002_recipe_missing_recipe_name() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  my-recipe:
    type: recipe
    machine: m1
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.message.contains("no recipe name")),
        "recipe without recipe field should error"
    );
}
