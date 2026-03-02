//! Coverage boost + FJ-036 codegen tests.

use super::test_fixtures::*;
use super::*;
use crate::core::types::ResourceType;

#[test]
fn test_fj036_check_script_user_contains_name() {
    let mut r = make_package();
    r.resource_type = ResourceType::User;
    r.name = Some("operator".to_string());
    let script = check_script(&r).unwrap();
    assert!(
        script.contains("operator"),
        "user check script must reference the username 'operator': {script}"
    );
}

#[test]
fn test_fj036_apply_docker_volumes_escaped() {
    let mut r = make_package();
    r.resource_type = ResourceType::Docker;
    r.name = Some("db".to_string());
    r.image = Some("postgres:15".to_string());
    r.volumes = vec![
        "/host/path with spaces:/container/data".to_string(),
        "/var/log:/logs".to_string(),
    ];
    let script = apply_script(&r).unwrap();
    // Volumes must be single-quoted to prevent shell word splitting
    assert!(
        script.contains("-v '/host/path with spaces:/container/data'"),
        "volume with spaces must be properly quoted: {script}"
    );
    assert!(
        script.contains("-v '/var/log:/logs'"),
        "second volume must be properly quoted: {script}"
    );
}

#[test]
fn test_fj036_state_query_mount_contains_findmnt() {
    let mut r = make_mount();
    r.path = Some("/mnt/nfs-share".to_string());
    let script = state_query_script(&r).unwrap();
    assert!(
        script.contains("findmnt"),
        "mount state_query must use findmnt to query mount details: {script}"
    );
}

#[test]
fn test_codegen_package_absent() {
    let mut r = make_package();
    r.state = Some("absent".to_string());
    let script = apply_script(&r).unwrap();
    assert!(
        script.contains("apt-get remove"),
        "package with state=absent should generate removal script: {script}"
    );
}

#[test]
fn test_codegen_file_with_owner_and_mode() {
    let mut r = make_file();
    r.owner = Some("www-data".to_string());
    r.mode = Some("0644".to_string());
    r.content = Some("hello".to_string());
    let script = apply_script(&r).unwrap();
    assert!(
        script.contains("chown 'www-data"),
        "file script should set owner to www-data: {script}"
    );
    assert!(
        script.contains("chmod '0644'"),
        "file script should set mode to 0644: {script}"
    );
    assert!(
        script.contains("hello"),
        "file script should contain content 'hello': {script}"
    );
}

#[test]
fn test_codegen_mount_with_options() {
    let mut r = make_mount();
    r.source = Some("/dev/sdb1".to_string());
    r.fs_type = Some("ext4".to_string());
    r.options = Some("noatime,errors=remount-ro".to_string());
    let script = apply_script(&r).unwrap();
    assert!(
        script.contains("mount -t 'ext4'"),
        "mount script should contain fstype ext4: {script}"
    );
    assert!(
        script.contains("noatime,errors=remount-ro"),
        "mount script should contain options: {script}"
    );
    assert!(
        script.contains("/dev/sdb1"),
        "mount script should reference the device: {script}"
    );
}

#[test]
fn test_codegen_service_disabled() {
    let mut r = make_service();
    r.state = Some("stopped".to_string());
    r.enabled = Some(false);
    let script = apply_script(&r).unwrap();
    assert!(
        script.contains("systemctl stop"),
        "stopped service should generate stop command: {script}"
    );
    assert!(
        script.contains("systemctl disable"),
        "disabled service should generate disable command: {script}"
    );
}

#[test]
fn test_codegen_user_absent() {
    let mut r = make_package();
    r.resource_type = ResourceType::User;
    r.name = Some("staleuser".to_string());
    r.state = Some("absent".to_string());
    let script = apply_script(&r).unwrap();
    assert!(
        script.contains("userdel"),
        "user absent should generate userdel: {script}"
    );
    assert!(
        !script.contains("useradd"),
        "user absent must not generate useradd: {script}"
    );
    assert!(
        !script.contains("usermod"),
        "user absent must not generate usermod: {script}"
    );
    assert!(
        script.contains("'staleuser'"),
        "userdel must reference the username: {script}"
    );
}

#[test]
fn test_codegen_docker_with_ports_and_env() {
    let mut r = make_package();
    r.resource_type = ResourceType::Docker;
    r.name = Some("webapp".to_string());
    r.image = Some("myapp:v2".to_string());
    r.state = Some("running".to_string());
    r.ports = vec!["8080:80".to_string(), "8443:443".to_string()];
    r.environment = vec!["DB_HOST=db.local".to_string(), "LOG_LEVEL=info".to_string()];
    r.volumes = vec!["/data:/app/data".to_string()];
    let check = check_script(&r).unwrap();
    assert!(
        check.contains("docker inspect"),
        "docker check must use inspect: {check}"
    );
    let apply = apply_script(&r).unwrap();
    assert!(
        apply.contains("-p '8080:80'"),
        "apply must map port 8080:80: {apply}"
    );
    assert!(
        apply.contains("-p '8443:443'"),
        "apply must map port 8443:443: {apply}"
    );
    assert!(
        apply.contains("-e 'DB_HOST=db.local'"),
        "apply must set env DB_HOST: {apply}"
    );
    assert!(
        apply.contains("-e 'LOG_LEVEL=info'"),
        "apply must set env LOG_LEVEL: {apply}"
    );
    assert!(
        apply.contains("-v '/data:/app/data'"),
        "apply must mount volume: {apply}"
    );
    assert!(
        apply.contains("docker run -d"),
        "apply must run in detached mode: {apply}"
    );
    let query = state_query_script(&r).unwrap();
    assert!(
        query.contains("docker inspect 'webapp'"),
        "state_query must inspect container: {query}"
    );
}

#[test]
fn test_codegen_cron_with_schedule() {
    let mut r = make_package();
    r.resource_type = ResourceType::Cron;
    r.name = Some("nightly-backup".to_string());
    r.owner = Some("deploy".to_string());
    r.schedule = Some("30 2 * * *".to_string());
    r.command = Some("/opt/backup/run.sh".to_string());
    let check = check_script(&r).unwrap();
    assert!(
        check.contains("crontab"),
        "cron check must use crontab: {check}"
    );
    assert!(
        check.contains("forjar:nightly-backup"),
        "cron check must reference job name: {check}"
    );
    let apply = apply_script(&r).unwrap();
    assert!(
        apply.contains("30 2 * * *"),
        "cron apply must include schedule: {apply}"
    );
    assert!(
        apply.contains("/opt/backup/run.sh"),
        "cron apply must include command: {apply}"
    );
    assert!(
        apply.contains("crontab -u 'deploy'"),
        "cron apply must target user 'deploy': {apply}"
    );
    let query = state_query_script(&r).unwrap();
    assert!(
        query.contains("crontab -u 'deploy' -l"),
        "cron state_query must list deploy's crontab: {query}"
    );
}

#[test]
fn test_codegen_network_reject() {
    let mut r = make_package();
    r.resource_type = ResourceType::Network;
    r.port = Some("25".to_string());
    r.protocol = Some("tcp".to_string());
    r.action = Some("reject".to_string());
    r.from_addr = Some("0.0.0.0/0".to_string());
    let check = check_script(&r).unwrap();
    assert!(
        check.contains("ufw status"),
        "network check must query ufw: {check}"
    );
    assert!(
        check.contains("reject"),
        "network check must include action 'reject': {check}"
    );
    assert!(
        check.contains("25/tcp"),
        "network check must include port/proto: {check}"
    );
    let apply = apply_script(&r).unwrap();
    assert!(
        apply.contains("ufw reject"),
        "network apply must use 'ufw reject': {apply}"
    );
    assert!(
        apply.contains("from '0.0.0.0/0'"),
        "network apply must include from_addr: {apply}"
    );
    assert!(
        apply.contains("port '25'"),
        "network apply must include port: {apply}"
    );
    let query = state_query_script(&r).unwrap();
    assert!(
        query.contains("ufw status verbose"),
        "network state_query must use ufw status verbose: {query}"
    );
}
