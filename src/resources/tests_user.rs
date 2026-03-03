use super::user::*;
use crate::core::types::{MachineTarget, Resource, ResourceType};

fn make_user_resource(name: &str) -> Resource {
    Resource {
        resource_type: ResourceType::User,
        machine: MachineTarget::Single("m1".to_string()),
        state: None,
        depends_on: vec![],
        provider: None,
        packages: vec![],
        version: None,
        path: None,
        content: None,
        source: None,
        target: None,
        owner: None,
        group: None,
        mode: None,
        name: Some(name.to_string()),
        enabled: None,
        restart_on: vec![],
        triggers: vec![],
        fs_type: None,
        options: None,
        uid: None,
        shell: None,
        home: None,
        groups: vec![],
        ssh_authorized_keys: vec![],
        system_user: false,
        schedule: None,
        command: None,
        image: None,
        ports: vec![],
        environment: vec![],
        volumes: vec![],
        restart: None,
        protocol: None,
        port: None,
        action: None,
        from_addr: None,
        recipe: None,
        inputs: std::collections::HashMap::new(),
        arch: vec![],
        tags: vec![],
        resource_group: None,
        when: None,
        count: None,
        for_each: None,
        chroot_dir: None,
        namespace_uid: None,
        namespace_gid: None,
        seccomp: false,
        netns: false,
        cpuset: None,
        memory_limit: None,
        overlay_lower: None,
        overlay_upper: None,
        overlay_work: None,
        overlay_merged: None,
        format: None,
        quantization: None,
        checksum: None,
        cache_dir: None,
        gpu_backend: None,
        driver_version: None,
        cuda_version: None,
        rocm_version: None,
        devices: vec![],
        persistence_mode: None,
        compute_mode: None,
        gpu_memory_limit_mb: None,
        output_artifacts: vec![],
        completion_check: None,
        timeout: None,
        working_dir: None,
        pre_apply: None,
        post_apply: None,
        lifecycle: None,
        store: false,
            sudo: false,
        script: None,
    }
}

#[test]
fn test_fj031_check_user() {
    let r = make_user_resource("deploy");
    let script = check_script(&r);
    assert!(script.contains("id 'deploy'"));
    assert!(script.contains("exists:deploy"));
    assert!(script.contains("missing:deploy"));
}

#[test]
fn test_fj031_apply_present_basic() {
    let r = make_user_resource("deploy");
    let script = apply_script(&r);
    assert!(script.contains("set -euo pipefail"));
    assert!(script.contains("useradd"));
    assert!(script.contains("'deploy'"));
    assert!(script.contains("--create-home"));
}

#[test]
fn test_fj031_apply_with_shell_and_home() {
    let mut r = make_user_resource("app");
    r.shell = Some("/bin/zsh".to_string());
    r.home = Some("/opt/app".to_string());
    let script = apply_script(&r);
    assert!(script.contains("--shell '/bin/zsh'"));
    assert!(script.contains("--home-dir '/opt/app'"));
}

#[test]
fn test_fj031_apply_with_uid() {
    let mut r = make_user_resource("svc");
    r.uid = Some(1500);
    let script = apply_script(&r);
    assert!(script.contains("--uid 1500"));
}

#[test]
fn test_fj031_apply_with_groups() {
    let mut r = make_user_resource("deploy");
    r.groups = vec!["docker".to_string(), "sudo".to_string()];
    let script = apply_script(&r);
    assert!(script.contains("groupadd 'docker'"));
    assert!(script.contains("groupadd 'sudo'"));
    assert!(script.contains("--groups 'docker,sudo'"));
}

#[test]
fn test_fj031_apply_system_user() {
    let mut r = make_user_resource("prometheus");
    r.system_user = true;
    let script = apply_script(&r);
    assert!(script.contains("--system"));
    // System users don't get --create-home by default
    assert!(!script.contains("--create-home"));
}

#[test]
fn test_fj031_apply_absent() {
    let mut r = make_user_resource("olduser");
    r.state = Some("absent".to_string());
    let script = apply_script(&r);
    assert!(script.contains("userdel"));
    assert!(script.contains("'olduser'"));
}

#[test]
fn test_fj031_apply_with_ssh_keys() {
    let mut r = make_user_resource("deploy");
    r.ssh_authorized_keys = vec!["ssh-ed25519 AAAA... deploy@host".to_string()];
    let script = apply_script(&r);
    assert!(script.contains("mkdir -p '/home/deploy'/.ssh"));
    assert!(script.contains("chmod 700"));
    assert!(script.contains("authorized_keys"));
    assert!(script.contains("ssh-ed25519 AAAA"));
}

#[test]
fn test_fj031_apply_with_primary_group() {
    let mut r = make_user_resource("app");
    r.group = Some("appgroup".to_string());
    let script = apply_script(&r);
    assert!(script.contains("--gid 'appgroup'"));
}

#[test]
fn test_fj031_state_query() {
    let r = make_user_resource("deploy");
    let script = state_query_script(&r);
    assert!(script.contains("id 'deploy'"));
    assert!(script.contains("getent passwd 'deploy'"));
    assert!(script.contains("user=MISSING"));
}

#[test]
fn test_fj031_apply_usermod_on_existing() {
    let r = make_user_resource("deploy");
    let script = apply_script(&r);
    // Should contain both create and modify branches
    assert!(script.contains("useradd"));
    assert!(script.contains("usermod"));
}

#[test]
fn test_fj031_sudo_detection() {
    let r = make_user_resource("deploy");
    let script = apply_script(&r);
    assert!(script.contains("SUDO=\"\""));
    assert!(script.contains("id -u"));
    assert!(script.contains("$SUDO useradd"));
}

/// Verify single-quoting prevents injection in username.
#[test]
fn test_fj031_quoted_username() {
    let r = make_user_resource("user; rm -rf /");
    let script = apply_script(&r);
    assert!(script.contains("'user; rm -rf /'"));
}

#[test]
fn test_fj031_ssh_keys_custom_home() {
    let mut r = make_user_resource("app");
    r.home = Some("/opt/app".to_string());
    r.ssh_authorized_keys = vec!["ssh-rsa AAAA...".to_string()];
    let script = apply_script(&r);
    assert!(
        script.contains("mkdir -p '/opt/app'/.ssh"),
        "should use custom home dir for .ssh"
    );
}

#[test]
fn test_fj031_absent_idempotent() {
    // absent should check if user exists before deleting
    let mut r = make_user_resource("gone");
    r.state = Some("absent".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("if id 'gone'"),
        "absent should check existence first"
    );
}

#[test]
fn test_fj031_absent_userdel_fallback() {
    // userdel -r can fail (mailbox), fallback to userdel without -r
    let mut r = make_user_resource("gone");
    r.state = Some("absent".to_string());
    let script = apply_script(&r);
    assert!(script.contains("userdel -r 'gone'"));
    assert!(script.contains("|| $SUDO userdel 'gone'"));
}

#[test]
fn test_fj031_ssh_key_permissions() {
    let mut r = make_user_resource("deploy");
    r.ssh_authorized_keys = vec!["ssh-ed25519 KEY".to_string()];
    let script = apply_script(&r);
    assert!(script.contains("chmod 700"), ".ssh dir should be 700");
    assert!(
        script.contains("chmod 600"),
        "authorized_keys should be 600"
    );
    assert!(
        script.contains("chown -R 'deploy'"),
        "ssh dir should be owned by user"
    );
}

#[test]
fn test_fj031_multiple_ssh_keys() {
    let mut r = make_user_resource("deploy");
    r.ssh_authorized_keys = vec![
        "ssh-ed25519 KEY1 deploy@laptop".to_string(),
        "ssh-rsa KEY2 deploy@desktop".to_string(),
    ];
    let script = apply_script(&r);
    assert!(script.contains("ssh-ed25519 KEY1"));
    assert!(script.contains("ssh-rsa KEY2"));
}

#[test]
fn test_fj031_group_ensures_before_create() {
    let mut r = make_user_resource("app");
    r.groups = vec!["docker".to_string()];
    let script = apply_script(&r);
    let groupadd_idx = script.find("groupadd").unwrap();
    let useradd_idx = script.find("useradd").unwrap();
    assert!(groupadd_idx < useradd_idx, "groupadd must precede useradd");
}

// ── Edge-case tests (FJ-124) ─────────────────────────────────

#[test]
fn test_fj031_no_name_defaults_to_unknown() {
    let mut r = make_user_resource("placeholder");
    r.name = None;
    let check = check_script(&r);
    assert!(check.contains("id 'unknown'"));
    let apply = apply_script(&r);
    assert!(apply.contains("useradd") && apply.contains("'unknown'"));
    let query = state_query_script(&r);
    assert!(query.contains("id 'unknown'"));
}

#[test]
fn test_fj031_system_user_with_explicit_home() {
    // system_user + explicit home: gets --system AND --home-dir
    let mut r = make_user_resource("prometheus");
    r.system_user = true;
    r.home = Some("/opt/prometheus".to_string());
    let script = apply_script(&r);
    assert!(script.contains("--system"));
    assert!(script.contains("--home-dir '/opt/prometheus'"));
}

#[test]
fn test_fj031_ssh_keys_chown_uses_primary_group() {
    // When primary group is set, chown should use it instead of username
    let mut r = make_user_resource("deploy");
    r.group = Some("deployers".to_string());
    r.ssh_authorized_keys = vec!["ssh-ed25519 KEY".to_string()];
    let script = apply_script(&r);
    assert!(script.contains("chown -R 'deploy':'deployers'"));
}

#[test]
fn test_fj031_modify_branch_carries_all_fields() {
    // Existing user path (usermod) should carry shell, home, uid, gid, groups
    let mut r = make_user_resource("app");
    r.shell = Some("/bin/fish".to_string());
    r.home = Some("/opt/app".to_string());
    r.uid = Some(2000);
    r.group = Some("appgrp".to_string());
    r.groups = vec!["docker".to_string()];
    let script = apply_script(&r);
    // usermod branch
    assert!(script.contains("usermod --shell '/bin/fish'"));
    assert!(script.contains("usermod") && script.contains("--home '/opt/app'"));
    assert!(script.contains("--uid 2000"));
    assert!(script.contains("--gid 'appgrp'"));
    assert!(script.contains("--groups 'docker'"));
}

// ── FJ-036: Additional user resource tests ───────────────────

#[test]
fn test_fj036_user_apply_with_ssh_keys() {
    // SSH key deployment should create .ssh dir and write authorized_keys
    let mut r = make_user_resource("operator");
    r.ssh_authorized_keys = vec![
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAA operator@workstation".to_string(),
        "ssh-rsa AAAAB3NzaC1yc2EAAAA operator@laptop".to_string(),
    ];
    let script = apply_script(&r);
    // Must create .ssh directory
    assert!(
        script.contains("mkdir -p '/home/operator'/.ssh"),
        "must create .ssh directory"
    );
    // Must set correct permissions on .ssh dir
    assert!(
        script.contains("chmod 700 '/home/operator'/.ssh"),
        "must set .ssh dir to 700"
    );
    // Must deploy both keys
    assert!(
        script.contains("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAA operator@workstation"),
        "must include ed25519 key"
    );
    assert!(
        script.contains("ssh-rsa AAAAB3NzaC1yc2EAAAA operator@laptop"),
        "must include rsa key"
    );
    // Must set correct permissions on authorized_keys
    assert!(
        script.contains("chmod 600") && script.contains("authorized_keys"),
        "must set authorized_keys to 600"
    );
    // Must set ownership
    assert!(
        script.contains("chown -R 'operator'"),
        "must set ownership to user"
    );
}

#[test]
fn test_fj036_user_apply_system_user() {
    // system_user=true must add --system flag and must NOT add --create-home
    let mut r = make_user_resource("nodeexporter");
    r.system_user = true;
    let script = apply_script(&r);
    assert!(
        script.contains("--system"),
        "system_user=true must add --system flag to useradd"
    );
    assert!(
        !script.contains("--create-home"),
        "system users should not get --create-home by default"
    );
    assert!(
        script.contains("useradd --system"),
        "system flag must be part of useradd command"
    );
}

#[test]
fn test_fj153_user_all_fields_present() {
    let mut r = make_user_resource("fulluser");
    r.uid = Some(3000);
    r.shell = Some("/bin/fish".to_string());
    r.home = Some("/srv/fulluser".to_string());
    r.group = Some("staff".to_string());
    r.groups = vec!["docker".to_string(), "wheel".to_string()];
    r.ssh_authorized_keys = vec![
        "ssh-ed25519 KEY1 user@host1".to_string(),
        "ssh-rsa KEY2 user@host2".to_string(),
    ];
    let script = apply_script(&r);
    assert!(script.contains("--uid 3000"));
    assert!(script.contains("--shell '/bin/fish'"));
    assert!(script.contains("--home-dir '/srv/fulluser'"));
    assert!(script.contains("--gid 'staff'"));
    assert!(script.contains("--groups 'docker,wheel'"));
    assert!(script.contains("groupadd 'docker'"));
    assert!(script.contains("groupadd 'wheel'"));
    assert!(script.contains("mkdir -p '/srv/fulluser'/.ssh"));
    assert!(script.contains("ssh-ed25519 KEY1"));
    assert!(script.contains("ssh-rsa KEY2"));
    assert!(script.contains("chown -R 'fulluser':'staff'"));
}

#[test]
fn test_fj153_user_system_no_home_no_create() {
    let mut r = make_user_resource("daemon-svc");
    r.system_user = true;
    r.home = None;
    let script = apply_script(&r);
    assert!(script.contains("--system"));
    assert!(!script.contains("--create-home"));
    assert!(!script.contains("--home-dir"));
}

#[test]
fn test_fj153_user_absent_with_all_fields() {
    let mut r = make_user_resource("old");
    r.state = Some("absent".to_string());
    r.uid = Some(5000);
    r.shell = Some("/bin/bash".to_string());
    r.groups = vec!["docker".to_string()];
    r.ssh_authorized_keys = vec!["ssh-ed25519 KEY".to_string()];
    let script = apply_script(&r);
    assert!(script.contains("userdel"));
    assert!(!script.contains("useradd"));
    assert!(!script.contains("usermod"));
    assert!(!script.contains("groupadd"));
    assert!(!script.contains(".ssh"));
}

#[test]
fn test_fj036_user_check_absent() {
    // state=absent must generate userdel, not useradd/usermod
    let mut r = make_user_resource("staleuser");
    r.state = Some("absent".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("userdel"),
        "absent state must generate userdel"
    );
    assert!(
        script.contains("'staleuser'"),
        "userdel must reference the username"
    );
    assert!(
        !script.contains("useradd"),
        "absent state must not create user"
    );
    assert!(
        !script.contains("usermod"),
        "absent state must not modify user"
    );
    // Should check if user exists before deleting
    assert!(
        script.contains("if id 'staleuser'"),
        "absent must check existence before deleting"
    );
}
