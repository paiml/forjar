//! FJ-009: Mount resource handler (NFS, bind, etc.).

use crate::core::types::Resource;

/// Generate shell to check mount state.
pub fn check_script(resource: &Resource) -> String {
    let target = resource.path.as_deref().unwrap_or("/mnt/unknown");
    format!(
        "mountpoint -q '{target}' 2>/dev/null && echo 'mounted:{target}' || echo 'unmounted:{target}'"
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
            lines.push(format!("mkdir -p '{target}'"));
            // Check if already mounted
            lines.push(format!(
                "if ! mountpoint -q '{target}'; then\n  mount -t '{fstype}' -o '{options}' '{source}' '{target}'\nfi"
            ));
            // Add to fstab if not already there
            lines.push(format!(
                "if ! grep -q '{target}' /etc/fstab 2>/dev/null; then\n  echo '{source} {target} {fstype} {options} 0 0' >> /etc/fstab\nfi"
            ));
        }
        "unmounted" => {
            lines.push(format!(
                "if mountpoint -q '{target}'; then\n  umount '{target}'\nfi"
            ));
        }
        "absent" => {
            lines.push(format!(
                "if mountpoint -q '{target}'; then\n  umount '{target}'\nfi"
            ));
            // Remove from fstab
            lines.push(format!(
                "sed -i '\\|{target}|d' /etc/fstab 2>/dev/null || true"
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
        "if mountpoint -q '{target}'; then\n\
           findmnt -n -o SOURCE,FSTYPE,OPTIONS '{target}' 2>/dev/null\n\
         else\n\
           echo 'UNMOUNTED'\n\
         fi"
    )
}
