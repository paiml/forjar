//! Tests: Show, explain, compare, template.

#![allow(unused_imports)]
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::show::*;
use super::diff_cmd::*;
use super::observe::*;
use super::test_fixtures::*;
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;



    #[test]
    fn test_fj132_cmd_show_all_resources() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [curl]
"#;
        std::fs::write(&file, yaml).unwrap();
        cmd_show(&file, None, false).unwrap();
    }


    #[test]
    fn test_fj132_cmd_show_specific_resource() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  my-file:
    type: file
    machine: m
    path: /etc/test.conf
    content: "hello"
"#;
        std::fs::write(&file, yaml).unwrap();
        cmd_show(&file, Some("my-file"), false).unwrap();
    }


    #[test]
    fn test_fj132_cmd_show_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [git]
"#;
        std::fs::write(&file, yaml).unwrap();
        cmd_show(&file, None, true).unwrap();
    }


    #[test]
    fn test_fj132_cmd_show_missing_resource() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: test
machines: {}
resources: {}
"#;
        std::fs::write(&file, yaml).unwrap();
        let result = cmd_show(&file, Some("nonexistent"), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }


    #[test]
    fn test_fj215_output_all() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_output_config(dir.path());
        cmd_output(&file, None, false).unwrap();
    }


    #[test]
    fn test_fj215_output_all_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_output_config(dir.path());
        cmd_output(&file, None, true).unwrap();
    }


    #[test]
    fn test_fj215_output_single_key() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_output_config(dir.path());
        cmd_output(&file, Some("raw_param"), false).unwrap();
    }


    #[test]
    fn test_fj215_output_single_key_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_output_config(dir.path());
        cmd_output(&file, Some("app_url"), true).unwrap();
    }


    #[test]
    fn test_fj215_output_unknown_key() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_output_config(dir.path());
        let result = cmd_output(&file, Some("nonexistent"), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not defined"));
    }


    #[test]
    fn test_fj215_output_no_outputs() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
"#;
        std::fs::write(&file, yaml).unwrap();
        cmd_output(&file, None, false).unwrap();
    }


    #[test]
    fn test_fj215_output_machine_ref() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_output_config(dir.path());
        cmd_output(&file, Some("host_ip"), false).unwrap();
    }

    // ================================================================
    // FJ-211: env file loading tests
    // ================================================================

    #[allow(dead_code)]
    fn write_env_config(dir: &Path) -> PathBuf {
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


    #[test]
    fn test_fj220_cmd_policy_no_violations() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
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
    path: /etc/app.conf
    owner: noah
policies:
  - type: require
    message: "files must have owner"
    resource_type: file
    field: owner
"#,
        )
        .unwrap();
        cmd_policy(&file, false).unwrap();
    }


    #[test]
    fn test_fj220_cmd_policy_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
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
    path: /etc/app.conf
policies:
  - type: warn
    message: "files should have owner"
    resource_type: file
    condition_field: owner
    condition_value: root
"#,
        )
        .unwrap();
        // JSON mode with no deny violations should succeed
        cmd_policy(&file, true).unwrap();
    }

}
