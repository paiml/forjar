//! Is an IP literal held by one of this host's interfaces? Used by
//! `is_local_addr` so a machine declared by an address it holds (a static
//! overlay IP) dispatches locally instead of SSHing to itself.

/// Does a local interface hold `ip`? Asked of the kernel by binding to it:
/// `bind` succeeds only for an assigned address (EADDRNOTAVAIL otherwise),
/// with no interface parsing and on every platform forjar runs on.
///
/// Unspecified, multicast and broadcast bind without being owned, so they are
/// excluded. With non-local bind enabled every bind succeeds, so the probe
/// cannot answer and declines: SSH, the behaviour before this probe existed.
pub(crate) fn ip_held_locally(ip: std::net::IpAddr, nonlocal_bind: bool) -> bool {
    use std::net::IpAddr;
    if nonlocal_bind || ip.is_unspecified() || ip.is_multicast() {
        return false;
    }
    if let IpAddr::V4(v4) = ip {
        if v4.is_broadcast() {
            return false;
        }
    }
    std::net::UdpSocket::bind((ip, 0)).is_ok()
}

/// `net.ipv{4,6}.ip_nonlocal_bind` on Linux; absent (false) elsewhere.
pub(crate) fn nonlocal_bind_enabled(ip: std::net::IpAddr) -> bool {
    let path = if ip.is_ipv4() {
        "/proc/sys/net/ipv4/ip_nonlocal_bind"
    } else {
        "/proc/sys/net/ipv6/ip_nonlocal_bind"
    };
    std::fs::read_to_string(path).is_ok_and(|v| v.trim() != "0")
}
