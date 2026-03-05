//! Shared test helpers for executor tests.

use super::*;

pub fn local_machine() -> Machine {
    Machine {
        hostname: "localhost".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    }
}

pub fn local_config() -> ForjarConfig {
    let yaml = r#"
version: "1.0"
name: test
params: {}
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: /tmp/forjar-test-executor.txt
    content: "hello from forjar"
policy:
  failure: stop_on_first
  tripwire: true
  lock_file: true
"#;
    serde_yaml_ng::from_str(yaml).unwrap()
}

pub fn drift_config(file_path: &str) -> ForjarConfig {
    let yaml = format!(
        r#"
version: "1.0"
name: drift-test
params: {{}}
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: {file_path}
    content: "hello from forjar"
policy:
  failure: stop_on_first
  tripwire: true
  lock_file: true
"#
    );
    serde_yaml_ng::from_str(&yaml).unwrap()
}
