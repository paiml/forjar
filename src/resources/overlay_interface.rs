//! FJ-035: Fleet overlay-interface resource handler.
//!
//! Provides a DNS/DHCP-independent fleet management overlay *natively* in
//! forjar, replacing the `machines/fleet-hosts` shell installer.
//!
//! # Problem
//!
//! A fleet with no DNS it controls (e.g. the ASUS ET12 in AP mode, with the
//! ISP H3600P as the real DHCP authority) re-leases every host onto a random
//! `192.168.1.x` on every power outage, breaking `ssh intel` / `ssh mini`.
//! Pinning `/etc/hosts` or DHCP reservations does not survive a reboot because
//! the IP itself isn't owned by anything we control.
//!
//! # Fix
//!
//! Each host binds a **static secondary IP** on a private flat L2 overlay
//! `10.42.0.0/24` (no gateway; host-to-host via ARP) on its OWN default-route
//! NIC. Because the IP is owned by the NIC config, it survives reboots,
//! outages, DHCP churn, and subnet flips. `/etc/hosts` maps names -> overlay IPs
//! (no DNS), and ufw is opened for the subnet.
//!
//! # Self-heal (defense in depth)
//!
//! A DHCP renewal or NIC flap flushes the secondary `ip addr`, so it must be
//! re-asserted:
//!   1. `fleet-overlay.service` (plain `Type=oneshot`, NO `RemainAfterExit`) —
//!      idempotently (re-)adds the IP to the default-route NIC.
//!   2. `fleet-overlay.timer` — `OnBootSec=20s` + `OnCalendar=minutely`; the
//!      sole re-assert path on systemd-networkd hosts (closes the gap to <=60s).
//!   3. NetworkManager dispatcher `/etc/NetworkManager/dispatcher.d/50-fleet-overlay`
//!      — instant (~0s) re-assert on up/dhcp4-change/dhcp6-change/reapply,
//!      installed ONLY where NM owns the NIC (dispatcher dir present).
//!
//! Critical anti-regressions (all just fixed live, do NOT reintroduce):
//!   * The service MUST NOT set `RemainAfterExit=yes` — that makes the timer's
//!     `start` a no-op so the IP never self-heals after a flush.
//!   * The timer MUST use `OnCalendar=minutely` (wall-clock), NOT
//!     `OnUnitActiveSec` — the latter fired only once on the old
//!     RemainAfterExit unit.
//!   * On a unit-content change, apply MUST `daemon-reload` + **restart**
//!     (service and timer), never `start`/`enable --now` alone.
//!
//! # YAML example
//!
//! ```yaml
//! fleet-overlay:
//!   type: overlay_interface
//!   machine: intel
//!   sudo: true
//!   overlay_ip: "10.42.0.11/24"
//!   # interface omitted -> auto-detect default-route NIC
//!   overlay_firewall: true
//!   overlay_hosts:
//!     lambda-labs: "10.42.0.10"
//!     intel: "10.42.0.11"
//!     mini: "10.42.0.12"
//! ```

use crate::core::shell_escape::{is_valid_iface, is_valid_overlay_ip, sh_squote};
use crate::core::types::Resource;

const OVERLAY_NET: &str = "10.42.0.0/24";
const SERVICE_PATH: &str = "/etc/systemd/system/fleet-overlay.service";
const TIMER_PATH: &str = "/etc/systemd/system/fleet-overlay.timer";
const OVERLAY_SCRIPT_PATH: &str = "/usr/local/sbin/fleet-overlay.sh";
const DISPATCHER_PATH: &str = "/etc/NetworkManager/dispatcher.d/50-fleet-overlay";

/// Error script emitted when `overlay_ip` is missing or not a valid IPv4/CIDR.
fn reject_bad_ip(ip: &str) -> String {
    format!(
        "echo {} >&2; exit 1",
        sh_squote(&format!(
            "ERROR: overlay_interface requires a valid overlay_ip (IPv4/CIDR), got: {ip}"
        ))
    )
}

/// Default overlay IP placeholder used only for fallback messaging.
fn overlay_ip(resource: &Resource) -> Option<&str> {
    resource.overlay_ip.as_deref()
}

/// Generate shell to check whether the overlay IP and its units are present.
pub fn check_script(resource: &Resource) -> String {
    let ip_cidr = match overlay_ip(resource) {
        Some(v) if is_valid_overlay_ip(v) => v,
        other => return reject_bad_ip(other.unwrap_or("<none>")),
    };
    let ip = ip_cidr.split('/').next().unwrap_or(ip_cidr);
    let ip_q = sh_squote(ip);

    format!(
        "set -u\n\
         if ip -4 -o addr show 2>/dev/null | grep -qw {ip_q}; then\n\
         \x20 echo 'ip:present:{ip}'\n\
         else\n\
         \x20 echo 'ip:absent:{ip}'\n\
         fi\n\
         if [ -f {SERVICE_PATH} ]; then echo 'service:present'; else echo 'service:absent'; fi\n\
         if [ -f {TIMER_PATH} ]; then echo 'timer:present'; else echo 'timer:absent'; fi\n\
         if systemctl is-active --quiet fleet-overlay.timer 2>/dev/null; then echo 'timer:active'; else echo 'timer:inactive'; fi"
    )
}

/// Generate the idempotent `fleet-overlay.sh` body (the ExecStart target).
///
/// Picks the default-route NIC (or an explicit `interface`), waits up to 60s
/// for a late NIC, moves the IP if it landed on the wrong iface, then
/// `ip addr add ... 2>/dev/null || true` (idempotent).
fn overlay_sh_body(resource: &Resource) -> Result<String, String> {
    // An explicit interface, if given, must be a safe iface name.
    let pick = match resource.overlay_iface.as_deref() {
        Some(iface) if is_valid_iface(iface) => {
            format!("IFACE={}", sh_squote(iface))
        }
        Some(iface) => {
            return Err(format!(
                "echo {} >&2; exit 1",
                sh_squote(&format!(
                    "ERROR: overlay_interface invalid interface name: {iface}"
                ))
            ));
        }
        None => {
            // Auto-detect: default-route NIC, else first real UP iface.
            "pick_iface() {\n\
             \x20 i=\"$(ip route show default 2>/dev/null | awk '{print $5; exit}')\"\n\
             \x20 [ -n \"$i\" ] && { printf '%s' \"$i\"; return; }\n\
             \x20 ip -o link show up 2>/dev/null | awk -F': ' '{print $2}' \\\n\
             \x20\x20\x20 | grep -vE '^(lo|docker|br-|veth|virbr|tun|tap|wg|tailscale)' | head -1\n\
             }\n\
             IFACE=\"\"\n\
             n=0\n\
             while [ \"$n\" -lt 60 ]; do\n\
             \x20 IFACE=\"$(pick_iface)\"\n\
             \x20 [ -n \"$IFACE\" ] && break\n\
             \x20 n=$((n + 1))\n\
             \x20 sleep 1\n\
             done"
                .to_string()
        }
    };

    Ok(format!(
        "#!/bin/sh\n\
         # fleet-overlay.sh (managed by forjar overlay_interface) — bind the stable\n\
         # fleet overlay IP as a SECONDARY address on the default-route NIC.\n\
         set -eu\n\
         OVERLAY=\"$1\"\n\
         IP=\"${{OVERLAY%/*}}\"\n\
         {pick}\n\
         [ -n \"$IFACE\" ] || {{ echo 'fleet-overlay: no LAN interface found (yet)' >&2; exit 1; }}\n\
         CUR_IF=\"$(ip -4 -o addr show 2>/dev/null \\\n\
         \x20 | awk -v ip=\"$IP\" '{{split($4,a,\"/\"); if (a[1]==ip) print $2}}' | head -1)\"\n\
         if [ -n \"$CUR_IF\" ] && [ \"$CUR_IF\" != \"$IFACE\" ]; then\n\
         \x20 ip addr del \"$OVERLAY\" dev \"$CUR_IF\" 2>/dev/null || true\n\
         \x20 CUR_IF=\"\"\n\
         fi\n\
         if [ \"$CUR_IF\" = \"$IFACE\" ]; then\n\
         \x20 echo \"fleet-overlay: $OVERLAY already on $IFACE\"\n\
         else\n\
         \x20 ip addr add \"$OVERLAY\" dev \"$IFACE\" 2>/dev/null || true\n\
         \x20 echo \"fleet-overlay: ensured $OVERLAY on $IFACE\"\n\
         fi"
    ))
}

/// The systemd unit file contents (heredoc'd into place by apply).
///
/// CRITICAL: plain `Type=oneshot` — NO `RemainAfterExit=yes` (so the timer's
/// `start`/`restart` re-runs ExecStart and the IP self-heals after a flush).
fn service_unit(ip_cidr: &str) -> String {
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
fn timer_unit() -> &'static str {
    "[Unit]\n\
     Description=Re-assert fleet management overlay IP (self-heal NIC flaps / boot races / DHCP flush)\n\
     Documentation=https://github.com/paiml/infra/tree/main/machines/fleet-hosts\n\
     \n\
     [Timer]\n\
     OnBootSec=20s\n\
     OnCalendar=minutely\n\
     Persistent=true\n\
     \n\
     [Install]\n\
     WantedBy=timers.target"
}

/// NetworkManager dispatcher hook contents (instant re-assert).
fn dispatcher_hook() -> &'static str {
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
fn hosts_block(resource: &Resource) -> Option<String> {
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
fn firewall_block(resource: &Resource) -> Option<String> {
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

/// Generate the apply script: converge the overlay IP + self-heal units.
pub fn apply_script(resource: &Resource) -> String {
    let ip_cidr = match overlay_ip(resource) {
        Some(v) if is_valid_overlay_ip(v) => v,
        other => return reject_bad_ip(other.unwrap_or("<none>")),
    };
    let state = resource.state.as_deref().unwrap_or("present");

    if state == "absent" {
        return format!(
            "set -euo pipefail\n\
             # Tear down units, then drop the overlay IP.\n\
             systemctl disable --now fleet-overlay.timer 2>/dev/null || true\n\
             systemctl disable --now fleet-overlay.service 2>/dev/null || true\n\
             rm -f {SERVICE_PATH} {TIMER_PATH} {OVERLAY_SCRIPT_PATH} {DISPATCHER_PATH}\n\
             systemctl daemon-reload 2>/dev/null || true\n\
             IP={ip}\n\
             CUR_IF=\"$(ip -4 -o addr show 2>/dev/null \\\n\
             \x20 | awk -v ip=\"$IP\" '{{split($4,a,\"/\"); if (a[1]==ip) print $2}}' | head -1)\"\n\
             if [ -n \"$CUR_IF\" ]; then\n\
             \x20 ip addr del {ip_cidr} dev \"$CUR_IF\" 2>/dev/null || true\n\
             fi\n\
             echo 'overlay_interface: removed:{ip_cidr}'",
            ip = sh_squote(ip_cidr.split('/').next().unwrap_or(ip_cidr)),
            ip_cidr = ip_cidr,
        );
    }

    let sh_body = match overlay_sh_body(resource) {
        Ok(b) => b,
        Err(e) => return e,
    };
    let service = service_unit(ip_cidr);
    let timer = timer_unit();
    let dispatcher = dispatcher_hook();
    let hosts = hosts_block(resource);
    let firewall = firewall_block(resource);

    let mut script = String::new();
    script.push_str("set -euo pipefail\n");
    // Systemd guard: this resource is systemd-centric; skip cleanly without it.
    script.push_str(
        "if ! command -v systemctl >/dev/null 2>&1; then\n\
         \x20 echo 'FORJAR_WARN: systemctl not found - skipping overlay_interface (no systemd)'\n\
         \x20 exit 0\n\
         fi\n",
    );

    // 1. Install the idempotent overlay script.
    script.push_str(&format!(
        "mkdir -p /usr/local/sbin\n\
         cat > {OVERLAY_SCRIPT_PATH} <<'FORJAR_OVERLAY_SH'\n\
         {sh_body}\n\
         FORJAR_OVERLAY_SH\n\
         chmod 0755 {OVERLAY_SCRIPT_PATH}\n"
    ));

    // 2. Install the service unit (plain oneshot).
    script.push_str(&format!(
        "cat > {SERVICE_PATH} <<'FORJAR_OVERLAY_SERVICE'\n\
         {service}\n\
         FORJAR_OVERLAY_SERVICE\n\
         chmod 0644 {SERVICE_PATH}\n"
    ));

    // 3. Install the timer unit (OnCalendar=minutely).
    script.push_str(&format!(
        "cat > {TIMER_PATH} <<'FORJAR_OVERLAY_TIMER'\n\
         {timer}\n\
         FORJAR_OVERLAY_TIMER\n\
         chmod 0644 {TIMER_PATH}\n"
    ));

    // 4. NM dispatcher hook — ONLY where NM owns the NIC (dispatcher dir present).
    script.push_str(&format!(
        "if [ -d /etc/NetworkManager/dispatcher.d ]; then\n\
         \x20 cat > {DISPATCHER_PATH} <<'FORJAR_OVERLAY_DISP'\n\
         {dispatcher}\n\
         FORJAR_OVERLAY_DISP\n\
         \x20 chmod 0755 {DISPATCHER_PATH}\n\
         \x20 echo 'overlay_interface: NetworkManager dispatcher hook installed'\n\
         fi\n"
    ));

    // 5. Reload + RESTART (NOT start/enable --now) so unit-content changes take
    //    effect without a reboot. enable+restart for both service and timer.
    script.push_str(
        "systemctl daemon-reload\n\
         systemctl enable fleet-overlay.service\n\
         systemctl restart fleet-overlay.service\n\
         systemctl enable fleet-overlay.timer\n\
         systemctl restart fleet-overlay.timer\n",
    );

    if let Some(h) = hosts {
        script.push_str(&h);
        script.push('\n');
    }
    if let Some(f) = firewall {
        script.push_str(&f);
        script.push('\n');
    }

    script.push_str(&format!("echo 'overlay_interface: ensured {ip_cidr}'"));
    script
}

/// Generate the state-query script (BLAKE3-hashed for drift detection).
///
/// Echoes: whether the overlay IP is bound, the unit-file sha (so a stale unit
/// surfaces as drift), and whether the timer is active.
pub fn state_query_script(resource: &Resource) -> String {
    let ip_cidr = match overlay_ip(resource) {
        Some(v) if is_valid_overlay_ip(v) => v,
        other => return reject_bad_ip(other.unwrap_or("<none>")),
    };
    let ip = ip_cidr.split('/').next().unwrap_or(ip_cidr);
    let ip_q = sh_squote(ip);

    format!(
        "set -u\n\
         if ip -4 -o addr show 2>/dev/null | grep -qw {ip_q}; then\n\
         \x20 echo 'overlay_ip=present:{ip}'\n\
         else\n\
         \x20 echo 'overlay_ip=absent:{ip}'\n\
         fi\n\
         SVC_SHA=$( (sha256sum {SERVICE_PATH} 2>/dev/null || echo missing) | awk '{{print $1}}')\n\
         TMR_SHA=$( (sha256sum {TIMER_PATH} 2>/dev/null || echo missing) | awk '{{print $1}}')\n\
         echo \"overlay_service_sha=$SVC_SHA\"\n\
         echo \"overlay_timer_sha=$TMR_SHA\"\n\
         echo \"overlay_timer_active=$(systemctl is-active fleet-overlay.timer 2>/dev/null || echo unknown)\""
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{MachineTarget, Resource, ResourceType};
    use std::collections::HashMap;

    fn make_overlay(ip: &str) -> Resource {
        Resource {
            resource_type: ResourceType::OverlayInterface,
            machine: MachineTarget::Single("intel".to_string()),
            overlay_ip: Some(ip.to_string()),
            sudo: true,
            ..Default::default()
        }
    }

    #[test]
    fn check_reports_ip_and_units() {
        let r = make_overlay("10.42.0.11/24");
        let s = check_script(&r);
        assert!(s.contains("ip:present:10.42.0.11"));
        assert!(s.contains("ip:absent:10.42.0.11"));
        assert!(s.contains("/etc/systemd/system/fleet-overlay.service"));
        assert!(s.contains("/etc/systemd/system/fleet-overlay.timer"));
    }

    #[test]
    fn apply_installs_plain_oneshot_no_remain_after_exit() {
        let r = make_overlay("10.42.0.11/24");
        let s = apply_script(&r);
        assert!(s.contains("Type=oneshot"), "{s}");
        // CRITICAL anti-regression: must NOT set RemainAfterExit.
        assert!(
            !s.contains("RemainAfterExit"),
            "service must be a PLAIN oneshot (no RemainAfterExit): {s}"
        );
    }

    #[test]
    fn apply_timer_uses_oncalendar_minutely_not_onunitactivesec() {
        let r = make_overlay("10.42.0.15/24");
        let s = apply_script(&r);
        assert!(s.contains("OnCalendar=minutely"), "{s}");
        assert!(s.contains("OnBootSec=20s"), "{s}");
        assert!(
            !s.contains("OnUnitActiveSec"),
            "timer must NOT use OnUnitActiveSec: {s}"
        );
        assert!(s.contains("Persistent=true"));
    }

    #[test]
    fn apply_restarts_not_just_starts() {
        let r = make_overlay("10.42.0.11/24");
        let s = apply_script(&r);
        // Upgrade safety: restart so unit-content changes take effect.
        assert!(s.contains("systemctl restart fleet-overlay.service"), "{s}");
        assert!(s.contains("systemctl restart fleet-overlay.timer"), "{s}");
        assert!(s.contains("systemctl daemon-reload"), "{s}");
    }

    #[test]
    fn apply_installs_overlay_script_and_binds_ip() {
        let r = make_overlay("10.42.0.11/24");
        let s = apply_script(&r);
        assert!(s.contains("/usr/local/sbin/fleet-overlay.sh"));
        assert!(s.contains("ExecStart=/usr/local/sbin/fleet-overlay.sh 10.42.0.11/24"));
        assert!(s.contains("ip addr add"));
        assert!(s.contains("set -euo pipefail"));
    }

    #[test]
    fn apply_autodetect_default_route_nic() {
        let r = make_overlay("10.42.0.11/24");
        let s = apply_script(&r);
        assert!(s.contains("ip route show default"), "{s}");
        // Skips virtual ifaces.
        assert!(s.contains("docker|br-|veth"), "{s}");
    }

    #[test]
    fn apply_explicit_interface_used() {
        let mut r = make_overlay("10.42.0.13/24");
        r.overlay_iface = Some("enp9s0".to_string());
        let s = apply_script(&r);
        assert!(s.contains("IFACE='enp9s0'"), "{s}");
        // No autodetect when iface is explicit.
        assert!(!s.contains("ip route show default"), "{s}");
    }

    #[test]
    fn apply_nm_dispatcher_conditional_on_dir() {
        let r = make_overlay("10.42.0.10/24");
        let s = apply_script(&r);
        assert!(
            s.contains("if [ -d /etc/NetworkManager/dispatcher.d ]"),
            "{s}"
        );
        assert!(s.contains("50-fleet-overlay"));
        assert!(
            s.contains("restart --no-block fleet-overlay.service"),
            "{s}"
        );
    }

    #[test]
    fn apply_absent_tears_down() {
        let mut r = make_overlay("10.42.0.11/24");
        r.state = Some("absent".to_string());
        let s = apply_script(&r);
        assert!(s.contains("disable --now fleet-overlay.timer"), "{s}");
        assert!(
            s.contains("rm -f /etc/systemd/system/fleet-overlay.service"),
            "{s}"
        );
        assert!(s.contains("ip addr del 10.42.0.11/24"), "{s}");
        assert!(s.contains("removed:10.42.0.11/24"), "{s}");
    }

    #[test]
    fn apply_optional_firewall() {
        let mut r = make_overlay("10.42.0.11/24");
        r.overlay_firewall = Some(true);
        let s = apply_script(&r);
        assert!(s.contains("ufw allow from 10.42.0.0/24"), "{s}");
    }

    #[test]
    fn apply_firewall_omitted_by_default() {
        let r = make_overlay("10.42.0.11/24");
        let s = apply_script(&r);
        assert!(!s.contains("ufw allow"), "{s}");
    }

    #[test]
    fn apply_optional_hosts_block() {
        let mut r = make_overlay("10.42.0.11/24");
        let mut h = HashMap::new();
        h.insert("intel".to_string(), "10.42.0.11".to_string());
        h.insert("mini".to_string(), "10.42.0.12".to_string());
        r.overlay_hosts = Some(h);
        let s = apply_script(&r);
        assert!(s.contains("# BEGIN forjar-fleet"), "{s}");
        assert!(s.contains("'intel'"), "{s}");
        assert!(s.contains("'10.42.0.12'"), "{s}");
        // Unbalanced-marker safety guard.
        assert!(s.contains("refusing to edit"), "{s}");
    }

    #[test]
    fn state_query_reports_ip_and_unit_shas() {
        let r = make_overlay("10.42.0.11/24");
        let s = state_query_script(&r);
        assert!(s.contains("overlay_ip=present:10.42.0.11"));
        assert!(s.contains("overlay_ip=absent:10.42.0.11"));
        assert!(s.contains("overlay_service_sha="));
        assert!(s.contains("overlay_timer_sha="));
        assert!(s.contains("overlay_timer_active="));
    }

    #[test]
    fn invalid_ip_rejected_all_phases() {
        let r = make_overlay("10.42.0.11");
        assert!(check_script(&r).contains("ERROR: overlay_interface requires a valid overlay_ip"));
        assert!(apply_script(&r).contains("ERROR: overlay_interface requires a valid overlay_ip"));
        assert!(
            state_query_script(&r).contains("ERROR: overlay_interface requires a valid overlay_ip")
        );
    }

    #[test]
    fn injection_in_ip_rejected() {
        let r = make_overlay("10.42.0.11/24;reboot");
        let s = apply_script(&r);
        assert!(
            s.contains("ERROR: overlay_interface requires a valid overlay_ip"),
            "{s}"
        );
        assert!(!s.contains("reboot\n"), "{s}");
    }

    #[test]
    fn injection_in_interface_rejected() {
        let mut r = make_overlay("10.42.0.13/24");
        r.overlay_iface = Some("eth0;reboot".to_string());
        let s = apply_script(&r);
        assert!(
            s.contains("ERROR: overlay_interface invalid interface name"),
            "{s}"
        );
    }

    #[test]
    fn missing_ip_rejected() {
        let mut r = make_overlay("10.42.0.11/24");
        r.overlay_ip = None;
        assert!(apply_script(&r).contains("ERROR: overlay_interface requires a valid overlay_ip"));
    }
}
