//! FJ-035: inline-handler unit tests for the overlay_interface resource.
//! Extracted from overlay_interface.rs to keep that file under the 500-line
//! file-health limit (CLAUDE.md). The Popperian *falsification* suite lives in
//! tests/falsification_overlay_interface.rs.

#![allow(clippy::field_reassign_with_default)]

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
    assert!(state_query_script(&r).contains("ERROR: overlay_interface requires a valid overlay_ip"));
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

#[test]
fn apply_emits_rfc5227_dad_probe_before_bind() {
    let r = make_overlay("10.42.0.11/24");
    let s = apply_script(&r);
    let dad = s.find("arping -D").expect("DAD probe present");
    let add = s.find("ip addr add").expect("ip addr add present");
    assert!(dad < add, "DAD must precede bind: {s}");
    assert!(s.contains("DUPLICATE ADDRESS"), "{s}");
    assert!(s.contains("command -v arping"), "arping-absent guard: {s}");
}

#[test]
fn apply_writes_status_json_and_fails_on_absent() {
    let r = make_overlay("10.42.0.11/24");
    let s = apply_script(&r);
    assert!(s.contains("/run/fleet-overlay/status.json"), "{s}");
    assert!(s.contains("repair_count"), "{s}");
    assert!(s.contains("REPAIRS=$((REPAIRS + 1))"), "{s}");
    assert!(s.contains("STILL ABSENT") && s.contains("exit 1"), "{s}");
}

#[test]
fn timer_has_randomized_delay_jitter() {
    let r = make_overlay("10.42.0.11/24");
    let s = apply_script(&r);
    assert!(s.contains("RandomizedDelaySec=30"), "{s}");
    assert!(s.contains("AccuracySec=1s"), "{s}");
}

#[test]
fn state_query_surfaces_heartbeat_class() {
    let r = make_overlay("10.42.0.11/24");
    let s = state_query_script(&r);
    assert!(s.contains("overlay_heartbeat=fresh"), "{s}");
    assert!(s.contains("overlay_heartbeat=stale"), "{s}");
    assert!(s.contains("overlay_heartbeat=missing"), "{s}");
    // raw counters go to stderr to avoid false drift.
    assert!(
        s.contains("overlay_repair_count=") && s.contains(">&2"),
        "{s}"
    );
}

#[test]
fn absent_removes_status_dir() {
    let mut r = make_overlay("10.42.0.11/24");
    r.state = Some("absent".to_string());
    let s = apply_script(&r);
    assert!(s.contains("rm -rf /run/fleet-overlay"), "{s}");
}
