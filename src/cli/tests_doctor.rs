//! Tests: Doctor diagnostics.

#![allow(unused_imports)]
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::doctor::*;
use super::commands::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj251_doctor_no_config() {
        // Doctor without config should check system basics and succeed
        let result = cmd_doctor(None, false, false);
        assert!(
            result.is_ok(),
            "doctor without config should pass on dev machine"
        );
    }


    #[test]
    fn test_fj251_doctor_json_output() {
        // JSON mode should not crash
        let result = cmd_doctor(None, true, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj251_doctor_with_local_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: /tmp/test.txt
    content: "test"
"#,
        )
        .unwrap();
        let result = cmd_doctor(Some(&file), false, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj251_doctor_with_ssh_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: test
machines:
  remote:
    hostname: remote
    addr: 10.0.0.1
    user: deploy
resources:
  cfg:
    type: file
    machine: remote
    path: /tmp/test.txt
    content: "test"
"#,
        )
        .unwrap();
        // Should check for ssh (which exists on dev machine)
        let result = cmd_doctor(Some(&file), false, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj251_doctor_with_container_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: test
machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      image: ubuntu:22.04
resources:
  cfg:
    type: file
    machine: test-box
    path: /tmp/test.txt
    content: "test"
"#,
        )
        .unwrap();
        // May fail if docker not installed, but should not crash
        let _result = cmd_doctor(Some(&file), false, false);
    }


    #[test]
    fn test_fj251_doctor_bad_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(&file, "invalid yaml: [[[").unwrap();
        let result = cmd_doctor(Some(&file), false, false);
        // Should report failure for bad config
        assert!(result.is_err());
    }


    #[test]
    fn test_fj251_doctor_json_with_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: /tmp/test.txt
    content: "test"
"#,
        )
        .unwrap();
        let result = cmd_doctor(Some(&file), true, false);
        assert!(result.is_ok());
    }

    // FJ-253: Completion tests


    #[test]
    fn test_fj287_doctor_no_fix_runs() {
        // doctor without fix should not crash
        let _ = cmd_doctor(None, false, false);
    }

    // ── FJ-290: forjar apply --parallel ──────────────────────────


    #[test]
    fn test_fj343_doctor_network_flag() {
        let cmd = Commands::Doctor(DoctorArgs {
            file: None,
            json: false,
            fix: false,
            network: true,
        });
        match cmd {
            Commands::Doctor(DoctorArgs { network, .. }) => assert!(network),
            _ => panic!("expected Doctor"),
        }
    }

}
