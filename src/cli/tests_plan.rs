//! Tests: Plan command.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::plan::*;
use super::commands::*;
use super::dispatch::*;
use super::test_fixtures::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj017_plan() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
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
    provider: apt
    packages: [curl]
"#,
        )
        .unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        cmd_plan(
            &config,
            &state,
            None,
            None,
            None,
            false,
            false,
            None,
            None,
            None,
            false,
            None,
            false, // no cost,
            &[],   // what_if
        )
        .unwrap();
    }


    #[test]
    fn test_fj017_plan_with_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: test
machines:
  a:
    hostname: a
    addr: 1.1.1.1
  b:
    hostname: b
    addr: 2.2.2.2
resources:
  pkg-a:
    type: package
    machine: a
    provider: apt
    packages: [curl]
  pkg-b:
    type: package
    machine: b
    provider: apt
    packages: [wget]
"#,
        )
        .unwrap();
        cmd_plan(
            &config,
            &state,
            Some("a"),
            None,
            None,
            false,
            false,
            None,
            None,
            None,
            false,
            None,
            false, // no cost,
            &[],   // what_if
        )
        .unwrap();
    }


    #[test]
    fn test_fj017_plan_validation_error() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::write(
            &config,
            r#"
version: "2.0"
name: ""
machines: {}
resources: {}
"#,
        )
        .unwrap();
        let result = cmd_plan(
            &config,
            &state,
            None,
            None,
            None,
            false,
            false,
            None,
            None,
            None,
            false,
            None,
            false, // no cost,
            &[],   // what_if
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("validation"));
    }


    #[test]
    fn test_fj017_dispatch_plan() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
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
    provider: apt
    packages: [curl]
"#,
        )
        .unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        dispatch(
            Commands::Plan {
                file: config,
                machine: None,
                resource: None,
                tag: None,
                group: None,
                state_dir: state,
                json: false,
                output_dir: None,
                env_file: None,
                workspace: None,
                no_diff: false,
                target: None,
                cost: false,
                what_if: vec![],
            },
            false,
            true,
        )
        .unwrap();
    }


    #[test]
    fn test_fj017_plan_nonexistent_state_dir() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
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
    provider: apt
    packages: [curl]
"#,
        )
        .unwrap();
        // Plan with nonexistent state dir → everything shows as Create
        let missing = dir.path().join("no-state");
        cmd_plan(
            &config,
            &missing,
            None,
            None,
            None,
            false,
            false,
            None,
            None,
            None,
            false,
            None,
            false, // no cost,
            &[],   // what_if
        )
        .unwrap();
    }


    #[test]
    fn test_fj017_plan_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: json-test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#,
        )
        .unwrap();
        // json=true should not panic (output goes to stdout)
        cmd_plan(
            &config,
            &state,
            None,
            None,
            None,
            true,
            false,
            None,
            None,
            None,
            false,
            None,
            false, // no cost,
            &[],   // what_if
        )
        .unwrap();
    }


    #[test]
    fn test_fj017_plan_verbose() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: verbose-test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#,
        )
        .unwrap();
        cmd_plan(
            &config,
            &state,
            None,
            None,
            None,
            false,
            true,
            None,
            None,
            None,
            false,
            None,
            false, // no cost,
            &[],   // what_if
        )
        .unwrap();
    }


    #[test]
    fn test_fj017_plan_output_dir() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        let output = dir.path().join("scripts");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: test
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
    path: /etc/test.conf
    content: "hello"
"#,
        )
        .unwrap();
        cmd_plan(
            &config,
            &state,
            None,
            None,
            None,
            false,
            false,
            Some(&output),
            None, // no env_file
            None, // no workspace
            false,
            None,
            false, // no cost,
            &[],   // what_if
        )
        .unwrap();

        // Should have created scripts for both resources
        assert!(output.exists());
        assert!(output.join("pkg.check.sh").exists());
        assert!(output.join("pkg.apply.sh").exists());
        assert!(output.join("pkg.state_query.sh").exists());
        assert!(output.join("conf.check.sh").exists());
        assert!(output.join("conf.apply.sh").exists());

        // Verify script content is non-empty
        let check = std::fs::read_to_string(output.join("pkg.check.sh")).unwrap();
        assert!(check.contains("dpkg"));
    }


    #[test]
    fn test_fj211_plan_with_env_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_env_config(dir.path());
        let env = dir.path().join("test.env.yaml");
        std::fs::write(&env, "data_dir: /test/data\n").unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        cmd_plan(
            &file,
            &state,
            None,
            None,
            None,
            false,
            false,
            None,
            Some(env.as_path()),
            None, // no workspace
            false,
            None,
            false, // no cost,
            &[],   // what_if
        )
        .unwrap();
    }


    #[test]
    fn test_fj210_plan_with_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: ws-test
params:
  env: "{{params.workspace}}"
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: /tmp/test.txt
    content: "env={{params.env}}"
"#,
        )
        .unwrap();
        let state = dir.path().join("state/staging");
        std::fs::create_dir_all(&state).unwrap();

        cmd_plan(
            &file,
            &state,
            None,
            None,
            None,
            false,
            false,
            None,
            None,
            Some("staging"),
            false,
            None,
            false, // no cost,
            &[],   // what_if
        )
        .unwrap();
    }

    // ================================================================
    // FJ-220: Policy check CLI tests
    // ================================================================

}
