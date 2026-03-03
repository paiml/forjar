//! Tests for codegen dispatch — FJ-005, FJ-040, falsification tests.

use super::test_fixtures::*;
use super::*;
use crate::core::types::ResourceType;

#[test]
fn test_fj005_check_dispatches_package() {
    let r = make_package();
    let script = check_script(&r).unwrap();
    assert!(script.contains("dpkg"));
}

#[test]
fn test_fj005_check_dispatches_file() {
    let r = make_file();
    let script = check_script(&r).unwrap();
    assert!(script.contains("test -f"));
}

#[test]
fn test_fj005_check_dispatches_service() {
    let r = make_service();
    let script = check_script(&r).unwrap();
    assert!(script.contains("systemctl"));
}

#[test]
fn test_fj005_check_dispatches_mount() {
    let r = make_mount();
    let script = check_script(&r).unwrap();
    assert!(script.contains("mountpoint"));
}

#[test]
fn test_fj005_apply_dispatches_package() {
    let r = make_package();
    let script = apply_script(&r).unwrap();
    assert!(script.contains("apt-get install"));
}

#[test]
fn test_fj005_apply_dispatches_file() {
    let r = make_file();
    let script = apply_script(&r).unwrap();
    assert!(script.contains("cat >"));
    assert!(script.contains("FORJAR_EOF"));
}

#[test]
fn test_fj005_apply_dispatches_service() {
    let r = make_service();
    let script = apply_script(&r).unwrap();
    assert!(script.contains("systemctl start"));
}

#[test]
fn test_fj005_apply_dispatches_mount() {
    let r = make_mount();
    let script = apply_script(&r).unwrap();
    assert!(script.contains("mount -t 'nfs'"));
}

#[test]
fn test_fj005_state_query_dispatches() {
    let r = make_package();
    let script = state_query_script(&r).unwrap();
    assert!(script.contains("dpkg-query"));
}

#[test]
fn test_fj005_state_query_file() {
    let mut r = make_package();
    r.resource_type = ResourceType::File;
    r.path = Some("/etc/conf".to_string());
    let script = state_query_script(&r).unwrap();
    assert!(script.contains("stat") || script.contains("/etc/conf"));
}

#[test]
fn test_fj005_state_query_service() {
    let mut r = make_package();
    r.resource_type = ResourceType::Service;
    r.name = Some("nginx".to_string());
    let script = state_query_script(&r).unwrap();
    assert!(script.contains("nginx"));
}

#[test]
fn test_fj005_pepita_supported() {
    let mut r = make_package();
    r.resource_type = ResourceType::Pepita;
    r.name = Some("sandbox".to_string());
    r.netns = true;
    assert!(check_script(&r).is_ok());
    assert!(apply_script(&r).is_ok());
    assert!(state_query_script(&r).is_ok());
}

/// FALSIFY-CD-001: All Phase 1 types produce Ok for all three functions.
#[test]
fn falsify_cd_001_dispatch_completeness() {
    let resources = [make_package(), make_file(), make_service(), make_mount()];
    for r in &resources {
        assert!(
            check_script(r).is_ok(),
            "check_script failed for {:?}",
            r.resource_type
        );
        assert!(
            apply_script(r).is_ok(),
            "apply_script failed for {:?}",
            r.resource_type
        );
        assert!(
            state_query_script(r).is_ok(),
            "state_query_script failed for {:?}",
            r.resource_type
        );
    }
}

#[test]
fn test_fj005_dispatch_user() {
    let mut r = make_package();
    r.resource_type = ResourceType::User;
    r.name = Some("deploy".to_string());
    assert!(check_script(&r).unwrap().contains("deploy"));
    assert!(apply_script(&r).unwrap().contains("useradd"));
    assert!(state_query_script(&r).unwrap().contains("deploy"));
}

#[test]
fn test_fj005_dispatch_docker() {
    let mut r = make_package();
    r.resource_type = ResourceType::Docker;
    r.image = Some("nginx:latest".to_string());
    r.name = Some("web".to_string());
    assert!(check_script(&r).unwrap().contains("docker"));
    assert!(apply_script(&r).unwrap().contains("docker"));
    assert!(state_query_script(&r).unwrap().contains("docker"));
}

#[test]
fn test_fj005_dispatch_cron() {
    let mut r = make_package();
    r.resource_type = ResourceType::Cron;
    r.schedule = Some("0 2 * * *".to_string());
    r.command = Some("/opt/backup.sh".to_string());
    assert!(check_script(&r).unwrap().contains("crontab"));
    assert!(apply_script(&r).unwrap().contains("backup.sh"));
    assert!(state_query_script(&r).unwrap().contains("crontab"));
}

#[test]
fn test_fj005_dispatch_network() {
    let mut r = make_package();
    r.resource_type = ResourceType::Network;
    r.port = Some("443".to_string());
    r.action = Some("allow".to_string());
    assert!(check_script(&r).unwrap().contains("ufw"));
    assert!(apply_script(&r).unwrap().contains("ufw"));
    assert!(state_query_script(&r).unwrap().contains("ufw"));
}

#[test]
fn test_fj005_apply_scripts_contain_pipefail() {
    let resources = [make_package(), make_file(), make_service(), make_mount()];
    for r in &resources {
        let script = apply_script(r).unwrap();
        assert!(
            script.contains("set -euo pipefail"),
            "apply script for {:?} must contain pipefail",
            r.resource_type
        );
    }
}

#[test]
fn test_fj040_pepita_codegen_check() {
    let mut r = make_package();
    r.resource_type = ResourceType::Pepita;
    r.name = Some("jail".to_string());
    r.chroot_dir = Some("/var/jail".to_string());
    let script = check_script(&r).unwrap();
    assert!(script.contains("chroot:present:jail"));
}

#[test]
fn test_fj005_recipe_type_not_dispatchable() {
    let mut r = make_package();
    r.resource_type = ResourceType::Recipe;
    assert!(
        check_script(&r).is_err(),
        "recipe type should not be directly dispatchable"
    );
    assert!(apply_script(&r).is_err());
    assert!(state_query_script(&r).is_err());
}

/// FALSIFY-CD-002: Dispatch is symmetric — same types handled by all three functions.
#[test]
fn falsify_cd_002_dispatch_symmetry() {
    let all_types = [
        ResourceType::Package,
        ResourceType::File,
        ResourceType::Service,
        ResourceType::Mount,
        ResourceType::Docker,
        ResourceType::User,
        ResourceType::Network,
        ResourceType::Cron,
        ResourceType::Pepita,
    ];

    for rt in &all_types {
        let mut r = make_package();
        r.resource_type = rt.clone();

        let check_ok = check_script(&r).is_ok();
        let apply_ok = apply_script(&r).is_ok();
        let query_ok = state_query_script(&r).is_ok();

        assert_eq!(
            check_ok, apply_ok,
            "check/apply asymmetry for {:?}: check={}, apply={}",
            rt, check_ok, apply_ok
        );
        assert_eq!(
            apply_ok, query_ok,
            "apply/query asymmetry for {:?}: apply={}, query={}",
            rt, apply_ok, query_ok
        );
    }
}

#[test]
fn test_fj040_pepita_codegen_apply_netns() {
    let mut r = make_package();
    r.resource_type = ResourceType::Pepita;
    r.name = Some("net-sandbox".to_string());
    r.netns = true;
    let script = apply_script(&r).unwrap();
    assert!(script.contains("ip netns add 'forjar-net-sandbox'"));
}
