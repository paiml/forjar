//! A machine declared by an address one of THIS host's interfaces holds runs
//! on the local transport, not over SSH to itself.
//!
//! `is_local_addr` said "check if the address matches any local interface"
//! and only compared against the loopback spellings and /etc/hostname. A box
//! declared by its static overlay address (`addr: 10.42.0.20`) and applied on
//! itself was dispatched to SSH — connection refused on a host with no sshd.
//!
//! Dispatch is observed, not inferred: the script echoes `$PPID`. The local
//! transport spawns bash as a child of THIS process, so its parent is our pid;
//! over SSH the parent is sshd. A matching pid proves the script ran here,
//! whatever sshd the host happens to run.

use forjar::core::types::Machine;
use forjar::transport::{exec_script, is_local_addr};
use std::net::{IpAddr, UdpSocket};

/// The source address the kernel would use for an outbound packet. UDP
/// `connect` sends nothing; it only resolves the route. `None` with no route.
fn route_source_v4() -> Option<IpAddr> {
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("192.0.2.1:9").ok()?;
    let ip = s.local_addr().ok()?.ip();
    (!ip.is_unspecified() && !ip.is_loopback()).then_some(ip)
}

fn machine_at(addr: &str) -> Machine {
    serde_yaml_ng::from_str(&format!("hostname: self-by-iface\naddr: {addr}\n"))
        .expect("machine yaml")
}

#[test]
fn falsify_iface_addr_is_local() {
    let Some(ip) = route_source_v4() else {
        eprintln!("no IPv4 route on this host: nothing to measure");
        return;
    };
    assert!(
        is_local_addr(&ip.to_string()),
        "{ip} is held by a local interface but was classified remote"
    );
}

#[test]
fn falsify_iface_addr_dispatches_locally() {
    let Some(ip) = route_source_v4() else {
        eprintln!("no IPv4 route on this host: nothing to measure");
        return;
    };
    let out = exec_script(&machine_at(&ip.to_string()), "echo \"$PPID\"")
        .unwrap_or_else(|e| panic!("addr {ip} (a local interface) was not run locally: {e}"));
    assert_eq!(
        out.stdout.trim(),
        std::process::id().to_string(),
        "addr {ip} ran, but not as a child of this process"
    );
}

/// Needs no network: Linux routes all of 127.0.0.0/8 to `lo`, so 127.0.0.2 is
/// held on every Linux runner, sandboxed or not. The route-source tests above
/// return early without a route; this one cannot.
#[cfg(target_os = "linux")]
#[test]
fn falsify_loopback_alias_dispatches_locally() {
    let out = exec_script(&machine_at("127.0.0.2"), "echo \"$PPID\"")
        .unwrap_or_else(|e| panic!("127.0.0.2 (held by lo) was not run locally: {e}"));
    assert_eq!(out.stdout.trim(), std::process::id().to_string());
}

#[test]
fn falsify_unowned_addr_stays_remote() {
    // TEST-NET-1 is never assigned: the fix must not widen "local" to it.
    assert!(!is_local_addr("192.0.2.1"));
    assert!(!is_local_addr("0.0.0.0"));
    assert!(!is_local_addr("224.0.0.1"));
    assert!(!is_local_addr("255.255.255.255"));
    // IPv4-mapped IPv6 is judged as the IPv4 address it is.
    assert!(!is_local_addr("::ffff:192.0.2.1"));
}

/// Linux lets a UDP socket BIND a subnet's directed broadcast, which no
/// interface holds; a bind-only probe called these local (quorum r1, measured
/// on 192.168.1.255, 10.42.0.255, 172.17.255.255 and 127.255.255.255).
#[test]
fn falsify_directed_broadcast_stays_remote() {
    #[cfg(target_os = "linux")]
    assert!(
        !is_local_addr("127.255.255.255"),
        "lo's broadcast is not held"
    );
    let Some(IpAddr::V4(v4)) = route_source_v4() else {
        eprintln!("no IPv4 route on this host: only the loopback case ran");
        return;
    };
    // The .255 of the held address: its /24 broadcast, or on a wider prefix
    // some other host's address — remote either way.
    let [a, b, c, _] = v4.octets();
    let bcast = format!("{a}.{b}.{c}.255");
    assert!(!is_local_addr(&bcast), "{bcast} is a broadcast, not held");
    assert!(!is_local_addr(&format!("::ffff:{bcast}")));
}

#[test]
fn falsify_mapped_held_addr_is_local() {
    assert!(is_local_addr("::ffff:127.0.0.1"));
}
