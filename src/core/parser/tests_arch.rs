//! Architecture tests (FJ-064).

use super::*;

#[test]
fn test_fj064_resource_arch_filter_parsed() {
    let yaml = r#"
version: "1.0"
name: arch-test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  gpu-driver:
    type: package
    machine: m1
    provider: apt
    packages: [nvidia-driver]
    arch: [x86_64]
"#;
    let config = parse_config(yaml).unwrap();
    assert_eq!(config.resources["gpu-driver"].arch, vec!["x86_64"]);
}

#[test]
fn test_fj064_resource_arch_multi() {
    let yaml = r#"
version: "1.0"
name: arch-test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  common:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
    arch: [x86_64, aarch64]
"#;
    let config = parse_config(yaml).unwrap();
    assert_eq!(config.resources["common"].arch, vec!["x86_64", "aarch64"]);
}

#[test]
fn test_fj064_resource_arch_empty_default() {
    let yaml = r#"
version: "1.0"
name: arch-test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#;
    let config = parse_config(yaml).unwrap();
    assert!(config.resources["pkg"].arch.is_empty());
}

#[test]
fn test_fj064_invalid_resource_arch() {
    let yaml = r#"
version: "1.0"
name: arch-test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
    arch: [sparc]
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors.iter().any(|e| e.message.contains("sparc")));
}

#[test]
fn test_fj064_invalid_machine_arch() {
    let yaml = r#"
version: "1.0"
name: arch-test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
    arch: mips
resources: {}
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors.iter().any(|e| e.message.contains("mips")));
}

#[test]
fn test_fj064_valid_machine_arch_aarch64() {
    let yaml = r#"
version: "1.0"
name: arch-test
machines:
  edge:
    hostname: jetson
    addr: 10.0.0.1
    arch: aarch64
resources: {}
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.is_empty(),
        "aarch64 should be valid, got: {:?}",
        errors
    );
}

#[test]
fn test_fj002_all_valid_arch_values_accepted() {
    for arch in &["x86_64", "aarch64", "armv7l", "riscv64", "s390x", "ppc64le"] {
        let yaml = format!(
            r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
    arch: {}
resources: {{}}
"#,
            arch
        );
        let config = parse_config(&yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors.is_empty(),
            "arch '{}' should be valid but got errors: {:?}",
            arch,
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }
}
