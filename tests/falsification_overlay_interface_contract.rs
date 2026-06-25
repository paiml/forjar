//! FJ-035: overlay_interface CONTRACT-teeth falsification (capstone PMAT-131).
//!
//! Popperian rejection criteria for the proof_obligations of
//! contracts/overlay-interface-v1.yaml — RFC 5227 DAD, status.json heartbeat,
//! jittered timer, injective overlay_ip, overlay_hosts hash sensitivity, and
//! idempotency. Split from falsification_overlay_interface.rs to keep each
//! file under the 500-line file-health limit (CLAUDE.md).
//!
//! Usage: cargo test --test falsification_overlay_interface_contract

#![allow(clippy::field_reassign_with_default)]

use forjar::core::parser::{parse_config, validate_config};
use forjar::core::planner::hash_desired_state;
use forjar::core::types::{Resource, ResourceType};
use forjar::resources::overlay_interface::{apply_script, state_query_script};

fn overlay(ip: &str) -> Resource {
    let mut r = Resource::default();
    r.resource_type = ResourceType::OverlayInterface;
    r.overlay_ip = Some(ip.into());
    r.sudo = true;
    r
}

// ============================================================================
// overlay-interface-v1 CONTRACT teeth (capstone — PMAT-131)
//
// Each test below is a Popperian rejection criterion for one proof_obligation
// of contracts/overlay-interface-v1.yaml. If a test goes red, the obligation
// it names is FALSIFIED.
// ============================================================================

/// CONTRACT P1 (no_remain_after_exit): the emitted service NEVER contains
/// RemainAfterExit — else the timer's restart becomes a no-op and the IP never
/// self-heals after a DHCP flush.
#[test]
fn contract_service_never_remain_after_exit() {
    let s = apply_script(&overlay("10.42.0.11/24"));
    assert!(s.contains("Type=oneshot"), "{s}");
    assert!(
        !s.contains("RemainAfterExit"),
        "CONTRACT VIOLATION: service must NEVER set RemainAfterExit: {s}"
    );
}

/// CONTRACT P2 (timer_wall_clock_jittered): the emitted timer ALWAYS uses
/// OnCalendar (never OnUnitActiveSec) AND carries RandomizedDelaySec (+
/// AccuracySec) for anti-thundering-herd jitter.
#[test]
fn contract_timer_oncalendar_and_randomized_delay() {
    let s = apply_script(&overlay("10.42.0.11/24"));
    assert!(s.contains("OnCalendar=minutely"), "{s}");
    assert!(
        !s.contains("OnUnitActiveSec"),
        "CONTRACT VIOLATION: timer must NEVER use OnUnitActiveSec: {s}"
    );
    assert!(
        s.contains("RandomizedDelaySec=30"),
        "CONTRACT VIOLATION (MUST-4): timer must carry RandomizedDelaySec jitter: {s}"
    );
    assert!(
        s.contains("AccuracySec=1s"),
        "CONTRACT VIOLATION (MUST-4): timer must set AccuracySec=1s so jitter is not coalesced: {s}"
    );
}

/// CONTRACT P3 (dad_before_bind): apply contains the RFC 5227 `arping -D` probe
/// BEFORE the `ip addr add`, and refuses (no `|| true`) on a reply.
#[test]
fn contract_arping_dad_probe_before_ip_addr_add() {
    let s = apply_script(&overlay("10.42.0.11/24"));
    let dad = s
        .find("arping -D")
        .expect("apply must contain arping -D DAD probe");
    let add = s
        .find("ip addr add")
        .expect("apply must contain ip addr add");
    assert!(
        dad < add,
        "CONTRACT VIOLATION (MUST-1): arping -D must appear BEFORE ip addr add: {s}"
    );
    assert!(s.contains("arping -D -c 3 -w 3 -I"), "{s}");
    // On a duplicate, REFUSE — must not silently `|| true` over a real conflict.
    assert!(
        s.contains("DUPLICATE ADDRESS"),
        "CONTRACT VIOLATION (MUST-1): DAD conflict must be a loud refusal: {s}"
    );
    // arping-absent guard (skip + warn), not a hard failure.
    assert!(
        s.contains("arping not installed") && s.contains("command -v arping"),
        "DAD probe must be guarded for arping-absent: {s}"
    );
}

/// CONTRACT P4 (status_json_and_fail_on_absent): apply writes
/// /run/fleet-overlay/status.json with the heartbeat fields, increments a
/// monotonic repair_count, and exits nonzero when the IP is still absent.
#[test]
fn contract_writes_status_json_and_fails_on_persistent_absent() {
    let s = apply_script(&overlay("10.42.0.11/24"));
    assert!(s.contains("/run/fleet-overlay/status.json"), "{s}");
    for field in [
        "last_converged",
        "ip_present_before",
        "action_taken",
        "repair_count",
        "ip_present_after",
    ] {
        assert!(
            s.contains(field),
            "CONTRACT VIOLATION (MUST-2): status.json must carry `{field}`: {s}"
        );
    }
    // monotonic repair counter: read+increment.
    assert!(s.contains("REPAIRS=$((REPAIRS + 1))"), "{s}");
    assert!(s.contains("/run/fleet-overlay/repair_count"), "{s}");
    // persistent-absent => nonzero exit (unit enters `failed`).
    assert!(
        s.contains("STILL ABSENT") && s.contains("exit 1"),
        "CONTRACT VIOLATION (MUST-2): persistent-absent must exit nonzero: {s}"
    );
}

/// CONTRACT P5 (heartbeat_drift_visible): state_query surfaces the heartbeat
/// class (fresh/stale/missing) on STDOUT (drift-visible) while the raw
/// monotonic values go to STDERR (NOT drift-hashed, so a healthy host's
/// ever-advancing counters don't false-fire drift every minute).
#[test]
fn contract_state_query_surfaces_heartbeat_without_false_drift() {
    let s = state_query_script(&overlay("10.42.0.11/24"));
    assert!(s.contains("overlay_heartbeat="), "{s}");
    assert!(s.contains("overlay_heartbeat=stale"), "{s}");
    assert!(s.contains("overlay_heartbeat=fresh"), "{s}");
    // Raw volatile values are emitted to stderr (>&2), never plain stdout.
    let stderr_line = s
        .lines()
        .find(|l| l.contains("overlay_repair_count="))
        .expect("repair_count line present");
    assert!(
        stderr_line.contains(">&2"),
        "CONTRACT VIOLATION (MUST-2): raw repair_count/last_converged must go to stderr to avoid false drift: {s}"
    );
}

/// CONTRACT P6 (overlay_ip_injective): a config with two overlay_interface
/// resources claiming the SAME overlay_ip on one machine is REJECTED at
/// validate-time, naming both resource ids.
#[test]
fn contract_duplicate_overlay_ip_rejected_at_validate() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  intel:
    hostname: intel
    addr: 127.0.0.1
    user: test
resources:
  fleet-a:
    type: overlay_interface
    machine: intel
    sudo: true
    overlay_ip: "10.42.0.11/24"
  fleet-b:
    type: overlay_interface
    machine: intel
    sudo: true
    overlay_ip: "10.42.0.11/16"
"#;
    let config = parse_config(yaml).expect("parses as YAML");
    let errors = validate_config(&config);
    let dup = errors
        .iter()
        .find(|e| e.message.contains("injective") && e.message.contains("10.42.0.11"));
    let msg = dup.map(|e| e.message.clone()).unwrap_or_default();
    assert!(
        dup.is_some(),
        "CONTRACT VIOLATION (MUST-1): duplicate overlay_ip must be a validation error: {errors:?}"
    );
    assert!(
        msg.contains("fleet-a") && msg.contains("fleet-b"),
        "duplicate-IP error must name BOTH resources: {msg}"
    );
}

/// Injectivity must NOT false-fire on DISTINCT IPs (no spurious rejection).
#[test]
fn contract_distinct_overlay_ips_accepted() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  intel:
    hostname: intel
    addr: 127.0.0.1
    user: test
resources:
  fleet-a:
    type: overlay_interface
    machine: intel
    sudo: true
    overlay_ip: "10.42.0.11/24"
  fleet-b:
    type: overlay_interface
    machine: intel
    sudo: true
    overlay_ip: "10.42.0.12/24"
"#;
    let config = parse_config(yaml).expect("parses as YAML");
    let errors = validate_config(&config);
    assert!(
        !errors.iter().any(|e| e.message.contains("injective")),
        "distinct overlay_ips must NOT trip the injectivity check: {errors:?}"
    );
}

/// CONTRACT P7 (hash_overlay_hosts_sensitive): two resources differing ONLY in
/// overlay_hosts must hash differently, so plan never false-reports NoOp on a
/// changed /etc/hosts block. (The MATERIAL FIX.)
#[test]
fn contract_changing_overlay_hosts_changes_desired_hash() {
    let mut a = overlay("10.42.0.11/24");
    let mut ha = std::collections::HashMap::new();
    ha.insert("intel".to_string(), "10.42.0.11".to_string());
    a.overlay_hosts = Some(ha);

    let mut b = overlay("10.42.0.11/24");
    let mut hb = std::collections::HashMap::new();
    hb.insert("intel".to_string(), "10.42.0.11".to_string());
    hb.insert("mini".to_string(), "10.42.0.12".to_string());
    b.overlay_hosts = Some(hb);

    assert_ne!(
        hash_desired_state(&a),
        hash_desired_state(&b),
        "CONTRACT VIOLATION (MATERIAL): overlay_hosts must affect the desired-state hash"
    );
}

/// overlay_hosts canonicalization is order-independent (HashMap insertion order
/// must not change the hash) — same entries => same hash.
#[test]
fn contract_overlay_hosts_hash_is_order_independent() {
    let mut a = overlay("10.42.0.11/24");
    let mut ha = std::collections::HashMap::new();
    ha.insert("intel".to_string(), "10.42.0.11".to_string());
    ha.insert("mini".to_string(), "10.42.0.12".to_string());
    ha.insert("lambda-labs".to_string(), "10.42.0.10".to_string());
    a.overlay_hosts = Some(ha);

    let mut b = overlay("10.42.0.11/24");
    let mut hb = std::collections::HashMap::new();
    hb.insert("lambda-labs".to_string(), "10.42.0.10".to_string());
    hb.insert("mini".to_string(), "10.42.0.12".to_string());
    hb.insert("intel".to_string(), "10.42.0.11".to_string());
    b.overlay_hosts = Some(hb);

    assert_eq!(
        hash_desired_state(&a),
        hash_desired_state(&b),
        "same overlay_hosts entries must hash identically regardless of insertion order"
    );
}

/// CONTRACT P8 (idempotent_apply): apply over a converged resource is a NoOp at
/// the plan level — a second `hash_desired_state` of an unchanged resource is
/// identical (the hash that drives the NoOp decision is stable).
#[test]
fn contract_apply_idempotent_hash_stable() {
    let mut r = overlay("10.42.0.15/24");
    let mut h = std::collections::HashMap::new();
    h.insert("intel".to_string(), "10.42.0.15".to_string());
    r.overlay_hosts = Some(h);
    r.overlay_firewall = Some(true);
    assert_eq!(
        hash_desired_state(&r),
        hash_desired_state(&r),
        "CONTRACT VIOLATION: re-running apply on an unchanged resource must remain a NoOp (stable hash)"
    );
}
