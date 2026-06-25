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

mod units;
use units::{dispatcher_hook, firewall_block, hosts_block, service_unit, timer_unit};

#[cfg(test)]
mod tests;

pub(super) const OVERLAY_NET: &str = "10.42.0.0/24";
const SERVICE_PATH: &str = "/etc/systemd/system/fleet-overlay.service";
const TIMER_PATH: &str = "/etc/systemd/system/fleet-overlay.timer";
pub(super) const OVERLAY_SCRIPT_PATH: &str = "/usr/local/sbin/fleet-overlay.sh";
const DISPATCHER_PATH: &str = "/etc/NetworkManager/dispatcher.d/50-fleet-overlay";
/// Volatile runtime dir (tmpfs) holding the convergence heartbeat + repair counter.
const STATUS_DIR: &str = "/run/fleet-overlay";
const STATUS_JSON: &str = "/run/fleet-overlay/status.json";
const REPAIR_COUNT_FILE: &str = "/run/fleet-overlay/repair_count";
/// Freshness window (seconds): a heartbeat older than this is reported `stale`
/// in the drift-visible state query. Must exceed the timer period (60s) with
/// margin so a HEALTHY host never flaps to `stale` — but a wedged ensure loop
/// (never-firing timer / hung pick_iface) crosses it and surfaces as drift.
const HEARTBEAT_FRESH_SECS: u64 = 300;

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
         #\n\
         # Defense in depth:\n\
         #  * RFC 5227 duplicate-address detection (arping -D) BEFORE binding a\n\
         #    not-yet-present IP — refuse + fail (nonzero exit) on a real conflict\n\
         #    so two hosts racing for one overlay IP do not flap forever.\n\
         #  * {STATUS_JSON} heartbeat every run (last_converged / action_taken /\n\
         #    repair_count / ip_present_before|after) so a wedged ensure loop is\n\
         #    OBSERVABLE, not invisible-until-ssh-breaks.\n\
         #  * nonzero exit when the IP is still absent after the run => the\n\
         #    oneshot enters `failed` and the operator/systemd sees it.\n\
         set -u\n\
         OVERLAY=\"$1\"\n\
         IP=\"${{OVERLAY%/*}}\"\n\
         ACTION=noop\n\
         STATUS_DIR={STATUS_DIR}\n\
         STATUS_JSON={STATUS_JSON}\n\
         COUNT_FILE={REPAIR_COUNT_FILE}\n\
         mkdir -p \"$STATUS_DIR\" 2>/dev/null || true\n\
         # repair_count: monotonic, read+increment ONLY when we actually repair.\n\
         REPAIRS=\"$(cat \"$COUNT_FILE\" 2>/dev/null || echo 0)\"\n\
         case \"$REPAIRS\" in ''|*[!0-9]*) REPAIRS=0 ;; esac\n\
         # Exact-address presence: split the addr/cidr column on '/' and compare\n\
         # the bare address (so 10.42.0.11 does NOT match 10.42.0.110).\n\
         ip_has() {{ ip -4 -o addr show 2>/dev/null \\\n\
         \x20 | awk -v ip=\"$1\" '{{split($4,a,\"/\"); if (a[1]==ip) found=1}} END{{exit !found}}'; }}\n\
         if ip_has \"$IP\"; then BEFORE=true; else BEFORE=false; fi\n\
         write_status() {{\n\
         \x20 # $1=action $2=present_after ; atomic temp+rename onto tmpfs.\n\
         \x20 _now=\"$(date +%s 2>/dev/null || echo 0)\"\n\
         \x20 _t=\"$STATUS_JSON.tmp.$$\"\n\
         \x20 printf '{{\"last_converged\":%s,\"ip\":\"%s\",\"ip_present_before\":%s,\"action_taken\":\"%s\",\"repair_count\":%s,\"ip_present_after\":%s}}\\n' \\\n\
         \x20\x20\x20 \"$_now\" \"$IP\" \"$BEFORE\" \"$1\" \"$REPAIRS\" \"$2\" > \"$_t\" 2>/dev/null \\\n\
         \x20\x20\x20 && mv -f \"$_t\" \"$STATUS_JSON\" 2>/dev/null || rm -f \"$_t\" 2>/dev/null || true\n\
         }}\n\
         fail() {{\n\
         \x20 # Loud, nonzero — drive the systemd unit to `failed`.\n\
         \x20 echo \"fleet-overlay: $1\" >&2\n\
         \x20 write_status \"$2\" \"$(if ip_has \"$IP\"; then echo true; else echo false; fi)\"\n\
         \x20 exit 1\n\
         }}\n\
         {pick}\n\
         [ -n \"$IFACE\" ] || fail 'no LAN interface found (yet)' failed\n\
         CUR_IF=\"$(ip -4 -o addr show 2>/dev/null \\\n\
         \x20 | awk -v ip=\"$IP\" '{{split($4,a,\"/\"); if (a[1]==ip) print $2}}' | head -1)\"\n\
         if [ -n \"$CUR_IF\" ] && [ \"$CUR_IF\" != \"$IFACE\" ]; then\n\
         \x20 ip addr del \"$OVERLAY\" dev \"$CUR_IF\" 2>/dev/null || true\n\
         \x20 CUR_IF=\"\"\n\
         \x20 ACTION=moved\n\
         fi\n\
         if [ \"$CUR_IF\" = \"$IFACE\" ]; then\n\
         \x20 [ \"$ACTION\" = noop ] && ACTION=noop\n\
         \x20 echo \"fleet-overlay: $OVERLAY already on $IFACE\"\n\
         else\n\
         \x20 # RFC 5227 duplicate-address detection: the IP is NOT currently ours,\n\
         \x20 # so probe before claiming it. arping -D exits 0 if free, 1 if a\n\
         \x20 # reply (address in use) is seen. Do NOT `|| true` over a real reply.\n\
         \x20 if command -v arping >/dev/null 2>&1; then\n\
         \x20\x20\x20 if arping -D -c 3 -w 3 -I \"$IFACE\" \"$IP\" >/dev/null 2>&1; then\n\
         \x20\x20\x20\x20\x20 :\n\
         \x20\x20\x20 else\n\
         \x20\x20\x20\x20\x20 fail \"DUPLICATE ADDRESS $IP already in use on the L2 segment (arping -D got a reply on $IFACE) — REFUSING to bind\" conflict\n\
         \x20\x20\x20 fi\n\
         \x20 else\n\
         \x20\x20\x20 echo 'fleet-overlay: WARNING arping not installed — skipping RFC 5227 duplicate-address detection (install iputils-arping)' >&2\n\
         \x20 fi\n\
         \x20 ip addr add \"$OVERLAY\" dev \"$IFACE\" 2>/dev/null || true\n\
         \x20 REPAIRS=$((REPAIRS + 1))\n\
         \x20 printf '%s\\n' \"$REPAIRS\" > \"$COUNT_FILE\" 2>/dev/null || true\n\
         \x20 [ \"$ACTION\" = moved ] || ACTION=reasserted\n\
         \x20 echo \"fleet-overlay: ensured $OVERLAY on $IFACE\"\n\
         fi\n\
         # Persistent-absent => fail loudly so the oneshot enters `failed`.\n\
         if ip_has \"$IP\"; then\n\
         \x20 write_status \"$ACTION\" true\n\
         else\n\
         \x20 fail \"IP $IP STILL ABSENT after converge attempt on $IFACE\" failed\n\
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
             rm -rf {STATUS_DIR}\n\
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
/// Echoes (STDOUT — folded into the drift hash, so every line MUST be stable
/// across runs of a HEALTHY host or drift would false-fire every minute):
///   * whether the overlay IP is bound,
///   * the unit-file shas (a stale unit surfaces as drift),
///   * whether the timer is active,
///   * the convergence-heartbeat *class* — `fresh` / `stale` / `missing` —
///     derived from `/run/fleet-overlay/status.json`'s `last_converged`. A
///     healthy host always emits `fresh`; a wedged ensure loop (never-firing
///     timer / hung pick_iface) lets the heartbeat age past
///     `HEARTBEAT_FRESH_SECS` and flips to `stale` => the silent desync becomes
///     VISIBLE as drift instead of invisible-until-ssh-breaks.
///
/// Emits to STDERR (NOT drift-hashed, so the monotonic/volatile values don't
/// cause spurious drift) the raw heartbeat: `last_converged`, `repair_count`,
/// `action_taken`, `ip_present_after` — for operators / `forjar status`.
pub fn state_query_script(resource: &Resource) -> String {
    let ip_cidr = match overlay_ip(resource) {
        Some(v) if is_valid_overlay_ip(v) => v,
        other => return reject_bad_ip(other.unwrap_or("<none>")),
    };
    let ip = ip_cidr.split('/').next().unwrap_or(ip_cidr);
    let ip_q = sh_squote(ip);
    let fresh = HEARTBEAT_FRESH_SECS;

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
         echo \"overlay_timer_active=$(systemctl is-active fleet-overlay.timer 2>/dev/null || echo unknown)\"\n\
         # -- convergence heartbeat (silent-desync detector) --\n\
         LC=\"$(sed -n 's/.*\"last_converged\":\\([0-9][0-9]*\\).*/\\1/p' {STATUS_JSON} 2>/dev/null | head -1)\"\n\
         RC=\"$(sed -n 's/.*\"repair_count\":\\([0-9][0-9]*\\).*/\\1/p' {STATUS_JSON} 2>/dev/null | head -1)\"\n\
         AT=\"$(sed -n 's/.*\"action_taken\":\"\\([a-z][a-z]*\\)\".*/\\1/p' {STATUS_JSON} 2>/dev/null | head -1)\"\n\
         if [ -z \"$LC\" ]; then\n\
         \x20 echo 'overlay_heartbeat=missing'\n\
         else\n\
         \x20 NOW=\"$(date +%s 2>/dev/null || echo 0)\"\n\
         \x20 AGE=$((NOW - LC))\n\
         \x20 if [ \"$AGE\" -ge 0 ] && [ \"$AGE\" -le {fresh} ]; then\n\
         \x20\x20\x20 echo 'overlay_heartbeat=fresh'\n\
         \x20 else\n\
         \x20\x20\x20 echo 'overlay_heartbeat=stale'\n\
         \x20 fi\n\
         fi\n\
         # raw heartbeat -> STDERR only (NOT drift-hashed): operator observability.\n\
         echo \"overlay_last_converged=${{LC:-none}} overlay_repair_count=${{RC:-0}} overlay_action_taken=${{AT:-unknown}}\" >&2"
    )
}
