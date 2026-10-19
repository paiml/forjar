//! `is_local_addr` must recognise an address held by a local interface, not
//! only the loopback spellings and /etc/hostname.
//!
//! A machine declared by its stable overlay address (`addr: 10.42.0.20`) and
//! applied ON that machine was dispatched to SSH, because the function said
//! "check if the address matches any local interface" and never did. On a
//! host with no sshd that is a connection refused to itself.

use super::local_iface::ip_held_locally;
use super::*;
use std::net::{IpAddr, UdpSocket};

/// The source address the kernel would use for an outbound packet, if any.
/// `connect` on UDP sends nothing; it only resolves the route.
fn route_source_v4() -> Option<IpAddr> {
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("192.0.2.1:9").ok()?;
    let ip = s.local_addr().ok()?.ip();
    (!ip.is_unspecified()).then_some(ip)
}

#[test]
fn test_local_iface_route_source_addr_is_local() {
    // No route at all (sandboxed CI): nothing to measure, and the loopback
    // tests below still discriminate.
    let Some(ip) = route_source_v4() else { return };
    assert!(
        is_local_addr(&ip.to_string()),
        "{ip} is assigned to a local interface but was classified remote"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn test_local_iface_whole_loopback_net_is_local() {
    // Linux routes all of 127.0.0.0/8 to lo; only the 127.0.0.1 spelling was
    // special-cased.
    assert!(is_local_addr("127.0.0.2"));
}

#[test]
fn test_local_iface_unowned_addrs_stay_remote() {
    assert!(!is_local_addr("192.0.2.1")); // TEST-NET-1, never assigned
    assert!(!is_local_addr("0.0.0.0")); // binds anywhere; owns nothing
    assert!(!is_local_addr("::"));
    assert!(!is_local_addr("224.0.0.1")); // multicast
    assert!(!is_local_addr("255.255.255.255"));
    assert!(!is_local_addr("not-an-ip.invalid"));
}

#[test]
fn test_local_iface_nonlocal_bind_never_claims_local() {
    // With net.ipv4.ip_nonlocal_bind=1 every bind succeeds, so a bind probe
    // would call EVERY host local and forjar would run a remote machine's
    // resources here. The probe must decline rather than guess.
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    assert!(!ip_held_locally(ip, true));
    assert!(ip_held_locally(ip, false));
}
