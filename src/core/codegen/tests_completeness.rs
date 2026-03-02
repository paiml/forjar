//! Completeness + nonempty tests for codegen — FJ-005, FJ-132.

use super::test_fixtures::*;
use super::*;
use crate::core::types::ResourceType;

#[test]
fn test_fj005_all_phase1_check_scripts_nonempty() {
    let types_and_resources = [
        make_package(),
        make_file(),
        make_service(),
        make_mount(),
        {
            let mut r = make_package();
            r.resource_type = ResourceType::User;
            r.name = Some("u".to_string());
            r
        },
        {
            let mut r = make_package();
            r.resource_type = ResourceType::Docker;
            r.name = Some("c".to_string());
            r.image = Some("img".to_string());
            r
        },
        {
            let mut r = make_package();
            r.resource_type = ResourceType::Cron;
            r.name = Some("j".to_string());
            r.schedule = Some("0 * * * *".to_string());
            r.command = Some("echo".to_string());
            r
        },
        {
            let mut r = make_package();
            r.resource_type = ResourceType::Network;
            r.port = Some("80".to_string());
            r
        },
    ];
    for r in &types_and_resources {
        let script = check_script(r).unwrap();
        assert!(
            !script.is_empty(),
            "check_script for {:?} should not be empty",
            r.resource_type
        );
    }
}

#[test]
fn test_fj005_all_phase1_state_query_scripts_nonempty() {
    let types_and_resources = [
        make_package(),
        make_file(),
        make_service(),
        make_mount(),
        {
            let mut r = make_package();
            r.resource_type = ResourceType::User;
            r.name = Some("u".to_string());
            r
        },
        {
            let mut r = make_package();
            r.resource_type = ResourceType::Docker;
            r.name = Some("c".to_string());
            r
        },
        {
            let mut r = make_package();
            r.resource_type = ResourceType::Cron;
            r.name = Some("j".to_string());
            r
        },
        {
            let mut r = make_package();
            r.resource_type = ResourceType::Network;
            r.port = Some("80".to_string());
            r
        },
    ];
    for r in &types_and_resources {
        let script = state_query_script(r).unwrap();
        assert!(
            !script.is_empty(),
            "state_query_script for {:?} should not be empty",
            r.resource_type
        );
    }
}

// --- FJ-132: Codegen edge case tests ---

#[test]
fn test_fj132_state_query_docker_contains_inspect() {
    let mut r = make_package();
    r.resource_type = ResourceType::Docker;
    r.name = Some("web".to_string());
    r.image = Some("nginx:latest".to_string());
    let script = state_query_script(&r).unwrap();
    assert!(
        script.contains("docker inspect"),
        "docker state_query should use docker inspect"
    );
}

#[test]
fn test_fj132_state_query_cron_contains_crontab() {
    let mut r = make_package();
    r.resource_type = ResourceType::Cron;
    r.name = Some("backup".to_string());
    r.schedule = Some("0 2 * * *".to_string());
    r.command = Some("/opt/backup.sh".to_string());
    let script = state_query_script(&r).unwrap();
    assert!(
        script.contains("crontab"),
        "cron state_query should read crontab"
    );
}

#[test]
fn test_fj132_state_query_network_contains_ufw() {
    let mut r = make_package();
    r.resource_type = ResourceType::Network;
    r.port = Some("443".to_string());
    r.action = Some("allow".to_string());
    let script = state_query_script(&r).unwrap();
    assert!(script.contains("ufw"), "network state_query should use ufw");
}

#[test]
fn test_fj132_state_query_user_contains_id() {
    let mut r = make_package();
    r.resource_type = ResourceType::User;
    r.name = Some("deploy".to_string());
    let script = state_query_script(&r).unwrap();
    assert!(
        script.contains("id ") || script.contains("getent"),
        "user state_query should use id or getent"
    );
}

#[test]
fn test_fj132_all_phase1_apply_pipefail() {
    let types_and_resources = [
        make_package(),
        make_file(),
        make_service(),
        make_mount(),
        {
            let mut r = make_package();
            r.resource_type = ResourceType::User;
            r.name = Some("u".to_string());
            r
        },
        {
            let mut r = make_package();
            r.resource_type = ResourceType::Docker;
            r.name = Some("c".to_string());
            r.image = Some("img".to_string());
            r
        },
        {
            let mut r = make_package();
            r.resource_type = ResourceType::Cron;
            r.name = Some("j".to_string());
            r.schedule = Some("0 * * * *".to_string());
            r.command = Some("echo".to_string());
            r
        },
        {
            let mut r = make_package();
            r.resource_type = ResourceType::Network;
            r.port = Some("80".to_string());
            r
        },
    ];
    for r in &types_and_resources {
        let script = apply_script(r).unwrap();
        assert!(
            script.contains("set -euo pipefail"),
            "apply script for {:?} must contain pipefail",
            r.resource_type
        );
    }
}

#[test]
fn test_fj132_check_script_all_types_succeed() {
    let resources = [
        make_package(),
        make_file(),
        make_service(),
        make_mount(),
        {
            let mut r = make_package();
            r.resource_type = ResourceType::User;
            r.name = Some("testuser".to_string());
            r
        },
        {
            let mut r = make_package();
            r.resource_type = ResourceType::Docker;
            r.name = Some("app".to_string());
            r.image = Some("nginx".to_string());
            r
        },
        {
            let mut r = make_package();
            r.resource_type = ResourceType::Cron;
            r.name = Some("backup".to_string());
            r.schedule = Some("0 2 * * *".to_string());
            r.command = Some("tar czf /backup.tar.gz /data".to_string());
            r
        },
        {
            let mut r = make_package();
            r.resource_type = ResourceType::Network;
            r.port = Some("443".to_string());
            r
        },
    ];
    for r in &resources {
        let result = check_script(r);
        assert!(
            result.is_ok(),
            "check_script for {:?} should succeed: {:?}",
            r.resource_type,
            result.err()
        );
    }
}

#[test]
fn test_fj132_state_query_all_types_succeed() {
    let resources = [
        make_package(),
        make_file(),
        make_service(),
        make_mount(),
        {
            let mut r = make_package();
            r.resource_type = ResourceType::User;
            r.name = Some("u".to_string());
            r
        },
        {
            let mut r = make_package();
            r.resource_type = ResourceType::Docker;
            r.name = Some("c".to_string());
            r.image = Some("img".to_string());
            r
        },
        {
            let mut r = make_package();
            r.resource_type = ResourceType::Cron;
            r.name = Some("j".to_string());
            r.schedule = Some("* * * * *".to_string());
            r.command = Some("echo".to_string());
            r
        },
        {
            let mut r = make_package();
            r.resource_type = ResourceType::Network;
            r.port = Some("80".to_string());
            r
        },
    ];
    for r in &resources {
        let result = state_query_script(r);
        assert!(
            result.is_ok(),
            "state_query for {:?} should succeed: {:?}",
            r.resource_type,
            result.err()
        );
    }
}
