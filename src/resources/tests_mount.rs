use super::mount::*;
use crate::core::types::{MachineTarget, Resource, ResourceType};

pub(super) fn make_mount_resource() -> Resource {
    Resource {
        resource_type: ResourceType::Mount,
        machine: MachineTarget::Single("m1".to_string()),
        state: None,
        depends_on: vec![],
        provider: None,
        packages: vec![],
        version: None,
        path: Some("/mnt/lambda-raid".to_string()),
        content: None,
        source: Some("192.168.1.1:/srv/nfs/export".to_string()),
        target: None,
        owner: None,
        group: None,
        mode: None,
        name: None,
        enabled: None,
        restart_on: vec![],
        triggers: vec![],
        fs_type: Some("nfs".to_string()),
        options: Some("ro,hard,intr".to_string()),
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
        pre_apply: None,
        post_apply: None,
        lifecycle: None,
        store: false,
        sudo: false,
        script: None,
        gather: vec![],
        scatter: vec![],
    }
}

#[test]
fn test_fj009_check_mount() {
    let r = make_mount_resource();
    let script = check_script(&r);
    assert!(script.contains("mountpoint -q '/mnt/lambda-raid'"));
}

#[test]
fn test_fj009_apply_mount() {
    let r = make_mount_resource();
    let script = apply_script(&r);
    assert!(script.contains("mkdir -p '/mnt/lambda-raid'"));
    assert!(script.contains("mount -t 'nfs' -o 'ro,hard,intr'"));
    assert!(script.contains("192.168.1.1:/srv/nfs/export"));
    assert!(script.contains("/etc/fstab"));
}

#[test]
fn test_fj009_unmount() {
    let mut r = make_mount_resource();
    r.state = Some("unmounted".to_string());
    let script = apply_script(&r);
    assert!(script.contains("umount '/mnt/lambda-raid'"));
}

#[test]
fn test_fj009_absent() {
    let mut r = make_mount_resource();
    r.state = Some("absent".to_string());
    let script = apply_script(&r);
    assert!(script.contains("umount"));
    assert!(script.contains("sed"));
    assert!(script.contains("fstab"));
}

#[test]
fn test_fj009_state_query_script() {
    let r = make_mount_resource();
    let script = state_query_script(&r);
    assert!(script.contains("mountpoint -q '/mnt/lambda-raid'"));
    assert!(script.contains("findmnt"));
    assert!(script.contains("UNMOUNTED"));
}

#[test]
fn test_fj009_apply_creates_mount_point_dir() {
    let r = make_mount_resource();
    let script = apply_script(&r);
    // mkdir -p must come before mount
    let mkdir_idx = script.find("mkdir -p").unwrap();
    let mount_idx = script.find("mount -t").unwrap();
    assert!(
        mkdir_idx < mount_idx,
        "mkdir must precede mount in apply script"
    );
}

#[test]
fn test_fj009_apply_bind_mount() {
    let mut r = make_mount_resource();
    r.source = Some("/srv/data".to_string());
    r.fs_type = Some("none".to_string());
    r.options = Some("bind".to_string());
    let script = apply_script(&r);
    assert!(script.contains("mount -t 'none' -o 'bind' '/srv/data'"));
}

#[test]
fn test_fj009_apply_default_options() {
    let mut r = make_mount_resource();
    r.options = None;
    let script = apply_script(&r);
    assert!(script.contains("-o 'defaults'"));
}

#[test]
fn test_fj009_apply_default_fstype() {
    let mut r = make_mount_resource();
    r.fs_type = None;
    let script = apply_script(&r);
    assert!(script.contains("-t 'auto'"));
}

#[test]
fn test_fj009_fstab_entry_format() {
    let r = make_mount_resource();
    let script = apply_script(&r);
    // Verify fstab entry has correct fields: source target fstype options dump pass
    assert!(script.contains("192.168.1.1:/srv/nfs/export /mnt/lambda-raid nfs ro,hard,intr 0 0"));
}

#[test]
fn test_fj009_fstab_idempotency() {
    let r = make_mount_resource();
    let script = apply_script(&r);
    // Should check if already in fstab before adding
    assert!(script.contains("grep -q '/mnt/lambda-raid' /etc/fstab"));
}

#[test]
fn test_fj009_absent_removes_fstab_entry() {
    let mut r = make_mount_resource();
    r.state = Some("absent".to_string());
    let script = apply_script(&r);
    assert!(script.contains("sed -i"));
    assert!(script.contains("/mnt/lambda-raid"));
    assert!(script.contains("fstab"));
}

#[test]
fn test_fj009_check_script_default_path() {
    let mut r = make_mount_resource();
    r.path = None;
    let script = check_script(&r);
    assert!(script.contains("/mnt/unknown"));
}

#[test]
fn test_fj009_apply_pipefail() {
    let r = make_mount_resource();
    let script = apply_script(&r);
    assert!(
        script.starts_with("set -euo pipefail"),
        "mount script must start with safety flags"
    );
}

// ── Edge-case tests (FJ-123) ─────────────────────────────────

#[test]
fn test_fj009_all_defaults() {
    // No source, no path, no fstype, no options — all defaults
    let mut r = make_mount_resource();
    r.source = None;
    r.path = None;
    r.fs_type = None;
    r.options = None;
    let script = apply_script(&r);
    assert!(script.contains("mount -t 'auto' -o 'defaults' 'none' '/mnt/unknown'"));
}

#[test]
fn test_fj009_unknown_state_no_op() {
    // Unknown state hits _ => {} — only pipefail emitted
    let mut r = make_mount_resource();
    r.state = Some("remounted".to_string());
    let script = apply_script(&r);
    assert!(!script.contains("mount -t"));
    assert!(!script.contains("umount"));
    assert!(script.starts_with("set -euo pipefail"));
}

#[test]
fn test_fj009_absent_no_path_uses_default() {
    let mut r = make_mount_resource();
    r.state = Some("absent".to_string());
    r.path = None;
    let script = apply_script(&r);
    assert!(script.contains("umount '/mnt/unknown'"));
    assert!(script.contains("sed -i"));
    assert!(script.contains("/mnt/unknown"));
}

#[test]
fn test_fj009_state_query_no_path() {
    let mut r = make_mount_resource();
    r.path = None;
    let script = state_query_script(&r);
    assert!(script.contains("mountpoint -q '/mnt/unknown'"));
    assert!(script.contains("UNMOUNTED"));
}

// --- FJ-132: Mount edge case tests ---
