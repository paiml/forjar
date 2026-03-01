use super::mount::*;
use crate::core::types::{MachineTarget, Resource, ResourceType};

fn make_mount_resource() -> Resource {
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
        source: Some("192.168.50.50:/mnt/nvme-raid0".to_string()),
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
        pre_apply: None,
        post_apply: None,
        lifecycle: None,
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
    assert!(script.contains("192.168.50.50:/mnt/nvme-raid0"));
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
    assert!(
        script.contains("192.168.50.50:/mnt/nvme-raid0 /mnt/lambda-raid nfs ro,hard,intr 0 0")
    );
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

#[test]
fn test_fj132_state_query_mounted_branch() {
    // state_query_script should have both mounted and unmounted branches
    let r = make_mount_resource();
    let script = state_query_script(&r);
    assert!(script.contains("mountpoint -q"), "should check if mounted");
    assert!(
        script.contains("findmnt"),
        "should use findmnt for mounted info"
    );
    assert!(
        script.contains("UNMOUNTED"),
        "should have unmounted fallback"
    );
}

#[test]
fn test_fj132_apply_unmounted_conditional() {
    // unmounted state should check mountpoint before unmounting
    let mut r = make_mount_resource();
    r.state = Some("unmounted".to_string());
    let script = apply_script(&r);
    assert!(script.contains("umount"), "should attempt umount");
    assert!(!script.contains("mount -t"), "should not attempt mounting");
}

#[test]
fn test_fj132_fstab_grep_idempotency() {
    // mounted state should grep fstab before appending
    let r = make_mount_resource();
    let script = apply_script(&r);
    assert!(
        script.contains("grep -q"),
        "should check fstab before appending"
    );
    assert!(script.contains("/etc/fstab"), "should reference fstab");
}

#[test]
fn test_fj132_check_script_idempotent() {
    let r = make_mount_resource();
    let s1 = check_script(&r);
    let s2 = check_script(&r);
    assert_eq!(s1, s2, "check_script must be idempotent");
}

#[test]
fn test_fj132_apply_nfs_mount() {
    // NFS mounts should use the nfs fstype
    let mut r = make_mount_resource();
    r.source = Some("192.168.1.10:/exports/data".to_string());
    r.fs_type = Some("nfs".to_string());
    r.options = Some("rw,soft,intr".to_string());
    let script = apply_script(&r);
    assert!(script.contains("mount -t 'nfs'"));
    assert!(script.contains("'rw,soft,intr'"));
    assert!(script.contains("192.168.1.10:/exports/data"));
}

// ── FJ-036: Mount resource handler tests ────────────────────

#[test]
fn test_fj036_mount_apply_creates_mountpoint() {
    let mut r = make_mount_resource();
    r.path = Some("/data/warehouse".to_string());
    r.source = Some("10.0.0.1:/exports/warehouse".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("mkdir -p '/data/warehouse'"),
        "apply must create the mountpoint directory before mounting"
    );
    let mkdir_idx = script.find("mkdir -p '/data/warehouse'").unwrap();
    let mount_idx = script.find("mount -t").unwrap();
    assert!(
        mkdir_idx < mount_idx,
        "mkdir must precede mount command in the script"
    );
}

#[test]
fn test_fj036_mount_apply_adds_fstab() {
    let mut r = make_mount_resource();
    r.path = Some("/mnt/shared".to_string());
    r.source = Some("nas:/vol1".to_string());
    r.fs_type = Some("nfs4".to_string());
    r.options = Some("rw,noatime".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("/etc/fstab"),
        "apply must reference /etc/fstab"
    );
    assert!(
        script.contains("nas:/vol1 /mnt/shared nfs4 rw,noatime 0 0"),
        "apply must add correctly formatted fstab entry"
    );
    assert!(
        script.contains("grep -q '/mnt/shared' /etc/fstab"),
        "apply must check fstab idempotently before appending"
    );
}

#[test]
fn test_fj036_mount_state_query_contains_mountpoint() {
    let mut r = make_mount_resource();
    r.path = Some("/srv/nfs-data".to_string());
    let script = state_query_script(&r);
    assert!(
        script.contains("mountpoint -q '/srv/nfs-data'"),
        "state_query must check the mountpoint"
    );
    assert!(
        script.contains("findmnt -n -o SOURCE,FSTYPE,OPTIONS '/srv/nfs-data'"),
        "state_query must query mount details for the mountpoint"
    );
}

// -- Coverage boost tests --

#[test]
fn test_mount_check_mounted() {
    let mut r = make_mount_resource();
    r.path = Some("/mnt/backup-vol".to_string());
    let script = check_script(&r);
    assert!(
        script.contains("mountpoint -q '/mnt/backup-vol'"),
        "check must use mountpoint -q on the target: {script}"
    );
    assert!(
        script.contains("mounted:/mnt/backup-vol"),
        "check must emit mounted token: {script}"
    );
    assert!(
        script.contains("unmounted:/mnt/backup-vol"),
        "check must emit unmounted token: {script}"
    );
}

#[test]
fn test_mount_absent_cleanup() {
    let mut r = make_mount_resource();
    r.path = Some("/mnt/old-share".to_string());
    r.state = Some("absent".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("set -euo pipefail"),
        "absent script must have safety flags: {script}"
    );
    assert!(
        script.contains("umount '/mnt/old-share'"),
        "absent must generate umount: {script}"
    );
    assert!(
        script.contains("sed -i"),
        "absent must use sed to remove fstab entry: {script}"
    );
    assert!(
        script.contains("/mnt/old-share"),
        "sed pattern must reference the mount path: {script}"
    );
    assert!(
        script.contains("fstab"),
        "absent must reference /etc/fstab: {script}"
    );
    assert!(
        !script.contains("mount -t"),
        "absent must not mount anything: {script}"
    );
    assert!(
        !script.contains("mkdir"),
        "absent must not create directories: {script}"
    );
}

#[test]
fn test_fj153_mount_default_state_is_mounted() {
    let mut r = make_mount_resource();
    r.state = None;
    let script = apply_script(&r);
    assert!(script.contains("mount -t"), "default state should mount");
    assert!(
        script.contains("mkdir -p"),
        "default state should create dir"
    );
    assert!(
        script.contains("/etc/fstab"),
        "default state should update fstab"
    );
}

#[test]
fn test_fj153_unmounted_no_fstab_no_mkdir() {
    let mut r = make_mount_resource();
    r.state = Some("unmounted".to_string());
    let script = apply_script(&r);
    assert!(script.contains("umount"));
    assert!(!script.contains("mkdir"), "unmounted must not create dirs");
    assert!(!script.contains("sed"), "unmounted must not modify fstab");
    assert!(!script.contains("grep"), "unmounted must not check fstab");
}

#[test]
fn test_fj153_mount_tmpfs() {
    let mut r = make_mount_resource();
    r.source = Some("tmpfs".to_string());
    r.path = Some("/tmp/ramdisk".to_string());
    r.fs_type = Some("tmpfs".to_string());
    r.options = Some("size=512m,mode=1777".to_string());
    let script = apply_script(&r);
    assert!(script.contains("mount -t 'tmpfs' -o 'size=512m,mode=1777' 'tmpfs' '/tmp/ramdisk'"));
}

#[test]
fn test_fj153_absent_no_source_defaults() {
    let mut r = make_mount_resource();
    r.state = Some("absent".to_string());
    r.source = None;
    let script = apply_script(&r);
    assert!(script.contains("umount"));
    assert!(script.contains("sed -i"));
    assert!(!script.contains("mount -t"));
}

#[test]
fn test_mount_bind_type() {
    let mut r = make_mount_resource();
    r.path = Some("/srv/container-data".to_string());
    r.source = Some("/data/volumes/app1".to_string());
    r.fs_type = Some("none".to_string());
    r.options = Some("rbind".to_string());
    r.state = None;
    let script = apply_script(&r);
    assert!(
        script
            .contains("mount -t 'none' -o 'rbind' '/data/volumes/app1' '/srv/container-data'"),
        "bind mount must use correct fstype, options, source, and target: {script}"
    );
    assert!(
        script.contains("mkdir -p '/srv/container-data'"),
        "bind mount must create target directory: {script}"
    );
    assert!(
        script.contains("/data/volumes/app1 /srv/container-data none rbind 0 0"),
        "fstab entry must have correct format for bind mount: {script}"
    );
}
