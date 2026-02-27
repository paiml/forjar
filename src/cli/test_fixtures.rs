//! Shared test helper functions used across multiple CLI test files.

use crate::core::{state, types};
use std::collections::HashMap;
use std::path::{Path, PathBuf};


/// Create a state directory with a lock file containing the given resources.
pub(crate) fn make_state_dir_with_lock(
    dir: &Path,
    machine: &str,
    resources: Vec<(&str, &str, types::ResourceStatus)>,
) {
    let mut res_map = indexmap::IndexMap::new();
    for (id, hash, status) in resources {
        res_map.insert(
            id.to_string(),
            types::ResourceLock {
                resource_type: types::ResourceType::File,
                status,
                applied_at: Some("2026-02-25T00:00:00Z".to_string()),
                duration_seconds: Some(0.1),
                hash: hash.to_string(),
                details: HashMap::new(),
            },
        );
    }
    let lock = types::StateLock {
        schema: "1.0".to_string(),
        machine: machine.to_string(),
        hostname: "test-host".to_string(),
        generated_at: "2026-02-25T00:00:00Z".to_string(),
        generator: "forjar 0.1.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources: res_map,
    };
    state::save_lock(dir, &lock).unwrap();
}


/// Create a test StateLock with given machine name and resources.
pub(crate) fn make_test_lock(
    machine: &str,
    resources: indexmap::IndexMap<String, types::ResourceLock>,
) -> types::StateLock {
    types::StateLock {
        schema: "1.0".to_string(),
        machine: machine.to_string(),
        hostname: machine.to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar 0.1.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    }
}


/// Create a test ResourceLock with the given type.
pub(crate) fn make_test_resource_lock(rtype: types::ResourceType) -> types::ResourceLock {
    types::ResourceLock {
        resource_type: rtype,
        status: types::ResourceStatus::Converged,
        applied_at: Some("2026-01-15T10:30:00Z".to_string()),
        duration_seconds: Some(0.5),
        hash: "blake3:abcdef123456".to_string(),
        details: HashMap::new(),
    }
}


/// Write a simple forjar config with one machine and two resources.
pub(crate) fn write_simple_config(dir: &Path) -> PathBuf {
    let config_path = dir.join("forjar.yaml");
    std::fs::write(
        &config_path,
        r#"
version: "1.0"
name: graph-test
machines:
  web:
    hostname: web
    addr: 1.1.1.1
resources:
  setup:
    type: file
    machine: web
    path: /tmp/setup
    state: directory
  app:
    type: file
    machine: web
    path: /tmp/setup/app.conf
    content: "config"
    depends_on: [setup]
"#,
    )
    .unwrap();
    config_path
}


/// Write a config with params and env file support.
pub(crate) fn write_env_config(dir: &Path) -> PathBuf {
    let file = dir.join("forjar.yaml");
    std::fs::write(
        &file,
        r#"
version: "1.0"
name: env-test
params:
  data_dir: /default/data
  log_level: info
machines:
  m1:
    hostname: localhost
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m1
    path: "{{params.data_dir}}/config.yaml"
    content: "level: {{params.log_level}}"
"#,
    )
    .unwrap();
    file
}


/// Write a config with outputs.
pub(crate) fn write_output_config(dir: &Path) -> PathBuf {
    let file = dir.join("forjar.yaml");
    let yaml = r#"
version: "1.0"
name: test-outputs
params:
  port: "8080"
  domain: example.com
machines:
  web:
    hostname: web
    addr: 10.0.0.1
resources: {}
outputs:
  app_url:
    value: "http://{{params.domain}}:{{params.port}}"
    description: "App URL"
  host_ip:
    value: "{{machine.web.addr}}"
  raw_param:
    value: "{{params.port}}"
"#;
    std::fs::write(&file, yaml).unwrap();
    file
}
