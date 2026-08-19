//! FJ-009: Mount resource handler (NFS, bind, etc.).

use crate::core::shell_escape::sh_squote;
use crate::core::types::Resource;
use crate::resources::verdict;

/// Escape sed BRE metacharacters so a path is matched literally inside a
/// `\|PATTERN|d` address. Escapes the `|` delimiter, `\` and the regex
/// specials `.`, `*`, `[`, `]`, `^`, `$`.
fn sed_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(c, '\\' | '|' | '.' | '*' | '[' | ']' | '^' | '$' | '/') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Generate shell to check mount state.
pub fn check_script(resource: &Resource) -> String {
    let target = resource.path.as_deref().unwrap_or("/mnt/unknown");
    let t = sh_squote(target);
    // The status labels embed the config-derived `target`, so route them
    // through sh_squote too — a raw label could close the single quote and
    // run command substitution (matches docker.rs/package.rs).
    verdict::single(
        &format!("mountpoint -q {t} 2>/dev/null"),
        &format!("mounted:{target}"),
        &format!("unmounted:{target}"),
    )
}

/// Generate shell to converge mount to desired state.
pub fn apply_script(resource: &Resource) -> String {
    let source = resource.source.as_deref().unwrap_or("none");
    let target = resource.path.as_deref().unwrap_or("/mnt/unknown");
    let fstype = resource.fs_type.as_deref().unwrap_or("auto");
    let options = resource.options.as_deref().unwrap_or("defaults");
    let state = resource.state.as_deref().unwrap_or("mounted");

    let s = sh_squote(source);
    let t = sh_squote(target);
    let ft = sh_squote(fstype);
    let o = sh_squote(options);

    let mut lines = vec!["set -euo pipefail".to_string()];

    match state {
        "mounted" => {
            lines.push(format!("mkdir -p {t}"));
            // Check if already mounted
            lines.push(format!(
                "if ! mountpoint -q {t}; then\n  mount -t {ft} -o {o} {s} {t}\nfi"
            ));
            // Add to fstab if not already there. The whole fstab line is
            // escaped as one shell word, so embedded quotes can't break out;
            // the literal field values still land in /etc/fstab verbatim.
            let fstab_line = sh_squote(&format!("{source} {target} {fstype} {options} 0 0"));
            lines.push(format!(
                "if ! grep -q {t} /etc/fstab 2>/dev/null; then\n  \
                 echo {fstab_line} >> /etc/fstab\nfi"
            ));
        }
        "unmounted" => {
            lines.push(format!("if mountpoint -q {t}; then\n  umount {t}\nfi"));
        }
        "absent" => {
            lines.push(format!("if mountpoint -q {t}; then\n  umount {t}\nfi"));
            // Remove from fstab via sed. The whole `\|PATTERN|d` program is
            // shell-quoted as one word (no break-out), and sed metacharacters
            // in the target are backslash-escaped so they stay literal.
            let sed_pattern = sed_escape(target);
            lines.push(format!(
                "sed -i {} /etc/fstab 2>/dev/null || true",
                sh_squote(&format!("\\|{sed_pattern}|d"))
            ));
        }
        _ => {}
    }

    lines.join("\n")
}

/// Generate shell to query mount state (for hashing).
pub fn state_query_script(resource: &Resource) -> String {
    let target = resource.path.as_deref().unwrap_or("/mnt/unknown");
    let t = sh_squote(target);
    format!(
        "if mountpoint -q {t}; then\n\
           findmnt -n -o SOURCE,FSTYPE,OPTIONS {t} 2>/dev/null\n\
         else\n\
           echo 'UNMOUNTED'\n\
         fi"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{MachineTarget, ResourceType};

    fn mount_resource() -> Resource {
        Resource {
            resource_type: ResourceType::Mount,
            machine: MachineTarget::Single("m1".to_string()),
            path: Some("/mnt/data".to_string()),
            source: Some("nas:/export".to_string()),
            fs_type: Some("nfs".to_string()),
            options: Some("rw,noatime".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn fj154_mount_fields_quoted() {
        let r = mount_resource();
        let script = apply_script(&r);
        assert!(script.contains("mount -t 'nfs' -o 'rw,noatime' 'nas:/export' '/mnt/data'"));
        assert!(script.contains("echo 'nas:/export /mnt/data nfs rw,noatime 0 0' >> /etc/fstab"));
    }

    #[test]
    fn fj154_mount_source_injection_neutralized() {
        let mut r = mount_resource();
        r.source = Some("x';reboot;'".to_string());
        let script = apply_script(&r);
        assert!(script.contains("'x'\\'';reboot;'\\'''"));
        assert!(!script.contains(" 'x';reboot"));
    }

    #[test]
    fn fj154_mount_absent_sed_program_quoted() {
        let mut r = mount_resource();
        r.state = Some("absent".to_string());
        let script = apply_script(&r);
        // sed program is one shell-quoted word; the `/` in the path is
        // sed-escaped so it stays a literal pattern.
        assert!(script.contains("sed -i '\\|\\/mnt\\/data|d' /etc/fstab"));
    }

    #[test]
    fn fj154_mount_absent_quote_in_path_neutralized() {
        let mut r = mount_resource();
        r.state = Some("absent".to_string());
        r.path = Some("/mnt/x';reboot;'".to_string());
        let script = apply_script(&r);
        // The single quote in the path is escaped — no break-out into a
        // standalone `reboot` command.
        assert!(script.contains("'\\''"));
        assert!(!script.contains("sed -i '\\|\\/mnt\\/x';reboot"));
    }

    #[test]
    fn fj154_mount_check_and_query_quoted() {
        let r = mount_resource();
        assert!(check_script(&r).contains("mountpoint -q '/mnt/data'"));
        assert!(state_query_script(&r).contains("mountpoint -q '/mnt/data'"));
    }

    #[test]
    fn fj165_mount_check_label_injection_neutralized() {
        // #165 (#161 sweep gap): a target containing command substitution must
        // not break out of the echo status labels in check_script.
        let mut r = mount_resource();
        r.path = Some("x$(touch /tmp/pwn)".to_string());
        let script = check_script(&r);
        // The `$(` payload stays inside a single-quoted word — no break-out.
        assert!(script.contains("echo 'mounted:x$(touch /tmp/pwn)'"));
        assert!(script.contains("echo 'unmounted:x$(touch /tmp/pwn)'"));
        // No bare command substitution outside quotes.
        assert!(!script.contains("echo mounted:x$(touch"));
        assert!(!script.contains("' $(touch"));
    }
}
