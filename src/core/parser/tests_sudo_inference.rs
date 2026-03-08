use super::*;

/// Issue #42: owner: root without sudo: true should warn.
#[test]
fn test_sudo_inference_owner_root_warns() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 10.0.0.1
    user: deploy
resources:
  sudoers:
    type: file
    machine: m1
    path: /tmp/sudoers
    content: sudoers-file
    owner: root
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let msgs: Vec<&str> = errors.iter().map(|e| e.message.as_str()).collect();
    assert!(
        msgs.iter()
            .any(|m| m.contains("owner: root") && m.contains("sudo: true")),
        "should warn about owner: root without sudo: {msgs:?}"
    );
}

/// Issue #42: file under /etc/ without sudo: true should warn.
#[test]
fn test_sudo_inference_privileged_path_warns() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 10.0.0.1
    user: deploy
resources:
  nginx-conf:
    type: file
    machine: m1
    path: /etc/nginx/nginx.conf
    content: "server {}"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    let msgs: Vec<&str> = errors.iter().map(|e| e.message.as_str()).collect();
    assert!(
        msgs.iter()
            .any(|m| m.contains("privileged path") && m.contains("sudo: true")),
        "should warn about /etc/ path without sudo: {msgs:?}"
    );
}

/// Issue #42: sudo: true suppresses the warning.
#[test]
fn test_sudo_inference_with_sudo_true_no_warn() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 10.0.0.1
    user: deploy
resources:
  nginx-conf:
    type: file
    machine: m1
    path: /etc/nginx/nginx.conf
    content: "server {}"
    sudo: true
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.is_empty(),
        "sudo: true should suppress warning: {errors:?}"
    );
}

/// Issue #42: user=root on machine means no sudo needed.
#[test]
fn test_sudo_inference_root_user_no_warn() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 10.0.0.1
    user: root
resources:
  nginx-conf:
    type: file
    machine: m1
    path: /etc/nginx/nginx.conf
    content: "server {}"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.is_empty(),
        "user=root should not need sudo: {errors:?}"
    );
}

/// Issue #42: non-privileged paths don't need sudo.
#[test]
fn test_sudo_inference_non_privileged_path_no_warn() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 10.0.0.1
    user: deploy
resources:
  app-conf:
    type: file
    machine: m1
    path: /home/deploy/.config/app.conf
    content: "setting=1"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.is_empty(),
        "non-privileged path should not warn: {errors:?}"
    );
}

/// Issue #42: only file resources trigger sudo inference, not packages.
#[test]
fn test_sudo_inference_package_type_no_warn() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 10.0.0.1
    user: deploy
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.is_empty(),
        "package type should not trigger sudo inference: {errors:?}"
    );
}

/// Issue #42: /usr/lib/systemd/ is a privileged path.
#[test]
fn test_sudo_inference_systemd_path() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 10.0.0.1
    user: deploy
resources:
  svc-unit:
    type: file
    machine: m1
    path: /usr/lib/systemd/system/myapp.service
    content: "[Unit]\nDescription=My App"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.message.contains("privileged path")),
        "systemd path should trigger sudo inference: {errors:?}"
    );
}

/// Issue #42: /opt/ is a privileged path.
#[test]
fn test_sudo_inference_opt_path() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 10.0.0.1
    user: deploy
resources:
  app-bin:
    type: file
    machine: m1
    path: /opt/myapp/config.yaml
    content: "key: value"
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.message.contains("privileged path")),
        "/opt/ should trigger sudo inference: {errors:?}"
    );
}

// --- Issue #48: kernel headers auto-include validation ---

/// Installing linux-image without matching linux-headers should warn.
#[test]
fn test_kernel_image_without_headers_warns() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 10.0.0.1
resources:
  hwe-kernel:
    type: package
    machine: m1
    provider: apt
    packages:
      - linux-image-generic-hwe-22.04
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("linux-headers-generic-hwe-22.04")),
        "should warn about missing headers: {errors:?}"
    );
}

/// Including both image and headers should not warn.
#[test]
fn test_kernel_image_with_headers_no_warn() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 10.0.0.1
resources:
  hwe-kernel:
    type: package
    machine: m1
    provider: apt
    packages:
      - linux-image-generic-hwe-22.04
      - linux-headers-generic-hwe-22.04
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        !errors.iter().any(|e| e.message.contains("linux-headers")),
        "should not warn when headers included: {errors:?}"
    );
}

/// Non-kernel packages should not trigger kernel header warnings.
#[test]
fn test_non_kernel_package_no_warn() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 10.0.0.1
resources:
  tools:
    type: package
    machine: m1
    provider: apt
    packages: [curl, wget]
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        !errors.iter().any(|e| e.message.contains("linux-headers")),
        "non-kernel packages should not trigger warning: {errors:?}"
    );
}
