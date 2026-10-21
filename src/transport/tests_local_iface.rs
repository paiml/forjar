//! `is_local_addr` must recognise an address held by a local interface, not
//! only the loopback spellings and /etc/hostname.
//!
//! A machine declared by its stable overlay address (`addr: 10.42.0.20`) and
//! applied ON that machine was dispatched to SSH, because the function said
//! "check if the address matches any local interface" and never did. On a
//! host with no sshd that is a connection refused to itself.

use super::local_iface::{
    ip_held_locally, nonlocal_bind_path, nonlocal_bind_read, probe_target, sysctl_enabled,
};
use super::*;
use std::net::{IpAddr, UdpSocket};

/// The source address the kernel would use for an outbound packet, if any.
/// `connect` on UDP sends nothing; it only resolves the route.
/// The probe declines (by design) where the nonlocal-bind sysctl is unreadable
/// or set, e.g. a sandbox with /proc/sys masked; a held-address assertion has
/// nothing to measure there.
fn probe_can_answer() -> bool {
    std::fs::read_to_string("/proc/sys/net/ipv4/ip_nonlocal_bind").is_ok_and(|v| v.trim() == "0")
}

fn route_source_v4() -> Option<IpAddr> {
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("192.0.2.1:9").ok()?;
    let ip = s.local_addr().ok()?.ip();
    (!ip.is_unspecified()).then_some(ip)
}

#[test]
fn test_local_iface_route_source_addr_is_local() {
    if !probe_can_answer() {
        eprintln!("nonlocal-bind sysctl unreadable or set: the probe declines here");
        return;
    }
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
    if !probe_can_answer() {
        eprintln!("nonlocal-bind sysctl unreadable or set: the probe declines here");
        return;
    }
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
    // Off Linux the probe declines regardless (test_local_iface_declines_off_linux).
    #[cfg(target_os = "linux")]
    assert!(ip_held_locally(ip, false));
}

#[test]
fn test_local_iface_unreadable_sysctl_declines() {
    // A host that will not say (masked /proc/sys) is not trusted to answer.
    let masked = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
    assert!(nonlocal_bind_read(Err(masked)));
    assert!(!nonlocal_bind_read(Ok("0\n".into())));
    assert!(nonlocal_bind_read(Ok("1\n".into())));
}

#[test]
fn test_local_iface_sysctl_path_follows_family() {
    // ::ffff:a.b.c.d reaches here already folded, so it reads the IPv4 one.
    let mapped = probe_target("::ffff:10.42.0.20").unwrap();
    assert_eq!(
        nonlocal_bind_path(mapped),
        "/proc/sys/net/ipv4/ip_nonlocal_bind"
    );
    let v6: IpAddr = "::1".parse().unwrap();
    assert_eq!(
        nonlocal_bind_path(v6),
        "/proc/sys/net/ipv6/ip_nonlocal_bind"
    );
}

#[test]
fn test_local_iface_sysctl_value_parse() {
    // Only an exact 0 lets the probe answer; anything else declines it.
    assert!(!sysctl_enabled("0\n"));
    assert!(!sysctl_enabled(" 0 "));
    assert!(sysctl_enabled("1\n"));
    assert!(sysctl_enabled("2"));
    assert!(sysctl_enabled(""));
}

#[test]
fn test_local_iface_mapped_v6_is_judged_as_v4() {
    // `::ffff:a.b.c.d` binds through the IPv4 stack, so the IPv4 sysctl must
    // govern it: the probe has to see the IPv4 address, not the v6 spelling.
    let v4: IpAddr = "10.42.0.20".parse().unwrap();
    assert_eq!(probe_target("::ffff:10.42.0.20"), Some(v4));
    assert!(probe_target("::ffff:10.42.0.20").unwrap().is_ipv4());
    assert!(probe_target("::1").unwrap().is_ipv6());
    assert_eq!(probe_target("thinkpad"), None);
}

/// Outside Linux the probe declines: its broadcast refusal is measured only on
/// Linux. 127.0.0.2 is not held on macOS's lo0 either, and stays remote.
#[cfg(not(target_os = "linux"))]
#[test]
fn test_local_iface_declines_off_linux() {
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    assert!(!ip_held_locally(ip, false));
    assert!(!is_local_addr("127.0.0.2"));
    assert!(is_local_addr("127.0.0.1")); // the spelling match is unchanged
}

/// The documented false negative: a held IPv6 link-local address is judged
/// remote (no scope id to bind with), so it keeps going over SSH. Pinned so
/// that recognising it later is a deliberate change, not a drive-by one.
#[cfg(target_os = "linux")]
#[test]
fn test_local_iface_link_local_is_a_false_negative() {
    let Ok(table) = std::fs::read_to_string("/proc/net/if_inet6") else {
        return;
    };
    // Rows: 32 hex digits, ifindex, prefix len, scope (20 = link), flags, name.
    let Some(hex) = table
        .lines()
        .filter_map(|l| l.split_whitespace().next().zip(l.split_whitespace().nth(3)))
        .find(|(_, scope)| *scope == "20")
        .map(|(h, _)| h.to_string())
    else {
        return;
    };
    let groups: Vec<&str> = (0..8).map(|i| &hex[i * 4..i * 4 + 4]).collect();
    let addr = groups.join(":");
    assert!(
        addr.starts_with("fe80"),
        "parsed {addr} from a scope-20 row"
    );
    assert!(
        !is_local_addr(&addr),
        "{addr} is link-local: documented as remote"
    );
    assert!(
        !is_local_addr(&format!("{addr}%lo")),
        "a scoped literal does not parse"
    );
}
