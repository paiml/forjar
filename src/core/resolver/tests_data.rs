use super::data::resolve_data_sources;
use super::template::resolve_template;
use super::*;

#[test]
fn test_fj223_data_source_file() {
    let dir = tempfile::tempdir().unwrap();
    let data_file = dir.path().join("version.txt");
    std::fs::write(&data_file, "1.2.3\n").unwrap();

    let yaml = format!(
        r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/version
    content: "v={{{{data.app_version}}}}"
data:
  app_version:
    type: file
    value: "{}"
"#,
        data_file.display()
    );
    let mut config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    resolve_data_sources(&mut config).unwrap();

    // Should have injected __data__app_version
    let val = config.params.get("__data__app_version").unwrap();
    assert_eq!(yaml_value_to_string(val), "1.2.3");
}

#[test]
fn test_fj223_data_source_command() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
data:
  hostname:
    type: command
    value: "echo test-host"
"#;
    let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    resolve_data_sources(&mut config).unwrap();

    let val = config.params.get("__data__hostname").unwrap();
    assert_eq!(yaml_value_to_string(val), "test-host");
}

#[test]
fn test_fj223_data_source_file_with_default() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
data:
  missing:
    type: file
    value: /nonexistent/file
    default: "fallback"
"#;
    let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    resolve_data_sources(&mut config).unwrap();

    let val = config.params.get("__data__missing").unwrap();
    assert_eq!(yaml_value_to_string(val), "fallback");
}

#[test]
fn test_fj223_data_source_file_no_default_fails() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
data:
  missing:
    type: file
    value: /nonexistent/file
"#;
    let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let result = resolve_data_sources(&mut config);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("file error"));
}

#[test]
fn test_fj223_data_template_resolution() {
    let dir = tempfile::tempdir().unwrap();
    let data_file = dir.path().join("env.txt");
    std::fs::write(&data_file, "production").unwrap();

    let yaml = format!(
        r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/env.conf
    content: "env={{{{data.env}}}}"
data:
  env:
    type: file
    value: "{}"
"#,
        data_file.display()
    );
    let mut config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    resolve_data_sources(&mut config).unwrap();

    // Now resolve the template
    let resolved =
        resolve_template("env={{data.env}}", &config.params, &config.machines).unwrap();
    assert_eq!(resolved, "env=production");
}

#[test]
fn test_fj223_data_source_command_with_default() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
data:
  fail:
    type: command
    value: "exit 1"
    default: "fallback"
"#;
    let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    resolve_data_sources(&mut config).unwrap();

    let val = config.params.get("__data__fail").unwrap();
    assert_eq!(yaml_value_to_string(val), "fallback");
}

#[test]
fn test_fj223_data_source_dns() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
data:
  loopback:
    type: dns
    value: localhost
"#;
    let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    resolve_data_sources(&mut config).unwrap();

    let val = config.params.get("__data__loopback").unwrap();
    let ip = yaml_value_to_string(val);
    assert!(ip == "127.0.0.1" || ip == "::1", "got: {}", ip);
}

#[test]
fn test_fj223_no_data_sources() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
"#;
    let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    resolve_data_sources(&mut config).unwrap();
    // No __data__ keys added
    assert!(!config.params.keys().any(|k| k.starts_with("__data__")));
}

