//! FJ-009: Mount resource handler (NFS, bind, etc.).

use crate::core::types::Resource;

/// Generate shell to check mount state.
pub fn check_script(resource: &Resource) -> String {
    let target = resource.path.as_deref().unwrap_or("/mnt/unknown");
    format!(
        "mountpoint -q '{}' 2>/dev/null && echo 'mounted:{}' || echo 'unmounted:{}'",
        target, target, target
    )
}

/// Generate shell to converge mount to desired state.
pub fn apply_script(resource: &Resource) -> String {
    let source = resource.source.as_deref().unwrap_or("none");
    let target = resource.path.as_deref().unwrap_or("/mnt/unknown");
    let fstype = resource.fs_type.as_deref().unwrap_or("auto");
    let options = resource.options.as_deref().unwrap_or("defaults");
    let state = resource.state.as_deref().unwrap_or("mounted");

    let mut lines = vec!["set -euo pipefail".to_string()];

    match state {
        "mounted" => {
            lines.push(format!("mkdir -p '{}'", target));
            // Check if already mounted
            lines.push(format!(
                "if ! mountpoint -q '{}'; then\n  mount -t '{}' -o '{}' '{}' '{}'\nfi",
                target, fstype, options, source, target
            ));
            // Add to fstab if not already there
            lines.push(format!(
                "if ! grep -q '{}' /etc/fstab 2>/dev/null; then\n  echo '{} {} {} {} 0 0' >> /etc/fstab\nfi",
                target, source, target, fstype, options
            ));
        }
        "unmounted" => {
            lines.push(format!(
                "if mountpoint -q '{}'; then\n  umount '{}'\nfi",
                target, target
            ));
        }
        "absent" => {
            lines.push(format!(
                "if mountpoint -q '{}'; then\n  umount '{}'\nfi",
                target, target
            ));
            // Remove from fstab
            lines.push(format!(
                "sed -i '\\|{}|d' /etc/fstab 2>/dev/null || true",
                target
            ));
        }
        _ => {}
    }

    lines.join("\n")
}

/// Generate shell to query mount state (for hashing).
pub fn state_query_script(resource: &Resource) -> String {
    let target = resource.path.as_deref().unwrap_or("/mnt/unknown");
    format!(
        "if mountpoint -q '{}'; then\n\
           findmnt -n -o SOURCE,FSTYPE,OPTIONS '{}' 2>/dev/null\n\
         else\n\
           echo 'UNMOUNTED'\n\
         fi",
        target, target
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{MachineTarget, ResourceType};

    fn make_mount_resource() -> Resource {
        Resource {
            resource_type: ResourceType::Mount,
            machine: MachineTarget::Single("m1".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
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
            fs_type: Some("nfs".to_string()),
            options: Some("ro,hard,intr".to_string()),
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
}
