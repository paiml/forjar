#![allow(unused_imports)]
use super::*;

use super::*;
use crate::core::types::{MachineTarget, ResourceType};
use std::collections::HashMap;

fn make_pepita_resource(name: &str) -> Resource {
    Resource {
        resource_type: ResourceType::Pepita,
        machine: MachineTarget::Single("m1".to_string()),
        state: Some("present".to_string()),
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
        inputs: HashMap::new(),
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

// ── FJ-040: Core pepita resource tests ─────────────────────────

#[test]
fn test_fj040_check_unconfigured() {
    let r = make_pepita_resource("sandbox");
    let script = check_script(&r);
    assert!(script.contains("set -euo pipefail"));
    assert!(script.contains("pepita:sandbox:unconfigured"));
}

#[test]
fn test_fj040_check_with_cgroup() {
    let mut r = make_pepita_resource("worker");
    r.memory_limit = Some(536870912); // 512 MiB
    let script = check_script(&r);
    assert!(script.contains("cgroup:present:worker"));
    assert!(script.contains("cgroup:absent:worker"));
    assert!(script.contains("/sys/fs/cgroup/forjar-worker"));
}

#[test]
fn test_fj040_check_with_chroot() {
    let mut r = make_pepita_resource("jail");
    r.chroot_dir = Some("/var/lib/forjar/jail".to_string());
    let script = check_script(&r);
    assert!(script.contains("chroot:present:jail"));
    assert!(script.contains("chroot:absent:jail"));
    assert!(script.contains("/var/lib/forjar/jail"));
}

#[test]
fn test_fj040_check_with_overlay() {
    let mut r = make_pepita_resource("layered");
    r.overlay_merged = Some("/mnt/merged".to_string());
    let script = check_script(&r);
    assert!(script.contains("overlay:mounted:layered"));
    assert!(script.contains("overlay:unmounted:layered"));
    assert!(script.contains("mountpoint -q '/mnt/merged'"));
}

#[test]
fn test_fj040_check_with_netns() {
    let mut r = make_pepita_resource("isolated");
    r.netns = true;
    let script = check_script(&r);
    assert!(script.contains("netns:present:isolated"));
    assert!(script.contains("netns:absent:isolated"));
    assert!(script.contains("forjar-isolated"));
}

#[test]
fn test_fj040_apply_cgroup_memory() {
    let mut r = make_pepita_resource("worker");
    r.memory_limit = Some(1073741824); // 1 GiB
    let script = apply_script(&r);
    assert!(script.contains("set -euo pipefail"));
    assert!(script.contains("mkdir -p '/sys/fs/cgroup/forjar-worker'"));
    assert!(script.contains("echo '1073741824' > '/sys/fs/cgroup/forjar-worker/memory.max'"));
}

#[test]
fn test_fj040_apply_cgroup_cpuset() {
    let mut r = make_pepita_resource("compute");
    r.cpuset = Some("0-3".to_string());
    let script = apply_script(&r);
    assert!(script.contains("echo '0-3' > '/sys/fs/cgroup/forjar-compute/cpuset.cpus'"));
}

#[test]
fn test_fj040_apply_cgroup_both() {
    let mut r = make_pepita_resource("full");
    r.memory_limit = Some(268435456); // 256 MiB
    r.cpuset = Some("0,2".to_string());
    let script = apply_script(&r);
    assert!(script.contains("memory.max"));
    assert!(script.contains("cpuset.cpus"));
}

#[test]
fn test_fj040_apply_chroot() {
    let mut r = make_pepita_resource("jail");
    r.chroot_dir = Some("/var/jail".to_string());
    let script = apply_script(&r);
    assert!(script.contains("mkdir -p '/var/jail'"));
}

#[test]
fn test_fj040_apply_overlay() {
    let mut r = make_pepita_resource("layered");
    r.overlay_lower = Some("/base".to_string());
    r.overlay_upper = Some("/upper".to_string());
    r.overlay_work = Some("/work".to_string());
    r.overlay_merged = Some("/merged".to_string());
    let script = apply_script(&r);
    assert!(script.contains("mount -t overlay overlay"));
    assert!(script.contains("lowerdir='/base'"));
    assert!(script.contains("upperdir='/upper'"));
    assert!(script.contains("workdir='/work'"));
    assert!(script.contains("'/merged'"));
}

#[test]
fn test_fj040_apply_overlay_defaults() {
    let mut r = make_pepita_resource("layered");
    r.overlay_merged = Some("/merged".to_string());
    // No explicit lower/upper/work — should use defaults
    let script = apply_script(&r);
    assert!(script.contains("mount -t overlay overlay"));
    assert!(script.contains("lowerdir='/'"));
    assert!(script.contains("/tmp/forjar-upper"));
    assert!(script.contains("/tmp/forjar-work"));
}

#[test]
fn test_fj040_apply_netns() {
    let mut r = make_pepita_resource("isolated");
    r.netns = true;
    let script = apply_script(&r);
    assert!(script.contains("ip netns add 'forjar-isolated'"));
    assert!(script.contains("ip link set lo up"));
}

#[test]
fn test_fj040_apply_seccomp() {
    let mut r = make_pepita_resource("secure");
    r.seccomp = true;
    let script = apply_script(&r);
    assert!(script.contains("seccomp:enabled"));
    assert!(script.contains("forjar-secure"));
}

#[test]
fn test_fj040_apply_absent() {
    let mut r = make_pepita_resource("teardown");
    r.state = Some("absent".to_string());
    r.overlay_merged = Some("/merged".to_string());
    r.netns = true;
    r.memory_limit = Some(1024);
    r.chroot_dir = Some("/jail".to_string());
    let script = apply_script(&r);
    assert!(script.contains("umount '/merged'"));
    assert!(script.contains("ip netns del 'forjar-teardown'"));
    assert!(script.contains("rmdir '/sys/fs/cgroup/forjar-teardown'"));
    assert!(script.contains("rm -rf '/jail'"));
}

#[test]
fn test_fj040_apply_absent_tolerant() {
    let mut r = make_pepita_resource("gone");
    r.state = Some("absent".to_string());
    r.netns = true;
    r.overlay_merged = Some("/m".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("|| true"),
        "absent teardown must tolerate missing resources"
    );
}

#[test]
fn test_fj040_state_query_cgroup() {
    let mut r = make_pepita_resource("worker");
    r.memory_limit = Some(1024);
    let script = state_query_script(&r);
    assert!(script.contains("cgroup=worker"));
    assert!(script.contains("cgroup=MISSING:worker"));
}

#[test]
fn test_fj040_state_query_overlay() {
    let mut r = make_pepita_resource("layered");
    r.overlay_merged = Some("/merged".to_string());
    let script = state_query_script(&r);
    assert!(script.contains("overlay=layered"));
    assert!(script.contains("overlay=MISSING:layered"));
}

#[test]
fn test_fj040_state_query_netns() {
    let mut r = make_pepita_resource("net");
    r.netns = true;
    let script = state_query_script(&r);
    assert!(script.contains("netns=net"));
    assert!(script.contains("netns=MISSING:net"));
}

#[test]
fn test_fj040_state_query_chroot() {
    let mut r = make_pepita_resource("jail");
    r.chroot_dir = Some("/var/jail".to_string());
    let script = state_query_script(&r);
    assert!(script.contains("chroot=jail"));
    assert!(script.contains("chroot=MISSING:jail"));
}

#[test]
fn test_fj040_state_query_unconfigured() {
    let r = make_pepita_resource("empty");
    let script = state_query_script(&r);
    assert!(script.contains("pepita=empty:unconfigured"));
}

#[test]
fn test_fj040_full_isolation() {
    let mut r = make_pepita_resource("full-sandbox");
    r.chroot_dir = Some("/var/sandbox".to_string());
    r.namespace_uid = Some(65534);
    r.namespace_gid = Some(65534);
    r.seccomp = true;
    r.netns = true;
    r.cpuset = Some("0-1".to_string());
    r.memory_limit = Some(536870912);
    r.overlay_lower = Some("/base".to_string());
    r.overlay_upper = Some("/upper".to_string());
    r.overlay_work = Some("/work".to_string());
    r.overlay_merged = Some("/merged".to_string());

    let apply = apply_script(&r);
    assert!(apply.contains("mkdir -p '/var/sandbox'"));
    assert!(apply.contains("memory.max"));
    assert!(apply.contains("cpuset.cpus"));
    assert!(apply.contains("mount -t overlay"));
    assert!(apply.contains("ip netns add"));
    assert!(apply.contains("seccomp:enabled"));

    let check = check_script(&r);
    assert!(check.contains("cgroup:present:full-sandbox"));
    assert!(check.contains("chroot:present:full-sandbox"));
    assert!(check.contains("overlay:mounted:full-sandbox"));
    assert!(check.contains("netns:present:full-sandbox"));

    let query = state_query_script(&r);
    assert!(query.contains("cgroup=full-sandbox"));
    assert!(query.contains("overlay=full-sandbox"));
    assert!(query.contains("netns=full-sandbox"));
    assert!(query.contains("chroot=full-sandbox"));
}

#[test]
fn test_fj040_idempotent() {
    let mut r = make_pepita_resource("idem");
    r.netns = true;
    r.memory_limit = Some(1024);
    let s1 = apply_script(&r);
    let s2 = apply_script(&r);
    assert_eq!(s1, s2, "apply_script must be idempotent");
}

#[test]
fn test_fj040_no_name_defaults_to_unknown() {
    let mut r = make_pepita_resource("placeholder");
    r.name = None;
    r.netns = true;
    let check = check_script(&r);
    assert!(check.contains("forjar-unknown"));
    let apply = apply_script(&r);
    assert!(apply.contains("forjar-unknown"));
    let query = state_query_script(&r);
    assert!(query.contains("netns=unknown"));
}

#[test]
fn test_fj040_absent_no_setup() {
    let mut r = make_pepita_resource("gone");
    r.state = Some("absent".to_string());
    r.netns = true;
    let script = apply_script(&r);
    assert!(
        !script.contains("ip netns add"),
        "absent must not create namespace"
    );
    assert!(
        script.contains("ip netns del"),
        "absent must delete namespace"
    );
}
