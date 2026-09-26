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

/// The ownership and permission options a kernel echoes back in the live mount's
/// OPTIONS (cifs, tmpfs, vfat, ntfs3). These are the ones whose drift changes who
/// may write, and #642 is exactly that: a declared `gid=` that never reached the
/// mount while apply reported converged.
const OWNERSHIP_KEYS: [&str; 5] = ["uid", "gid", "file_mode", "dir_mode", "mode"];

/// Shell that prints `value` in a form comparable with the kernel's: a name for
/// `uid=`/`gid=` becomes its number (cifs echoes `uid=1000` for `uid=noah`), and a
/// mode loses its leading zeros (tmpfs echoes `mode=755` for `mode=0755`).
fn declared_value(key: &str, value: &str) -> String {
    let numeric = !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit());
    let v = sh_squote(value);
    match key {
        "uid" | "gid" if numeric => v,
        "uid" => format!("\"$(id -u {v} 2>/dev/null)\""),
        "gid" => format!("\"$(getent group {v} 2>/dev/null | cut -d: -f3)\""),
        _ => format!("\"$(printf '%s' {v} | sed 's/^0*\\(.\\)/\\1/')\""),
    }
}

/// A POSIX condition that is true when every declared ownership option equals the
/// live mount's value, or `None` when none is declared. A key the kernel does not
/// echo is NOT compared: tmpfs and vfat omit `uid=0`/`gid=0` and tmpfs omits
/// `mode=1777` because they are the defaults, so absence is the default, not drift.
fn options_condition(target: &str, options: Option<&str>) -> Option<String> {
    let t = sh_squote(target);
    let parts: Vec<String> = options?
        .split(',')
        .filter_map(|o| o.split_once('='))
        .filter(|(k, _)| OWNERSHIP_KEYS.contains(k))
        .map(|(k, v)| {
            let live = format!(
                "$(findmnt -n -o OPTIONS {t} 2>/dev/null | tail -1 | tr ',' '\\n' | sed -n 's/^{k}=//p' | sed 's/^0*\\(.\\)/\\1/')"
            );
            format!(
                "{{ _fj_v=\"{live}\"; [ -z \"$_fj_v\" ] || [ \"$_fj_v\" = {} ]; }}",
                declared_value(k, v)
            )
        })
        .collect();
    (!parts.is_empty()).then(|| parts.join(" && "))
}

/// Generate shell to check mount state.
pub fn check_script(resource: &Resource) -> String {
    let target = resource.path.as_deref().unwrap_or("/mnt/unknown");
    let t = sh_squote(target);
    let source = resource.source.as_deref().unwrap_or("none");
    let s = sh_squote(source);
    // `mountpoint -q` answers "is ANYTHING mounted here", which is not the
    // question the resource asks. A cifs mount of the wrong share satisfies it
    // exactly as well as the right one, so `check` reported converged on two
    // hosts still mounted to a share the config had stopped declaring
    // (paiml/infra, 2026-08-19).
    //
    // Compare the MOUNTED source against the DECLARED one. `state: mounted`
    // with no `source:` keeps the old semantics — there is nothing to compare.
    let mut condition = if resource.source.is_some() {
        format!("[ \"$(findmnt -n -o SOURCE {t} 2>/dev/null | tail -1)\" = {s} ]")
    } else {
        format!("mountpoint -q {t} 2>/dev/null")
    };
    // #642: the right share with the wrong owner is not the declared mount either.
    if let Some(opts) = options_condition(target, resource.options.as_deref()) {
        condition = format!("{condition} && {opts}");
    }
    // The status labels embed the config-derived `target`, so route them
    // through sh_squote too — a raw label could close the single quote and
    // run command substitution (matches docker.rs/package.rs).
    verdict::single(
        &condition,
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
            // APPLY MUST BE CORRECTIVE, NOT MERELY CREATIVE.
            //
            // Both guards here used to test the TARGET PATH and never the
            // source: `mountpoint -q <target>` for the mount, and
            // `grep -q <target> /etc/fstab` for the declaration. On a bare host
            // that works. On a host that is PRESENT BUT WRONG, both
            // short-circuit, apply exits 0, and forjar reports converged over a
            // host it never touched.
            //
            // Measured on paiml/infra 2026-08-19: `source` changed from
            // //192.168.1.179/Personal-Drive to //192.168.1.179/media, applied
            // to intel and lambda-labs, both reported `1 converged` — and both
            // kept the old share mounted AND the old fstab line. The fstab half
            // is the worse one: it is written once at first apply and never
            // corrected, so every later change to source/fs_type/options is
            // discarded permanently while forjar keeps reporting success.
            //
            // So: compare against the DECLARED state, not the path's existence.
            lines.push(format!("mkdir -p {t}"));

            // Remount when the mounted source differs from the declared one.
            // `findmnt` answers what is ACTUALLY mounted; `mountpoint -q` only
            // answers whether something is.
            lines.push(format!(
                "_fj_cur=$(findmnt -n -o SOURCE {t} 2>/dev/null | tail -1 || true)\n\
                 if [ \"$_fj_cur\" != {s} ]; then\n  \
                 if mountpoint -q {t}; then umount {t} 2>/dev/null || umount -l {t} 2>/dev/null || true; fi\n  \
                 mount -t {ft} -o {o} {s} {t}\n\
                 fi"
            ));

            // #642: the right share mounted with the wrong owner or modes. Remount,
            // but with a plain `umount` and NO lazy fallback: `umount -l` would
            // detach a mount that live jobs are writing into. Busy is a loud
            // failure that names the path, never a silent convergence.
            if let Some(opts) = options_condition(target, resource.options.as_deref()) {
                lines.push(format!(
                    "if [ \"$(findmnt -n -o SOURCE {t} 2>/dev/null | tail -1 || true)\" = {s} ] && ! {{ {opts}; }}; then\n  \
                     umount {t} || {{ echo \"forjar: mount \"{t}\" options drift, but it is busy; not detaching a path in use (#642)\" >&2; exit 1; }}\n  \
                     mount -t {ft} -o {o} {s} {t}\n\
                     fi"
                ));
            }

            // Rewrite the fstab line whenever it differs from the declared one.
            // Matching on the MOUNTPOINT FIELD (second whitespace-separated
            // column) rather than a substring: a bare `grep <target>` also hits
            // /mnt/unas-backup and any comment mentioning the path.
            let fstab_line = format!("{source} {target} {fstype} {options} 0 0");
            let q_line = sh_squote(&fstab_line);
            // The target must be a STRING literal in awk, not a bare word:
            // `$2 != /mnt/unas` makes awk parse /mnt/unas/ as a REGEX and
            // compare $2 against the match result, silently dropping every
            // line. Quote it.
            let awk_drop = sh_squote(&format!("$2 != \"{target}\""));
            lines.push(format!(
                "if ! grep -qxF {q_line} /etc/fstab 2>/dev/null; then\n  \
                 _fj_tmp=$(mktemp)\n  \
                 awk {awk_drop} /etc/fstab > \"$_fj_tmp\" 2>/dev/null || true\n  \
                 printf '%s\\n' {q_line} >> \"$_fj_tmp\"\n  \
                 cat \"$_fj_tmp\" > /etc/fstab\n  \
                 rm -f \"$_fj_tmp\"\n\
                 fi"
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
        // Asserts the declared fstab LINE is emitted, not the command emitting
        // it. `echo ... >> /etc/fstab` was only correct on a bare host.
        assert!(script.contains("'nas:/export /mnt/data nfs rw,noatime 0 0'"));
    }

    #[test]
    fn fj154_mount_source_injection_neutralized() {
        let mut r = mount_resource();
        r.source = Some("x';reboot;'".to_string());
        let script = apply_script(&r);
        assert!(script.contains("'x'\"'\"';reboot;'\"'\"''"));
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
        assert!(script.contains("'\"'\"'"));
        assert!(!script.contains("sed -i '\\|\\/mnt\\/x';reboot"));
    }

    #[test]
    fn fj154_mount_check_and_query_quoted() {
        let r = mount_resource();
        // This test pins SHELL QUOTING of config-derived values, so it asserts
        // the quoted forms appear — not which command consumes them. It used to
        // pin `mountpoint -q '/mnt/data'` and failed when the check was
        // corrected to compare the mounted SOURCE against the declared one.
        let c = check_script(&r);
        assert!(c.contains("'/mnt/data'"), "target must be quoted: {c}");
        assert!(c.contains("'nas:/export'"), "source must be quoted: {c}");
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
