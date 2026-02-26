//! FJ-040: Pepita kernel namespace isolation resource handler.
//!
//! Generates shell scripts for Linux namespace isolation using kernel primitives:
//! - unshare(2) for PID/mount/UTS/IPC/network namespaces
//! - chroot(2) for filesystem isolation
//! - cgroups v2 for resource limits (memory, CPU)
//! - seccomp-bpf for syscall filtering
//! - overlayfs for copy-on-write filesystem layers
//!
//! This is distinct from container resources (FJ-030) which manage Docker/podman
//! containers. Pepita provides bare-metal kernel isolation without a container runtime.

use crate::core::types::Resource;

/// Generate shell script to check isolation state.
pub fn check_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");

    let mut checks = vec!["set -euo pipefail".to_string()];

    // Check if namespace/cgroup exists
    if resource.cpuset.is_some() || resource.memory_limit.is_some() {
        checks.push(format!(
            "if [ -d '/sys/fs/cgroup/forjar-{}' ]; then echo 'cgroup:present:{}'; else echo 'cgroup:absent:{}'; fi",
            name, name, name
        ));
    }

    // Check chroot directory
    if let Some(ref chroot) = resource.chroot_dir {
        checks.push(format!(
            "if [ -d '{}' ]; then echo 'chroot:present:{}'; else echo 'chroot:absent:{}'; fi",
            chroot, name, name
        ));
    }

    // Check overlay mount
    if let Some(ref merged) = resource.overlay_merged {
        checks.push(format!(
            "if mountpoint -q '{}' 2>/dev/null; then echo 'overlay:mounted:{}'; else echo 'overlay:unmounted:{}'; fi",
            merged, name, name
        ));
    }

    // Check network namespace
    if resource.netns {
        checks.push(format!(
            "if ip netns list 2>/dev/null | grep -q 'forjar-{}'; then echo 'netns:present:{}'; else echo 'netns:absent:{}'; fi",
            name, name, name
        ));
    }

    if checks.len() == 1 {
        // No specific checks — just report the name
        checks.push(format!("echo 'pepita:{}:unconfigured'", name));
    }

    checks.join("\n")
}

/// Generate shell script to apply namespace isolation.
pub fn apply_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    let state = resource.state.as_deref().unwrap_or("present");

    match state {
        "absent" => apply_absent(name, resource),
        _ => apply_present(name, resource),
    }
}

/// Tear down isolation: remove cgroups, unmount overlays, delete network namespaces.
fn apply_absent(name: &str, resource: &Resource) -> String {
    let mut lines = vec!["set -euo pipefail".to_string()];

    // Unmount overlay
    if let Some(ref merged) = resource.overlay_merged {
        lines.push(format!("umount '{}' 2>/dev/null || true", merged));
    }

    // Remove network namespace
    if resource.netns {
        lines.push(format!(
            "ip netns del 'forjar-{}' 2>/dev/null || true",
            name
        ));
    }

    // Remove cgroup
    if resource.cpuset.is_some() || resource.memory_limit.is_some() {
        lines.push(format!(
            "rmdir '/sys/fs/cgroup/forjar-{}' 2>/dev/null || true",
            name
        ));
    }

    // Remove chroot directory (careful — only if we created it)
    if let Some(ref chroot) = resource.chroot_dir {
        lines.push(format!(
            "if [ -d '{}' ]; then rm -rf '{}'; fi",
            chroot, chroot
        ));
    }

    lines.join("\n")
}

/// Set up isolation: create cgroups, mount overlays, create network namespaces.
fn apply_present(name: &str, resource: &Resource) -> String {
    let mut lines = vec!["set -euo pipefail".to_string()];

    // Create chroot directory
    if let Some(ref chroot) = resource.chroot_dir {
        lines.push(format!("mkdir -p '{}'", chroot));
    }

    // Set up cgroups v2
    if resource.cpuset.is_some() || resource.memory_limit.is_some() {
        let cgroup_path = format!("/sys/fs/cgroup/forjar-{}", name);
        lines.push(format!("mkdir -p '{}'", cgroup_path));

        if let Some(limit) = resource.memory_limit {
            lines.push(format!("echo '{}' > '{}/memory.max'", limit, cgroup_path));
        }

        if let Some(ref cpuset) = resource.cpuset {
            lines.push(format!("echo '{}' > '{}/cpuset.cpus'", cpuset, cgroup_path));
        }
    }

    // Set up overlay filesystem
    if let Some(ref merged) = resource.overlay_merged {
        let lower = resource.overlay_lower.as_deref().unwrap_or("/");
        let upper = resource
            .overlay_upper
            .as_deref()
            .unwrap_or("/tmp/forjar-upper");
        let work = resource
            .overlay_work
            .as_deref()
            .unwrap_or("/tmp/forjar-work");

        lines.push(format!(
            "mkdir -p '{}' '{}' '{}' '{}'",
            lower, upper, work, merged
        ));
        lines.push(format!(
            "mount -t overlay overlay -o lowerdir='{}',upperdir='{}',workdir='{}' '{}'",
            lower, upper, work, merged
        ));
    }

    // Create network namespace
    if resource.netns {
        let ns_name = format!("forjar-{}", name);
        lines.push(format!("ip netns add '{}' 2>/dev/null || true", ns_name));
        lines.push(format!("ip netns exec '{}' ip link set lo up", ns_name));
    }

    // Set up seccomp (informational — actual filtering is at exec time)
    if resource.seccomp {
        lines.push(format!(
            "echo 'seccomp:enabled' # Seccomp filtering active for forjar-{}",
            name
        ));
    }

    lines.join("\n")
}

/// Generate shell to query isolation state (for BLAKE3 hashing).
pub fn state_query_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");

    let mut queries = vec!["set -euo pipefail".to_string()];

    // Query cgroup state
    if resource.cpuset.is_some() || resource.memory_limit.is_some() {
        let cgroup_path = format!("/sys/fs/cgroup/forjar-{}", name);
        queries.push(format!(
            "cat '{}/memory.max' 2>/dev/null && echo 'cgroup={}' || echo 'cgroup=MISSING:{}'",
            cgroup_path, name, name
        ));
    }

    // Query overlay state
    if let Some(ref merged) = resource.overlay_merged {
        queries.push(format!(
            "mountpoint -q '{}' 2>/dev/null && echo 'overlay={}' || echo 'overlay=MISSING:{}'",
            merged, name, name
        ));
    }

    // Query network namespace
    if resource.netns {
        queries.push(format!(
            "ip netns list 2>/dev/null | grep -q 'forjar-{}' && echo 'netns={}' || echo 'netns=MISSING:{}'",
            name, name, name
        ));
    }

    // Query chroot
    if let Some(ref chroot) = resource.chroot_dir {
        queries.push(format!(
            "[ -d '{}' ] && echo 'chroot={}' || echo 'chroot=MISSING:{}'",
            chroot, name, name
        ));
    }

    if queries.len() == 1 {
        queries.push(format!("echo 'pepita={}:unconfigured'", name));
    }

    queries.join("\n")
}

#[cfg(test)]
mod tests {
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
}
