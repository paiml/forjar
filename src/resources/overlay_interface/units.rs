//! FJ-035: systemd unit + shell-fragment emitters for the overlay_interface
//! resource. Split out of `mod.rs` to keep each file under the 500-line
//! file-health limit (CLAUDE.md). Pure string builders; every interpolated
//! value is single-quoted via `sh_squote`.

use super::{OVERLAY_NET, OVERLAY_SCRIPT_PATH};
use crate::core::shell_escape::sh_squote;
use crate::core::types::Resource;

/// The systemd unit file contents (heredoc'd into place by apply).
///
/// CRITICAL: plain `Type=oneshot` — NO `RemainAfterExit=yes` (so the timer's
/// `start`/`restart` re-runs ExecStart and the IP self-heals after a flush).
pub(super) fn service_unit(ip_cidr: &str) -> String {
    format!(
        "[Unit]\n\
         Description=Fleet management overlay IP (DNS/DHCP-independent)\n\
         Documentation=https://github.com/paiml/infra/tree/main/machines/fleet-hosts\n\
         After=network-online.target\n\
         Wants=network-online.target\n\
         \n\
         [Service]\n\
         Type=oneshot\n\
         ExecStart={OVERLAY_SCRIPT_PATH} {ip_cidr}\n\
         \n\
         [Install]\n\
         WantedBy=multi-user.target"
    )
}

/// The systemd timer file contents.
///
/// CRITICAL: `OnBootSec=20s` + `OnCalendar=minutely` (wall-clock), NOT
/// `OnUnitActiveSec`. `Persistent=true`. `WantedBy=timers.target`.
///
/// Anti-thundering-herd (QUORUM MUST-4): `RandomizedDelaySec=30` jitters each
/// host's fire time so a fleet-wide power event does not stampede every host
/// onto the same wall-clock second (and so the DAD probes don't collide).
/// `AccuracySec=1s` keeps the jitter usefully fine-grained (systemd's 1min
/// default accuracy would otherwise coalesce the jitter away). This mirrors the
/// jittered `resyncPeriod` pattern used by Kubernetes informers.
pub(super) fn timer_unit() -> &'static str {
    "[Unit]\n\
     Description=Re-assert fleet management overlay IP (self-heal NIC flaps / boot races / DHCP flush)\n\
     Documentation=https://github.com/paiml/infra/tree/main/machines/fleet-hosts\n\
     \n\
     [Timer]\n\
     OnBootSec=20s\n\
     OnCalendar=minutely\n\
     RandomizedDelaySec=30\n\
     AccuracySec=1s\n\
     Persistent=true\n\
     \n\
     [Install]\n\
     WantedBy=timers.target"
}

/// NetworkManager dispatcher hook contents (instant re-assert).
pub(super) fn dispatcher_hook() -> &'static str {
    "#!/bin/sh\n\
     # fleet-overlay dispatcher (managed by forjar overlay_interface).\n\
     # restart --no-block (NOT start) re-runs ExecStart on the plain oneshot.\n\
     case \"${2:-}\" in\n\
     \x20 up|dhcp4-change|dhcp6-change|reapply)\n\
     \x20\x20\x20 systemctl restart --no-block fleet-overlay.service 2>/dev/null || true\n\
     \x20\x20\x20 ;;\n\
     esac"
}

/// Emit the `/etc/hosts` managed-block rewrite, if `overlay_hosts` is set.
///
/// Idempotent marker-block rewrite; REFUSES to edit on unbalanced markers (a
/// lone BEGIN would make the strip-awk delete to EOF).
pub(super) fn hosts_block(resource: &Resource) -> Option<String> {
    let hosts = resource.overlay_hosts.as_ref()?;
    if hosts.is_empty() {
        return None;
    }
    // Deterministic ordering: sort by IP then name so the block is stable.
    let mut entries: Vec<(&String, &String)> = hosts.iter().collect();
    entries.sort_by(|a, b| a.1.cmp(b.1).then(a.0.cmp(b.0)));
    let mut body = String::new();
    for (name, ip) in entries {
        // Names/IPs are interpolated into /etc/hosts content via single quotes.
        body.push_str(&format!(
            "printf '%s   %s\\n' {} {} >> \"$TMP\"\n",
            sh_squote(ip),
            sh_squote(name)
        ));
    }
    Some(format!(
        "# -- managed /etc/hosts block (overlay_hosts) --\n\
         HOSTS=/etc/hosts\n\
         TMP=\"$(mktemp)\"\n\
         trap 'rm -f \"$TMP\" \"$TMP.2\"' EXIT\n\
         nb=$(grep -c '^# BEGIN forjar-fleet' \"$HOSTS\" 2>/dev/null || echo 0)\n\
         ne=$(grep -c '^# END forjar-fleet' \"$HOSTS\" 2>/dev/null || echo 0)\n\
         if [ \"$nb\" != \"$ne\" ]; then\n\
         \x20 echo \"fleet-hosts: refusing to edit — unbalanced markers ($nb/$ne)\" >&2\n\
         else\n\
         \x20 awk '\n\
         \x20\x20\x20 index($0,\"# BEGIN forjar-fleet\")==1 {{skip=1}}\n\
         \x20\x20\x20 skip!=1 {{print}}\n\
         \x20\x20\x20 index($0,\"# END forjar-fleet\")==1 {{skip=0}}\n\
         \x20 ' \"$HOSTS\" > \"$TMP.2\"\n\
         \x20 awk 'NF{{last=NR}} {{line[NR]=$0}} END{{for(i=1;i<=last;i++) print line[i]}}' \"$TMP.2\" > \"$TMP\"\n\
         \x20 printf '\\n%s\\n' '# BEGIN forjar-fleet (managed by forjar overlay_interface -- do not edit by hand)' >> \"$TMP\"\n\
         {body}\
         \x20 printf '%s\\n' '# END forjar-fleet' >> \"$TMP\"\n\
         \x20 cp \"$HOSTS\" \"$HOSTS.fleet.bak\" 2>/dev/null || true\n\
         \x20 cat \"$TMP\" > \"$HOSTS\"\n\
         \x20 echo 'fleet-hosts: updated /etc/hosts'\n\
         fi"
    ))
}

/// Emit the ufw firewall opening for the overlay subnet, if requested.
pub(super) fn firewall_block(resource: &Resource) -> Option<String> {
    if !resource.overlay_firewall.unwrap_or(false) {
        return None;
    }
    Some(format!(
        "# -- ufw allow overlay subnet --\n\
         if command -v ufw >/dev/null 2>&1 && ufw status 2>/dev/null | grep -qi '^Status: active'; then\n\
         \x20 if ufw status 2>/dev/null | grep -q '{OVERLAY_NET}'; then\n\
         \x20\x20\x20 echo 'fleet-firewall: {OVERLAY_NET} already allowed'\n\
         \x20 else\n\
         \x20\x20\x20 ufw allow from {OVERLAY_NET} comment 'fleet overlay'\n\
         \x20\x20\x20 echo 'fleet-firewall: allowed {OVERLAY_NET}'\n\
         \x20 fi\n\
         else\n\
         \x20 echo 'fleet-firewall: ufw inactive/absent — skipping'\n\
         fi"
    ))
}
