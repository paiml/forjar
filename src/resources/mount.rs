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
}
