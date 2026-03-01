//! Tests: Core validation command.

#![allow(unused_imports)]
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::validate_core::*;
use super::validate_compliance::*;
use super::validate_paths::*;
use super::validate_resources::*;
use super::validate_structural::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj590_validate_check_dependencies_json() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  pkg1:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [curl]\n").unwrap();
        let result = cmd_validate_check_dependencies(&cfg, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj601_validate_check_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  cfg1:\n    type: file\n    machine: m1\n    path: /etc/app/config.yaml\n    content: hello\n    mode: '0644'\n    owner: noah\n").unwrap();
        let result = cmd_validate_check_permissions(&cfg, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj601_validate_check_permissions_json() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  pkg1:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [curl]\n").unwrap();
        let result = cmd_validate_check_permissions(&cfg, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj611_validate_check_idempotency_deep() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  pkg1:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [curl]\n").unwrap();
        let result = cmd_validate_check_idempotency_deep(&cfg, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj611_validate_check_idempotency_deep_json() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  cfg1:\n    type: file\n    machine: m1\n    path: /tmp/test\n    content: hello\n    mode: '0644'\n").unwrap();
        let result = cmd_validate_check_idempotency_deep(&cfg, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj621_validate_check_machine_reachability() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  pkg1:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [curl]\n").unwrap();
        let result = cmd_validate_check_machine_reachability(&cfg, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj621_validate_check_machine_reachability_json() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  pkg1:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [curl]\n").unwrap();
        let result = cmd_validate_check_machine_reachability(&cfg, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj631_validate_check_circular_refs() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  pkg1:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [curl]\n").unwrap();
        let result = cmd_validate_check_circular_refs(&cfg, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj631_validate_check_circular_refs_json() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  pkg1:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [curl]\n").unwrap();
        let result = cmd_validate_check_circular_refs(&cfg, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj641_validate_check_naming_conventions() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines: {}\nresources:\n  good-name:\n    type: file\n    machine: m\n    path: /tmp/x\n").unwrap();
        let result = cmd_validate_check_naming_conventions(&cfg, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj641_validate_check_naming_conventions_json() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines: {}\nresources:\n  good-name:\n    type: file\n    machine: m\n    path: /tmp/x\n").unwrap();
        let result = cmd_validate_check_naming_conventions(&cfg, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj651_validate_check_resource_limits() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n").unwrap();
        let result = cmd_validate_check_resource_limits(&cfg, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj661_validate_check_owner_consistency() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines: {}\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    owner: noah\n").unwrap();
        let result = cmd_validate_check_owner_consistency(&cfg, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj661_validate_check_owner_consistency_json() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines: {}\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n").unwrap();
        let result = cmd_validate_check_owner_consistency(&cfg, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj671_validate_check_path_conflicts() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines: {}\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/shared\n  b:\n    type: file\n    machine: m\n    path: /tmp/shared\n").unwrap();
        let result = cmd_validate_check_path_conflicts(&cfg, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj671_validate_check_path_conflicts_json() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: '1.0'\nname: test\nmachines: {}\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n").unwrap();
        let result = cmd_validate_check_path_conflicts(&cfg, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj681_validate_check_service_deps() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("forjar.yaml");
        std::fs::write(&f, "version: '1.0'\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  svc:\n    type: service\n    machine: m\n    name: nginx\n").unwrap();
        let result = cmd_validate_check_service_deps(&f, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj691_validate_check_template_vars() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("forjar.yaml");
        std::fs::write(&f, "version: '1.0'\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  f:\n    type: file\n    machine: m\n    path: /tmp/test\n").unwrap();
        let result = cmd_validate_check_template_vars(&f, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj691_validate_check_template_vars_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("forjar.yaml");
        std::fs::write(&f, "version: '1.0'\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  f:\n    type: file\n    machine: m\n    path: /tmp/test\n").unwrap();
        let result = cmd_validate_check_template_vars(&f, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj701_validate_check_mode_consistency() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("forjar.yaml");
        std::fs::write(&f, "version: '1.0'\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  f1:\n    type: file\n    machine: m\n    path: /tmp/a\n    mode: '0644'\n  f2:\n    type: file\n    machine: m\n    path: /tmp/b\n    mode: '0755'\n").unwrap();
        let result = cmd_validate_check_mode_consistency(&f, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj701_validate_check_mode_consistency_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("forjar.yaml");
        std::fs::write(&f, "version: '1.0'\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  f1:\n    type: file\n    machine: m\n    path: /tmp/a\n    mode: '0644'\n").unwrap();
        let result = cmd_validate_check_mode_consistency(&f, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj711_validate_check_group_consistency() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("forjar.yaml");
        std::fs::write(&f, "version: '1.0'\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  f1:\n    type: file\n    machine: m\n    path: /tmp/a\n    owner: noah\n  f2:\n    type: file\n    machine: m\n    path: /tmp/b\n    owner: root\n").unwrap();
        let result = cmd_validate_check_group_consistency(&f, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj711_validate_check_group_consistency_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("forjar.yaml");
        std::fs::write(&f, "version: '1.0'\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  f1:\n    type: file\n    machine: m\n    path: /tmp/a\n    owner: noah\n").unwrap();
        let result = cmd_validate_check_group_consistency(&f, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj721_validate_check_mount_points() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("forjar.yaml");
        std::fs::write(&f, "version: '1.0'\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  mnt:\n    type: mount\n    machine: m\n    source: /dev/sda1\n    path: /mnt/data\n    fstype: ext4\n").unwrap();
        let result = cmd_validate_check_mount_points(&f, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj721_validate_check_mount_points_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("forjar.yaml");
        std::fs::write(&f, "version: '1.0'\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n").unwrap();
        let result = cmd_validate_check_mount_points(&f, true);
        assert!(result.is_ok());
    }

}
