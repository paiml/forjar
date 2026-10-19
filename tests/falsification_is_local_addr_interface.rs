//! A machine declared by an address one of THIS host's interfaces holds runs
//! on the local transport, not over SSH to itself.
//!
//! `is_local_addr` said "check if the address matches any local interface"
//! and only compared against the loopback spellings and /etc/hostname. A box
//! declared by its static overlay address (`addr: 10.42.0.20`) and applied on
//! itself was dispatched to SSH — connection refused on a host with no sshd.
//!
//! Dispatch is observed, not inferred: the test process exports a marker and
//! the script echoes it. The local transport inherits the environment; an SSH
//! session to the same host does not, so a marker that comes back proves the
//! script ran in-process-tree, whatever sshd the host happens to run.

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
    let mark = format!("forjar-iface-{}", std::process::id());
    std::env::set_var("FORJAR_IFACE_MARK", &mark);
    let out = exec_script(
        &machine_at(&ip.to_string()),
        "echo \"${FORJAR_IFACE_MARK:-unset}\"",
    )
    .unwrap_or_else(|e| panic!("addr {ip} (a local interface) was not run locally: {e}"));
    assert_eq!(
        out.stdout.trim(),
        mark,
        "addr {ip} ran, but not in this process tree (the marker did not survive)"
    );
}

#[test]
fn falsify_unowned_addr_stays_remote() {
    // TEST-NET-1 is never assigned: the fix must not widen "local" to it.
    assert!(!is_local_addr("192.0.2.1"));
    assert!(!is_local_addr("0.0.0.0"));
    assert!(!is_local_addr("224.0.0.1"));
    assert!(!is_local_addr("255.255.255.255"));
}
