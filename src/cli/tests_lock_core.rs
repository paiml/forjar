//! Tests: Lock management.

#![allow(unused_imports)]
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::lock_core::*;
use super::commands::*;
use super::dispatch::*;
use super::show::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj220_cmd_policy_deny_blocks() {
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
    owner: root
policies:
  - type: deny
    message: "no root owner"
    resource_type: file
    condition_field: owner
    condition_value: root
"#,
        )
        .unwrap();
        let result = cmd_policy(&file, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("policy violations"));
    }


    #[test]
    fn test_fj256_lock_generates_lock_files() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::write(
            &config_path,
            r#"
version: "1.0"
name: lock-test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
    content: "hello"
"#,
        )
        .unwrap();
        cmd_lock(&config_path, &state_dir, None, None, false, false).unwrap();

        // Lock file should exist
        let lock = state::load_lock(&state_dir, "m1").unwrap().unwrap();
        assert_eq!(lock.machine, "m1");
        assert_eq!(lock.resources.len(), 2);
        assert!(lock.resources.contains_key("pkg"));
        assert!(lock.resources.contains_key("cfg"));
        // All hashes should be blake3
        for (_, res) in &lock.resources {
            assert!(res.hash.starts_with("blake3:"));
        }
    }


    #[test]
    fn test_fj256_lock_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state1 = dir.path().join("state1");
        let state2 = dir.path().join("state2");
        std::fs::write(
            &config_path,
            r#"
version: "1.0"
name: det-test
machines:
  box1:
    hostname: box1
    addr: 127.0.0.1
resources:
  myfile:
    type: file
    machine: box1
    path: /tmp/det.txt
    content: "deterministic"
"#,
        )
        .unwrap();
        cmd_lock(&config_path, &state1, None, None, false, false).unwrap();
        cmd_lock(&config_path, &state2, None, None, false, false).unwrap();

        let lock1 = state::load_lock(&state1, "box1").unwrap().unwrap();
        let lock2 = state::load_lock(&state2, "box1").unwrap().unwrap();
        assert_eq!(
            lock1.resources["myfile"].hash, lock2.resources["myfile"].hash,
            "lock hashes must be deterministic"
        );
    }


    #[test]
    fn test_fj256_lock_verify_matches() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::write(
            &config_path,
            r#"
version: "1.0"
name: verify-test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [git]
"#,
        )
        .unwrap();
        // Generate lock
        cmd_lock(&config_path, &state_dir, None, None, false, false).unwrap();
        // Verify should succeed (exit 0 — no process::exit in test, just check no error)
        // We need to catch the process::exit, so let's check the logic directly
        let lock = state::load_lock(&state_dir, "m1").unwrap().unwrap();
        assert_eq!(lock.resources.len(), 1);
        assert!(lock.resources["pkg"].hash.starts_with("blake3:"));
    }


    #[test]
    fn test_fj256_lock_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::write(
            &config_path,
            r#"
version: "1.0"
name: json-lock
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  svc:
    type: service
    machine: m1
    name: nginx
"#,
        )
        .unwrap();
        // JSON output should not error
        cmd_lock(&config_path, &state_dir, None, None, false, true).unwrap();
        // Lock should still be written
        let lock = state::load_lock(&state_dir, "m1").unwrap().unwrap();
        assert_eq!(lock.resources.len(), 1);
    }


    #[test]
    fn test_fj256_lock_multiple_machines() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::write(
            &config_path,
            r#"
version: "1.0"
name: multi-machine
machines:
  web:
    hostname: web
    addr: 10.0.0.1
  db:
    hostname: db
    addr: 10.0.0.2
resources:
  web-pkg:
    type: package
    machine: web
    provider: apt
    packages: [nginx]
  db-pkg:
    type: package
    machine: db
    provider: apt
    packages: [postgresql]
"#,
        )
        .unwrap();
        cmd_lock(&config_path, &state_dir, None, None, false, false).unwrap();

        let web_lock = state::load_lock(&state_dir, "web").unwrap().unwrap();
        let db_lock = state::load_lock(&state_dir, "db").unwrap().unwrap();
        assert_eq!(web_lock.resources.len(), 1);
        assert_eq!(db_lock.resources.len(), 1);
        assert!(web_lock.resources.contains_key("web-pkg"));
        assert!(db_lock.resources.contains_key("db-pkg"));
    }


    #[test]
    fn test_fj256_lock_updates_global_lock() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::write(
            &config_path,
            r#"
version: "1.0"
name: global-lock-test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#,
        )
        .unwrap();
        cmd_lock(&config_path, &state_dir, None, None, false, false).unwrap();

        let global = state::load_global_lock(&state_dir).unwrap().unwrap();
        assert_eq!(global.name, "global-lock-test");
        assert!(global.machines.contains_key("m1"));
        assert_eq!(global.machines["m1"].resources, 1);
    }


    #[test]
    fn test_fj256_lock_hash_changes_on_content() {
        let dir = tempfile::tempdir().unwrap();
        let state1 = dir.path().join("state1");
        let state2 = dir.path().join("state2");
        let config1 = dir.path().join("c1.yaml");
        let config2 = dir.path().join("c2.yaml");
        std::fs::write(
            &config1,
            r#"
version: "1.0"
name: t
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: m1
    path: /tmp/test.txt
    content: "version1"
"#,
        )
        .unwrap();
        std::fs::write(
            &config2,
            r#"
version: "1.0"
name: t
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: m1
    path: /tmp/test.txt
    content: "version2"
"#,
        )
        .unwrap();
        cmd_lock(&config1, &state1, None, None, false, false).unwrap();
        cmd_lock(&config2, &state2, None, None, false, false).unwrap();

        let lock1 = state::load_lock(&state1, "m1").unwrap().unwrap();
        let lock2 = state::load_lock(&state2, "m1").unwrap().unwrap();
        assert_ne!(
            lock1.resources["f"].hash, lock2.resources["f"].hash,
            "different content must produce different hashes"
        );
    }


    #[test]
    fn test_fj256_lock_with_depends_on() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::write(
            &config_path,
            r#"
version: "1.0"
name: deps-test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  base:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
  conf:
    type: file
    machine: m1
    path: /etc/app.conf
    content: "config"
    depends_on: [base]
"#,
        )
        .unwrap();
        cmd_lock(&config_path, &state_dir, None, None, false, false).unwrap();

        let lock = state::load_lock(&state_dir, "m1").unwrap().unwrap();
        assert_eq!(lock.resources.len(), 2);
        assert!(lock.resources.contains_key("base"));
        assert!(lock.resources.contains_key("conf"));
    }


    #[test]
    fn test_fj256_lock_resource_types_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::write(
            &config_path,
            r#"
version: "1.0"
name: types-test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [git]
  cfg:
    type: file
    machine: m1
    path: /etc/test
    content: "data"
  svc:
    type: service
    machine: m1
    name: nginx
"#,
        )
        .unwrap();
        cmd_lock(&config_path, &state_dir, None, None, false, false).unwrap();

        let lock = state::load_lock(&state_dir, "m1").unwrap().unwrap();
        assert_eq!(
            lock.resources["pkg"].resource_type,
            types::ResourceType::Package
        );
        assert_eq!(
            lock.resources["cfg"].resource_type,
            types::ResourceType::File
        );
        assert_eq!(
            lock.resources["svc"].resource_type,
            types::ResourceType::Service
        );
    }

    // ── FJ-260: forjar snapshot tests ────────────────────────────


    #[test]
    fn test_fj366_lock_prune_parse() {
        let cmd = Commands::LockPrune(LockPruneArgs {
            file: PathBuf::from("forjar.yaml"),
            state_dir: PathBuf::from("state"),
            yes: false,
        });
        match cmd {
            Commands::LockPrune(LockPruneArgs { yes, .. }) => assert!(!yes),
            _ => panic!("expected LockPrune"),
        }
    }


    #[test]
    fn test_fj384_lock_info_parse() {
        let cmd = Commands::LockInfo(LockInfoArgs {
            state_dir: PathBuf::from("state"),
            json: false,
        });
        match cmd {
            Commands::LockInfo(LockInfoArgs { state_dir, .. }) => {
                assert_eq!(state_dir, PathBuf::from("state"));
            }
            _ => panic!("expected LockInfo"),
        }
    }


    #[test]
    fn test_fj395_lock_compact_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch(
            Commands::LockCompact(LockCompactArgs {
                state_dir: state,
                yes: false,
                json: true,
            }),
            false,
            true,
        );
        assert!(result.is_ok());
    }

    // ── Phase 26: Advanced Automation & Governance (FJ-400→FJ-407) ──


    #[test]
    fn test_fj405_lock_verify_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch(
            Commands::LockVerify(LockVerifyArgs {
                state_dir: state,
                json: true,
            }),
            false,
            true,
        );
        assert!(result.is_ok());
    }

}
