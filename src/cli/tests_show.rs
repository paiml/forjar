//! Tests: Show, explain, compare, template.

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
    fn test_fj017_show_full_config() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: show-test
params:
  env: staging
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  conf:
    type: file
    machine: m1
    path: /etc/{{params.env}}.conf
    content: "env={{params.env}}"
"#,
        )
        .unwrap();
        // Should resolve templates without error
        cmd_show(&config, None, false).unwrap();
    }


    #[test]
    fn test_fj017_show_specific_resource() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: show-test
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
  conf:
    type: file
    machine: m1
    path: /etc/test
    content: hello
"#,
        )
        .unwrap();
        // Show specific resource
        cmd_show(&config, Some("conf"), false).unwrap();
    }


    #[test]
    fn test_fj017_show_missing_resource() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: show-test
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
"#,
        )
        .unwrap();
        let result = cmd_show(&config, Some("nonexistent"), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }


    #[test]
    fn test_fj054_policy_hooks_parsed() {
        let yaml = r#"
version: "1.0"
name: test
machines: {}
resources: {}
policy:
  failure: stop_on_first
  pre_apply: "echo before"
  post_apply: "echo after"
"#;
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.policy.pre_apply.as_deref(), Some("echo before"));
        assert_eq!(config.policy.post_apply.as_deref(), Some("echo after"));
    }

    // ── forjar diff tests ──────────────────────────────────────────

    fn make_state_dir_with_lock(
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


    #[test]
    fn test_fj017_show_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: json-show-test
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
"#,
        )
        .unwrap();
        // JSON output should succeed
        cmd_show(&config, None, true).unwrap();
    }


    #[test]
    fn test_fj017_show_specific_resource_json() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: show-test
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
"#,
        )
        .unwrap();
        cmd_show(&config, Some("pkg"), true).unwrap();
    }

    // ── Fmt edge cases ─────────────────────────────────────────


    #[test]
    fn test_fj131_cmd_diff_json_output() {
        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap();

        // Both have web machine
        let from_lock = types::StateLock {
            schema: "1.0".to_string(),
            machine: "web".to_string(),
            hostname: "web-box".to_string(),
            generated_at: "2026-02-25T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources: indexmap::IndexMap::new(),
        };
        state::save_lock(from_dir.path(), &from_lock).unwrap();
        state::save_lock(to_dir.path(), &from_lock).unwrap();

        // JSON output should not error
        cmd_diff(from_dir.path(), to_dir.path(), None, None, true).unwrap();
    }


    #[test]
    fn test_fj131_cmd_anomaly_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let machine_dir = dir.path().join("web");
        std::fs::create_dir_all(&machine_dir).unwrap();

        let events = [
            r#"{"ts":"2026-02-25T00:00:00Z","event":"resource_converged","machine":"web","resource":"pkg","duration_seconds":1.0,"hash":"blake3:abc"}"#,
            r#"{"ts":"2026-02-25T01:00:00Z","event":"resource_failed","machine":"web","resource":"pkg","error":"timeout"}"#,
        ];
        std::fs::write(machine_dir.join("events.jsonl"), events.join("\n")).unwrap();

        cmd_anomaly(dir.path(), None, 1, true).unwrap();
    }


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
