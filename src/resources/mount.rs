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

/// The value a kernel means by leaving `key` out of OPTIONS. tmpfs and vfat omit
/// `uid=0`/`gid=0`, and tmpfs omits `mode=1777`, because those are the defaults.
/// `file_mode`/`dir_mode` have no default here: cifs always echoes both, so a
/// missing one is compared as empty and never matches a declared value.
fn omitted_default(key: &str) -> &'static str {
    match key {
        "uid" | "gid" => "0",
        "mode" => "1777",
        _ => "",
    }
}

/// A POSIX condition that is true when every declared ownership option equals the
/// live mount's value, or `None` when none is declared. A key the kernel does not
/// echo is compared AS ITS DEFAULT, never skipped. Skipping it would call a declared
/// `uid=1000` converged over a mount the kernel reports as uid 0.
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
                "{{ _fj_v=\"{live}\"; [ \"${{_fj_v:-{}}}\" = {} ]; }}",
                omitted_default(k),
                declared_value(k, v)
            )
        })
        .collect();
    (!parts.is_empty()).then(|| parts.join(" && "))
}

/// Shell that prints how many FILESYSTEMS are mounted at `t`, not counting an
/// autofs trigger. `findmnt <path>` lists every mount stacked there, top last.
fn fs_count(t: &str) -> String {
    format!(
        "$(findmnt -n -o FSTYPE {t} 2>/dev/null | awk '$1 != \"autofs\" {{ n++ }} END {{ print n + 0 }}')"
    )
}

/// A mount systemd owns through an automount trigger (`x-systemd.automount`).
/// forjar must not `mount` it itself: `mount.cifs` touching the trigger point
/// fires the automount, and systemd mounts from its OWN unit underneath (#648).
fn is_automount(options: &str) -> bool {
    options.split(',').any(|o| o == "x-systemd.automount")
}

/// Detach EVERY filesystem stacked at `t`, top first, with a plain `umount`.
/// Busy is a loud failure; `lazy` keeps the pre-#642 fallback for a wrong source.
/// Bounded, because an automount that keeps re-mounting must not spin forever.
fn detach_stack(t: &str, lazy: bool) -> String {
    let n = fs_count(t);
    let busy = if lazy {
        format!("umount -l {t} 2>/dev/null || true")
    } else {
        format!(
            "{{ echo \"forjar: mount \"{t}\" options drift, but it is busy; not detaching a path in use (#642)\" >&2; exit 1; }}"
        )
    };
    format!(
        "_fj_i=0\n  \
         while [ \"{n}\" -gt 0 ]; do\n    \
         _fj_i=$((_fj_i + 1)); [ \"$_fj_i\" -le 8 ] || {{ echo \"forjar: \"{t}\" re-mounts as fast as it is detached (#648)\" >&2; exit 1; }}\n    \
         umount {t} 2>/dev/null || {busy}\n  \
         done"
    )
}

/// Detach whatever is at `t` and mount the declared filesystem. An automount
/// entry is mounted BY systemd (#648): reload its unit from the fstab already
/// written BEFORE detaching, so a re-trigger mid-detach mounts the declared
/// options, then touch the path instead of racing systemd with our own `mount`.
fn remount(t: &str, ft: &str, o: &str, s: &str, automount: bool, lazy: bool) -> String {
    let detach = detach_stack(t, lazy);
    if automount {
        format!("systemctl daemon-reload\n  {detach}\n  ls {t} >/dev/null")
    } else {
        format!("{detach}\n  mount -t {ft} -o {o} {s} {t}")
    }
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
    // #648: `tail -1` above reads the TOP mount only. A second filesystem stacked
    // under it (a stale-owner mount systemd put there) is drift, not convergence.
    if resource.source.is_some() {
        condition = format!("{condition} && [ \"{}\" -le 1 ]", fs_count(&t));
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

            // Rewrite the fstab line whenever it differs from the declared one.
            // FIRST (#648): an automount entry is re-mounted by systemd from the
            // unit it generates out of fstab, so fstab must be declared before
            // anything is detached, or systemd re-mounts the OLD options.
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

            let automount = is_automount(options);

            // Remount when the mounted source differs from the declared one.
            // `findmnt` answers what is ACTUALLY mounted; `mountpoint -q` only
            // answers whether something is. The lazy fallback is for a plain
            // mount only: under an automount, `umount -l` on a busy stack would
            // detach live writers and let systemd re-mount underneath (#648).
            lines.push(format!(
                "_fj_cur=$(findmnt -n -o SOURCE {t} 2>/dev/null | tail -1 || true)\n\
                 if [ \"$_fj_cur\" != {s} ]; then\n  \
                 {}\n\
                 fi",
                remount(&t, &ft, &o, &s, automount, !automount)
            ));

            // #642: the right share mounted with the wrong owner or modes, and
            // #648: a stale mount stacked under the right one. Remount, with a
            // plain `umount` and NO lazy fallback: `umount -l` would detach a
            // mount that live jobs are writing into. Busy is a loud failure that
            // names the path, never a silent convergence.
            let stacked = format!("[ \"{}\" -gt 1 ]", fs_count(&t));
            let drift = match options_condition(target, resource.options.as_deref()) {
                Some(opts) => format!("! {{ {opts}; }} || {stacked}"),
                None => stacked,
            };
            lines.push(format!(
                "if [ \"$(findmnt -n -o SOURCE {t} 2>/dev/null | tail -1 || true)\" = {s} ] && {{ {drift}; }}; then\n  \
                 {}\n\
                 fi",
                remount(&t, &ft, &o, &s, automount, false)
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
