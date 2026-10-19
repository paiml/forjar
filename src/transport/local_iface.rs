//! Is an IP literal held by one of this host's interfaces? Used by
//! `is_local_addr` so a machine declared by an address it holds (a static
//! overlay IP) dispatches locally instead of SSHing to itself.

/// Is `addr` an IP literal held by one of this host's interfaces?
/// "Held here" is "this host" for every packet the controller sends to it,
/// unless the controller itself redirects that address (an OUTPUT DNAT rule):
/// the same trust the `/etc/hostname` match in `is_local_addr` extends.
/// IPv4-mapped IPv6 (`::ffff:a.b.c.d`) is judged as the IPv4 address it is,
/// so the IPv4 sysctl governs it, as it governs the kernel's bind.
pub(crate) fn addr_held_locally(addr: &str) -> bool {
    probe_target(addr).is_some_and(|ip| ip_held_locally(ip, nonlocal_bind_enabled(ip)))
}

/// The address the probe judges: the parsed literal, IPv4-mapped IPv6 folded
/// to IPv4. `None` for anything that is not an IP literal.
pub(crate) fn probe_target(addr: &str) -> Option<std::net::IpAddr> {
    addr.parse::<std::net::IpAddr>()
        .ok()
        .map(|ip| ip.to_canonical())
}

/// Does a local interface hold `ip`? Asked of the kernel, with no interface
/// parsing and on every platform forjar runs on:
///
/// - `bind` succeeds only for an address the kernel routes to this host
///   (EADDRNOTAVAIL otherwise) — but Linux also lets a UDP socket bind a
///   subnet's DIRECTED broadcast (192.168.1.255), which nothing holds;
/// - `connect` to the address from that socket refuses a broadcast destination
///   without SO_BROADCAST (EACCES), and succeeds for a held address.
///
/// Unspecified and multicast are excluded outright. With non-local bind
/// enabled every bind succeeds, so the probe cannot answer and declines:
/// SSH, the behaviour before this probe existed.
pub(crate) fn ip_held_locally(ip: std::net::IpAddr, nonlocal_bind: bool) -> bool {
    if nonlocal_bind || ip.is_unspecified() || ip.is_multicast() {
        return false;
    }
    std::net::UdpSocket::bind((ip, 0)).is_ok_and(|s| s.connect((ip, 9)).is_ok())
}

/// `net.ipv{4,6}.ip_nonlocal_bind` on Linux; absent (false) elsewhere.
fn nonlocal_bind_enabled(ip: std::net::IpAddr) -> bool {
    let path = if ip.is_ipv4() {
        "/proc/sys/net/ipv4/ip_nonlocal_bind"
    } else {
        "/proc/sys/net/ipv6/ip_nonlocal_bind"
    };
    std::fs::read_to_string(path).is_ok_and(|v| sysctl_enabled(&v))
}

/// A sysctl value reads enabled unless it is exactly `0`. An unreadable or
/// garbled value is NOT `0`, so it declines the probe rather than trusting it.
pub(crate) fn sysctl_enabled(value: &str) -> bool {
    value.trim() != "0"
}
