//! FJ-035: Fleet overlay-interface resource type falsification.
//!
//! Popperian rejection criteria for the DNS/DHCP-independent fleet overlay:
//! - Script generation (check, apply, state_query)
//! - The three validated self-heal anti-regressions:
//!     1. service is a PLAIN oneshot (NO RemainAfterExit)
//!     2. timer uses OnCalendar=minutely (NOT OnUnitActiveSec)
//!     3. apply RESTARTs (not just start/enable --now) after daemon-reload
//! - NM-vs-networkd detection keys on the dispatcher dir
//! - Config validation (missing/invalid overlay_ip, invalid interface)
//! - Planner default-state / desired-hash sensitivity
//! - Absent-state teardown
//! - Shell-injection hardening on overlay_ip / interface
//!
//! Usage: cargo test --test falsification_overlay_interface

#![allow(clippy::field_reassign_with_default)]

use forjar::core::parser::{parse_config, validate_config};
use forjar::core::planner::hash_desired_state;
use forjar::core::types::{Resource, ResourceType};
use forjar::resources::overlay_interface::{apply_script, check_script, state_query_script};

// ============================================================================
// Helpers
// ============================================================================

fn overlay(ip: &str) -> Resource {
    let mut r = Resource::default();
    r.resource_type = ResourceType::OverlayInterface;
    r.overlay_ip = Some(ip.into());
    r.sudo = true;
    r
}

// ============================================================================
// check_script
// ============================================================================

#[test]
fn check_reports_ip_and_units() {
    let s = check_script(&overlay("10.42.0.11/24"));
    assert!(s.contains("ip:present:10.42.0.11"));
    assert!(s.contains("ip:absent:10.42.0.11"));
    assert!(s.contains("/etc/systemd/system/fleet-overlay.service"));
    assert!(s.contains("/etc/systemd/system/fleet-overlay.timer"));
}

// ============================================================================
// apply_script — the three anti-regressions
// ============================================================================

/// REGRESSION GUARD #1: the service must be a PLAIN Type=oneshot.
/// RemainAfterExit=yes makes the timer's `start` a no-op so the IP never
/// self-heals after a DHCP flush.
#[test]
fn service_is_plain_oneshot_no_remain_after_exit() {
    let s = apply_script(&overlay("10.42.0.11/24"));
    assert!(s.contains("Type=oneshot"), "{s}");
    assert!(
        !s.contains("RemainAfterExit"),
        "service MUST NOT set RemainAfterExit: {s}"
    );
}

/// REGRESSION GUARD #2: the timer must use OnCalendar=minutely (wall-clock),
/// NOT OnUnitActiveSec.
#[test]
fn timer_uses_oncalendar_minutely() {
    let s = apply_script(&overlay("10.42.0.11/24"));
    assert!(s.contains("OnCalendar=minutely"), "{s}");
    assert!(s.contains("OnBootSec=20s"), "{s}");
    assert!(s.contains("Persistent=true"), "{s}");
    assert!(
        !s.contains("OnUnitActiveSec"),
        "timer MUST NOT use OnUnitActiveSec: {s}"
    );
}

/// REGRESSION GUARD #3: on a unit-content change apply must daemon-reload +
/// RESTART (service and timer), never start/enable --now alone.
#[test]
fn apply_daemon_reload_then_restart_both_units() {
    let s = apply_script(&overlay("10.42.0.11/24"));
    assert!(s.contains("systemctl daemon-reload"), "{s}");
    assert!(s.contains("systemctl restart fleet-overlay.service"), "{s}");
    assert!(s.contains("systemctl restart fleet-overlay.timer"), "{s}");
}

// ============================================================================
// apply_script — core overlay-IP binding + NM detection
// ============================================================================

#[test]
fn apply_binds_overlay_ip_via_execstart() {
    let s = apply_script(&overlay("10.42.0.11/24"));
    assert!(
        s.contains("ExecStart=/usr/local/sbin/fleet-overlay.sh 10.42.0.11/24"),
        "{s}"
    );
    assert!(s.contains("ip addr add"), "{s}");
    assert!(s.contains("set -euo pipefail"), "{s}");
}

#[test]
fn apply_autodetects_default_route_nic_when_no_interface() {
    let s = apply_script(&overlay("10.42.0.11/24"));
    assert!(s.contains("ip route show default"), "{s}");
    assert!(s.contains("docker|br-|veth"), "skip virtual ifaces: {s}");
}

#[test]
fn apply_uses_explicit_interface_when_given() {
    let mut r = overlay("10.42.0.13/24");
    r.overlay_iface = Some("enp9s0".into());
    let s = apply_script(&r);
    assert!(s.contains("IFACE='enp9s0'"), "{s}");
    assert!(!s.contains("ip route show default"), "{s}");
}

/// NM-vs-networkd: dispatcher hook is installed conditionally on the dispatcher
/// directory existing, and uses `restart --no-block` (not start).
#[test]
fn apply_nm_dispatcher_conditional_and_restart_no_block() {
    let s = apply_script(&overlay("10.42.0.10/24"));
    assert!(
        s.contains("if [ -d /etc/NetworkManager/dispatcher.d ]"),
        "{s}"
    );
    assert!(s.contains("50-fleet-overlay"), "{s}");
    assert!(
        s.contains("restart --no-block fleet-overlay.service"),
        "{s}"
    );
}

// ============================================================================
// Optional sub-features: hosts block + firewall
// ============================================================================

#[test]
fn apply_optional_firewall_opens_overlay_subnet() {
    let mut r = overlay("10.42.0.11/24");
    r.overlay_firewall = Some(true);
    let s = apply_script(&r);
    assert!(s.contains("ufw allow from 10.42.0.0/24"), "{s}");
}

#[test]
fn apply_firewall_omitted_by_default() {
    let s = apply_script(&overlay("10.42.0.11/24"));
    assert!(!s.contains("ufw allow"), "{s}");
}

#[test]
fn apply_optional_hosts_block_with_marker_safety() {
    let mut r = overlay("10.42.0.11/24");
    let mut h = std::collections::HashMap::new();
    h.insert("intel".to_string(), "10.42.0.11".to_string());
    r.overlay_hosts = Some(h);
    let s = apply_script(&r);
    assert!(s.contains("# BEGIN forjar-fleet"), "{s}");
    assert!(s.contains("# END forjar-fleet"), "{s}");
    assert!(
        s.contains("refusing to edit"),
        "unbalanced-marker guard: {s}"
    );
}

// ============================================================================
// absent
// ============================================================================

#[test]
fn apply_absent_tears_down_units_and_ip() {
    let mut r = overlay("10.42.0.11/24");
    r.state = Some("absent".into());
    let s = apply_script(&r);
    assert!(s.contains("disable --now fleet-overlay.timer"), "{s}");
    assert!(
        s.contains("rm -f /etc/systemd/system/fleet-overlay.service"),
        "{s}"
    );
    assert!(s.contains("ip addr del 10.42.0.11/24"), "{s}");
}

// ============================================================================
// state_query
// ============================================================================

#[test]
fn state_query_reports_ip_unit_shas_and_timer() {
    let s = state_query_script(&overlay("10.42.0.11/24"));
    assert!(s.contains("overlay_ip=present:10.42.0.11"));
    assert!(s.contains("overlay_service_sha="));
    assert!(s.contains("overlay_timer_sha="));
    assert!(s.contains("overlay_timer_active="));
}

// ============================================================================
// Config validation (end-to-end via parse_config)
// ============================================================================

fn config_with(resource_yaml: &str) -> String {
    format!(
        r#"
version: "1.0"
name: test
machines:
  intel:
    hostname: intel
    addr: 127.0.0.1
    user: test
resources:
  fleet-overlay:
{resource_yaml}
"#
    )
}

#[test]
fn validate_valid_overlay_config_parses() {
    let yaml = config_with(
        "    type: overlay_interface\n\
         \x20\x20\x20\x20machine: intel\n\
         \x20\x20\x20\x20sudo: true\n\
         \x20\x20\x20\x20overlay_ip: \"10.42.0.11/24\"\n\
         \x20\x20\x20\x20overlay_firewall: true",
    );
    let config = parse_config(&yaml);
    assert!(
        config.is_ok(),
        "valid overlay config must parse: {config:?}"
    );
    let cfg = config.unwrap();
    let r = cfg.resources.get("fleet-overlay").unwrap();
    assert_eq!(r.resource_type, ResourceType::OverlayInterface);
    assert_eq!(r.overlay_ip.as_deref(), Some("10.42.0.11/24"));
    assert_eq!(r.overlay_firewall, Some(true));
}

#[test]
fn validate_missing_ip_rejected() {
    let yaml = config_with(
        "    type: overlay_interface\n\
         \x20\x20\x20\x20machine: intel",
    );
    let config = parse_config(&yaml).expect("parses as YAML");
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.message.contains("no overlay_ip")),
        "missing overlay_ip must be a validation error: {errors:?}"
    );
}

#[test]
fn validate_bad_ip_rejected() {
    let yaml = config_with(
        "    type: overlay_interface\n\
         \x20\x20\x20\x20machine: intel\n\
         \x20\x20\x20\x20overlay_ip: \"not-an-ip\"",
    );
    let config = parse_config(&yaml).expect("parses as YAML");
    let errors = validate_config(&config);
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("not a valid IPv4/CIDR")),
        "invalid overlay_ip must be a validation error: {errors:?}"
    );
}

#[test]
fn validate_overlay_hosts_parses_as_map() {
    let yaml = config_with(
        "    type: overlay_interface\n\
         \x20\x20\x20\x20machine: intel\n\
         \x20\x20\x20\x20overlay_ip: \"10.42.0.11/24\"\n\
         \x20\x20\x20\x20overlay_hosts:\n\
         \x20\x20\x20\x20\x20\x20intel: \"10.42.0.11\"\n\
         \x20\x20\x20\x20\x20\x20mini: \"10.42.0.12\"",
    );
    let config = parse_config(&yaml).expect("overlay_hosts map must parse");
    let r = config.resources.get("fleet-overlay").unwrap();
    let hosts = r.overlay_hosts.as_ref().unwrap();
    assert_eq!(hosts.get("mini").map(String::as_str), Some("10.42.0.12"));
}

// ============================================================================
// Planner: desired-hash sensitivity (plan must not mis-detect NoOp)
// ============================================================================

#[test]
fn changing_overlay_ip_changes_desired_hash() {
    let a = hash_desired_state(&overlay("10.42.0.11/24"));
    let b = hash_desired_state(&overlay("10.42.0.12/24"));
    assert_ne!(a, b, "different overlay_ip must hash differently");
}

#[test]
fn changing_firewall_flag_changes_desired_hash() {
    let mut on = overlay("10.42.0.11/24");
    on.overlay_firewall = Some(true);
    let mut off = overlay("10.42.0.11/24");
    off.overlay_firewall = Some(false);
    assert_ne!(
        hash_desired_state(&on),
        hash_desired_state(&off),
        "firewall flag must affect desired hash"
    );
}

#[test]
fn same_overlay_resource_hashes_deterministically() {
    assert_eq!(
        hash_desired_state(&overlay("10.42.0.15/24")),
        hash_desired_state(&overlay("10.42.0.15/24"))
    );
}

// ============================================================================
// Shell-injection hardening
// ============================================================================

#[test]
fn injection_in_ip_rejected_all_phases() {
    let r = overlay("10.42.0.11/24;reboot");
    for s in [check_script(&r), apply_script(&r), state_query_script(&r)] {
        assert!(
            s.contains("ERROR: overlay_interface requires a valid overlay_ip"),
            "{s}"
        );
        assert!(!s.contains("reboot\n"), "{s}");
    }
}

#[test]
fn injection_in_interface_rejected() {
    let mut r = overlay("10.42.0.13/24");
    r.overlay_iface = Some("eth0; rm -rf /".into());
    let s = apply_script(&r);
    assert!(
        s.contains("ERROR: overlay_interface invalid interface name"),
        "{s}"
    );
}
