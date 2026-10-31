//! Is an IP literal held by one of this host's interfaces? Used by
//! `is_local_addr` so a machine declared by an address it holds (a static
//! overlay IP) dispatches locally instead of SSHing to itself.
//!
//! Scope, measured on Linux and claimed nowhere else:
//! - Linux only. Elsewhere the probe declines (SSH, the behaviour before it
//!   existed): its broadcast refusal rests on Linux `connect` semantics.
//! - IPv6 link-local (`fe80::…`) is NOT recognised: it cannot be bound without
//!   a scope id, and a scoped literal (`fe80::1%eth0`) does not parse as an
//!   `IpAddr`. Such a machine still goes over SSH — a false negative, never a
//!   false positive.
//! - "Held here" means every packet the controller sends to that address
//!   reaches this host, unless the controller itself redirects it (an OUTPUT
//!   DNAT rule). Local dispatch also bypasses everything SSH would have
//!   applied: an ssh-config remap of that address (`Host`/`HostName`/`Port`/
//!   `ProxyJump`) and the machine's `user` and `ssh_key`. That is the trust the
//!   loopback spellings and the `/etc/hostname` match already extend.

/// Is `addr` an IP literal held by one of this host's interfaces?
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
/// parsing:
///
/// - `bind` succeeds only for an address the kernel routes to this host
///   (EADDRNOTAVAIL otherwise) — but Linux also lets a UDP socket bind a
///   subnet's DIRECTED broadcast (192.168.1.255), which nothing holds;
/// - `connect` to the address from that socket refuses a broadcast destination
///   without SO_BROADCAST (EACCES), and succeeds for a held address.
///
/// Unspecified and multicast are excluded outright. With non-local bind
/// enabled every bind succeeds, so the probe cannot answer and declines.
pub(crate) fn ip_held_locally(ip: std::net::IpAddr, nonlocal_bind: bool) -> bool {
    if !cfg!(target_os = "linux") || nonlocal_bind || ip.is_unspecified() || ip.is_multicast() {
        return false;
    }
    std::net::UdpSocket::bind((ip, 0)).is_ok_and(|s| s.connect((ip, 9)).is_ok())
}

/// `net.ipv{4,6}.ip_nonlocal_bind`. Unreadable counts as enabled, so a host
/// that will not say declines the probe rather than being trusted.
fn nonlocal_bind_enabled(ip: std::net::IpAddr) -> bool {
    nonlocal_bind_read(std::fs::read_to_string(nonlocal_bind_path(ip)))
}

/// The sysctl that governs a bind of `ip` (already folded by `probe_target`).
pub(crate) fn nonlocal_bind_path(ip: std::net::IpAddr) -> &'static str {
    if ip.is_ipv4() {
        "/proc/sys/net/ipv4/ip_nonlocal_bind"
    } else {
        "/proc/sys/net/ipv6/ip_nonlocal_bind"
    }
}

/// A failed read counts as enabled: the probe declines.
pub(crate) fn nonlocal_bind_read(read: std::io::Result<String>) -> bool {
    read.map_or(true, |v| sysctl_enabled(&v))
}

/// A sysctl value reads enabled unless it is exactly `0`; a garbled value is
/// not `0`, so it declines the probe too.
pub(crate) fn sysctl_enabled(value: &str) -> bool {
    value.trim() != "0"
}
