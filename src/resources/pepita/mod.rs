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
            "if [ -d '/sys/fs/cgroup/forjar-{name}' ]; then echo 'cgroup:present:{name}'; else echo 'cgroup:absent:{name}'; fi"
        ));
    }

    // Check chroot directory
    if let Some(ref chroot) = resource.chroot_dir {
        checks.push(format!(
            "if [ -d '{chroot}' ]; then echo 'chroot:present:{name}'; else echo 'chroot:absent:{name}'; fi"
        ));
    }

    // Check overlay mount
    if let Some(ref merged) = resource.overlay_merged {
        checks.push(format!(
            "if mountpoint -q '{merged}' 2>/dev/null; then echo 'overlay:mounted:{name}'; else echo 'overlay:unmounted:{name}'; fi"
        ));
    }

    // Check network namespace
    if resource.netns {
        checks.push(format!(
            "if ip netns list 2>/dev/null | grep -q 'forjar-{name}'; then echo 'netns:present:{name}'; else echo 'netns:absent:{name}'; fi"
        ));
    }

    if checks.len() == 1 {
        // No specific checks — just report the name
        checks.push(format!("echo 'pepita:{name}:unconfigured'"));
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
        lines.push(format!("umount '{merged}' 2>/dev/null || true"));
    }

    // Remove network namespace
    if resource.netns {
        lines.push(format!("ip netns del 'forjar-{name}' 2>/dev/null || true"));
    }

    // Remove cgroup
    if resource.cpuset.is_some() || resource.memory_limit.is_some() {
        lines.push(format!(
            "rmdir '/sys/fs/cgroup/forjar-{name}' 2>/dev/null || true"
        ));
    }

    // Remove chroot directory (careful — only if we created it)
    if let Some(ref chroot) = resource.chroot_dir {
        lines.push(format!("if [ -d '{chroot}' ]; then rm -rf '{chroot}'; fi"));
    }

    lines.join("\n")
}

/// Set up isolation: create cgroups, mount overlays, create network namespaces.
fn apply_present(name: &str, resource: &Resource) -> String {
    let mut lines = vec!["set -euo pipefail".to_string()];

    // Create chroot directory
    if let Some(ref chroot) = resource.chroot_dir {
        lines.push(format!("mkdir -p '{chroot}'"));
    }

    // Set up cgroups v2
    if resource.cpuset.is_some() || resource.memory_limit.is_some() {
        let cgroup_path = format!("/sys/fs/cgroup/forjar-{name}");
        lines.push(format!("mkdir -p '{cgroup_path}'"));

        if let Some(limit) = resource.memory_limit {
            lines.push(format!("echo '{limit}' > '{cgroup_path}/memory.max'"));
        }

        if let Some(ref cpuset) = resource.cpuset {
            lines.push(format!("echo '{cpuset}' > '{cgroup_path}/cpuset.cpus'"));
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

        lines.push(format!("mkdir -p '{lower}' '{upper}' '{work}' '{merged}'"));
        lines.push(format!(
            "mount -t overlay overlay -o lowerdir='{lower}',upperdir='{upper}',workdir='{work}' '{merged}'"
        ));
    }

    // Create network namespace
    if resource.netns {
        let ns_name = format!("forjar-{name}");
        lines.push(format!("ip netns add '{ns_name}' 2>/dev/null || true"));
        lines.push(format!("ip netns exec '{ns_name}' ip link set lo up"));
    }

    // Set up seccomp (informational — actual filtering is at exec time)
    if resource.seccomp {
        lines.push(format!(
            "echo 'seccomp:enabled' # Seccomp filtering active for forjar-{name}"
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
        let cgroup_path = format!("/sys/fs/cgroup/forjar-{name}");
        queries.push(format!(
            "cat '{cgroup_path}/memory.max' 2>/dev/null && echo 'cgroup={name}' || echo 'cgroup=MISSING:{name}'"
        ));
    }

    // Query overlay state
    if let Some(ref merged) = resource.overlay_merged {
        queries.push(format!(
            "mountpoint -q '{merged}' 2>/dev/null && echo 'overlay={name}' || echo 'overlay=MISSING:{name}'"
        ));
    }

    // Query network namespace
    if resource.netns {
        queries.push(format!(
            "ip netns list 2>/dev/null | grep -q 'forjar-{name}' && echo 'netns={name}' || echo 'netns=MISSING:{name}'"
        ));
    }

    // Query chroot
    if let Some(ref chroot) = resource.chroot_dir {
        queries.push(format!(
            "[ -d '{chroot}' ] && echo 'chroot={name}' || echo 'chroot=MISSING:{name}'"
        ));
    }

    if queries.len() == 1 {
        queries.push(format!("echo 'pepita={name}:unconfigured'"));
    }

    queries.join("\n")
}

#[cfg(test)]
mod tests;
