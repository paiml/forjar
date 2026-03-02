//! Edge-case validation tests, duplicate FJ-002 coverage, recipe integration.

use super::*;

#[test]
fn test_fj002_unknown_arch_in_resource() {
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
    packages: [vim]
    provider: apt
    arch: [mips64]
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.message.contains("unknown arch")),
        "mips64 should be an unknown arch"
    );
}

#[test]
fn test_fj002_unknown_arch_in_machine() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
    arch: sparc64
resources: {}
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.message.contains("unknown arch")),
        "sparc64 should be an unknown machine arch"
    );
}

#[test]
fn test_fj002_container_transport_missing_block() {
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
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("no 'container' block")),
        "container transport without container block should error"
    );
}

#[test]
fn test_fj002_container_runtime_containerd_rejected() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      runtime: containerd
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
fn test_fj002_container_ephemeral_no_image() {
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
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("ephemeral but has no container image")),
        "ephemeral container without image should error"
    );
}

#[test]
fn test_fj002_self_dependency_detected() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  loopy:
    type: file
    machine: m1
    path: /etc/loopy
    depends_on: [loopy]
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("depends on itself")),
        "self-dependency should be caught"
    );
}

#[test]
fn test_fj002_depends_on_unknown_resource() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  web:
    type: file
    machine: m1
    path: /etc/nginx.conf
    depends_on: [ghost-resource]
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("unknown resource 'ghost-resource'")));
}

#[test]
fn test_fj002_file_both_content_and_source() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  both:
    type: file
    machine: m1
    path: /etc/both
    content: "hello"
    source: ./local.txt
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("both content and source")));
}

#[test]
fn test_fj002_file_symlink_without_target() {
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
fn test_fj002_localhost_machine_ref_always_valid() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  local-file:
    type: file
    machine: localhost
    path: /tmp/local
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        !errors.iter().any(|e| e.message.contains("unknown machine")),
        "'localhost' should be accepted without being in machines map"
    );
}

#[test]
fn test_fj002_service_invalid_state() {
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
    state: restarted
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.message.contains("invalid state")),
        "'restarted' is not a valid service state"
    );
}

#[test]
fn test_fj002_mount_invalid_state() {
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
    state: enabled
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.message.contains("invalid state")),
        "'enabled' is not a valid mount state"
    );
}

#[test]
fn test_fj002_cron_invalid_state() {
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
    schedule: "0 2 * * *"
    command: echo hi
    state: running
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.message.contains("invalid state")),
        "'running' is not a valid cron state"
    );
}

#[test]
fn test_fj002_docker_invalid_state() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  c:
    type: docker
    machine: m1
    name: c
    image: nginx
    state: paused
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.message.contains("invalid state")),
        "'paused' is not a valid docker state"
    );
}

// ---- Recipe expansion integration tests ----

#[test]
fn test_expand_recipes_replaces_recipe_resources() {
    let dir = tempfile::tempdir().unwrap();
    let recipes_dir = dir.path().join("recipes");
    std::fs::create_dir_all(&recipes_dir).unwrap();
    std::fs::write(
        recipes_dir.join("test-recipe.yaml"),
        r#"
recipe:
  name: test-recipe
  inputs:
    greeting:
      type: string
      default: hello
resources:
  config-file:
    type: file
    path: /etc/test.conf
    content: "{{inputs.greeting}} world"
"#,
    )
    .unwrap();

    let yaml = r#"
version: "1.0"
name: recipe-test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  setup:
    type: recipe
    machine: m1
    recipe: test-recipe
    inputs:
      greeting: hi
"#;
    let mut config = parse_config(yaml).unwrap();
    expand_recipes(&mut config, Some(dir.path())).unwrap();

    assert!(!config.resources.contains_key("setup"));
    assert!(config.resources.contains_key("setup/config-file"));

    let file_res = &config.resources["setup/config-file"];
    assert_eq!(file_res.resource_type, ResourceType::File);
    assert_eq!(file_res.content.as_deref(), Some("hi world"));
    assert_eq!(file_res.machine.to_vec(), vec!["m1"]);
}
