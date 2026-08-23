//! FJ-031 user-resource tests, part B.
//!
//! Split from `tests_user.rs` when it crossed the repo's 500-line ceiling while
//! its authorized_keys assertions were being strengthened from `script
//! .contains(<key>)` to asserting on the deployed content (C8, GH #296). The
//! tests below are moved verbatim.

use super::user::*;
use crate::core::types::{MachineTarget, Resource, ResourceType};

fn make_user_resource(name: &str) -> Resource {
    Resource {
        phony: false,
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
        task_mode: None,
        task_inputs: vec![],
        stages: vec![],
        cache: false,
        gpu_device: None,
        restart_delay: None,
        quality_gate: None,
        health_check: None,
        restart_policy: None,
        pre_apply: None,
        post_apply: None,
        lifecycle: None,
        store: false,
        sudo: false,
        script: None,
        gather: vec![],
        scatter: vec![],
        build_machine: None,
        repo: None,
        tag: None,
        asset_pattern: None,
        binary: None,
        install_dir: None,
        overlay_ip: None,
        overlay_iface: None,
        overlay_hosts: None,
        overlay_firewall: None,
        ..Default::default()
    }
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
