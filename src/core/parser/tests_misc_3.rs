//! FJ-131/FJ-132 parser edge case tests, FJ-036 structural tests.

use super::*;
use std::path::Path;

// ---- FJ-131: Parser edge case tests ----

#[test]
fn test_fj131_parse_and_validate_valid_config() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(
        &config_path,
        r#"
version: "1.0"
name: valid-config
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: /tmp/test.txt
    content: "hello"
"#,
    )
    .unwrap();

    let config = parse_and_validate(&config_path).unwrap();
    assert_eq!(config.name, "valid-config");
    assert!(config.resources.contains_key("test-file"));
}

#[test]
fn test_fj131_parse_and_validate_error_formatting() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(
        &config_path,
        r#"
version: "1.0"
name: bad-config
machines: {}
resources:
  bad-pkg:
    type: package
    machine: unknown-machine
  bad-file:
    type: file
    machine: another-unknown
"#,
    )
    .unwrap();

    let err = parse_and_validate(&config_path).unwrap_err();
    assert!(
        err.starts_with("validation errors:\n"),
        "error should start with 'validation errors:'"
    );
    assert!(
        err.contains("  - "),
        "each error should be indented with '  - '"
    );
    let bullet_count = err.matches("  - ").count();
    assert!(
        bullet_count >= 2,
        "expected multiple errors, got {bullet_count} bullets"
    );
}

#[test]
fn test_fj131_parse_and_validate_nonexistent_file() {
    let result = parse_and_validate(Path::new("/tmp/nonexistent-forjar-config.yaml"));
    assert!(result.is_err());
}

#[test]
fn test_fj131_package_both_missing_provider_and_packages() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 1.1.1.1
resources:
  bad-pkg:
    type: package
    machine: m
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let pkg_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("bad-pkg"))
        .collect();
    assert!(
        pkg_errors.len() >= 2,
        "should have at least 2 errors for missing packages AND provider, got {}",
        pkg_errors.len()
    );
    assert!(pkg_errors.iter().any(|e| e.message.contains("no packages")));
    assert!(pkg_errors.iter().any(|e| e.message.contains("no provider")));
}

#[test]
fn test_fj131_file_invalid_state_error_lists_valid_options() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 1.1.1.1
resources:
  bad-file:
    type: file
    machine: m
    path: /tmp/test
    state: "executable"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let file_err = errors
        .iter()
        .find(|e| e.message.contains("invalid state"))
        .expect("should have invalid state error");
    assert!(file_err
        .message
        .contains("file, directory, symlink, absent"));
}

#[test]
fn test_fj131_container_valid_config_no_errors() {
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
      ephemeral: false
      name: my-container
resources:
  f:
    type: file
    machine: test-box
    path: /tmp/test
    content: "hello"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.is_empty(),
        "valid container config should have no errors: {errors:?}"
    );
}

#[test]
fn test_fj131_service_invalid_state_lists_valid_options() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 1.1.1.1
resources:
  svc:
    type: service
    machine: m
    name: nginx
    state: "paused"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let svc_err = errors
        .iter()
        .find(|e| e.message.contains("invalid state"))
        .expect("should have invalid state error");
    assert!(svc_err
        .message
        .contains("running, stopped, enabled, disabled"));
}

#[test]
fn test_fj131_mount_invalid_state_lists_valid_options() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 1.1.1.1
resources:
  mnt:
    type: mount
    machine: m
    source: /dev/sda1
    path: /mnt/data
    state: "bound"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let mnt_err = errors
        .iter()
        .find(|e| e.message.contains("invalid state"))
        .expect("should have invalid state error");
    assert!(mnt_err.message.contains("mounted, unmounted, absent"));
}

#[test]
fn test_fj131_cron_both_missing_schedule_and_command() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 1.1.1.1
resources:
  bad-cron:
    type: cron
    machine: m
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let cron_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("bad-cron"))
        .collect();
    assert!(
        cron_errors.len() >= 3,
        "expected at least 3 cron errors, got {}",
        cron_errors.len()
    );
}

#[test]
fn test_fj131_network_both_invalid_protocol_and_action() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 1.1.1.1
resources:
  bad-net:
    type: network
    machine: m
    port: "22"
    protocol: icmp
    action: forward
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let net_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("bad-net"))
        .collect();
    assert!(
        net_errors.len() >= 2,
        "expected at least 2 network errors, got {}",
        net_errors.len()
    );
    assert!(net_errors.iter().any(|e| e.message.contains("protocol")));
    assert!(net_errors.iter().any(|e| e.message.contains("action")));
}

#[test]
fn test_fj131_parse_and_validate_with_recipe_expansion() {
    let dir = tempfile::tempdir().unwrap();

    let recipes_dir = dir.path().join("recipes");
    std::fs::create_dir_all(&recipes_dir).unwrap();
    std::fs::write(
        recipes_dir.join("web.yaml"),
        r#"
recipe:
  name: web-recipe
resources:
  web-file:
    type: file
    path: /tmp/web.txt
    content: "web"
"#,
    )
    .unwrap();

    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(
        &config_path,
        r#"
version: "1.0"
name: recipe-test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  web:
    type: recipe
    machine: local
    recipe: web
"#,
    )
    .unwrap();

    let config = parse_and_validate(&config_path).unwrap();
    assert!(
        !config.resources.contains_key("web"),
        "recipe resource should be replaced"
    );
    assert!(
        config.resources.keys().any(|k| k.contains("web-file")),
        "expanded resource should be present"
    );
}

// ---- FJ-132 tests ----

#[test]
fn test_fj132_validate_container_no_block() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  box:
    hostname: box
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
fn test_fj132_validate_container_bad_runtime() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  box:
    hostname: box
    addr: container
    transport: container
    container:
      runtime: lxc
      image: ubuntu:22.04
resources: {}
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("docker' or 'podman'")),
        "invalid runtime should error"
    );
}

#[test]
fn test_fj132_validate_ephemeral_no_image() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  box:
    hostname: box
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
        "ephemeral without image should error"
    );
}

#[test]
fn test_fj132_validate_file_both_content_and_source() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  bad-file:
    type: file
    machine: m
    path: /etc/test.conf
    content: "inline"
    source: /builds/app.conf
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("both content and source")),
        "file with both content and source should error"
    );
}
