//! LAN address discovery.

use std::net::{Ipv4Addr, UdpSocket};

/// The address the OS would use to reach the internet. A UDP `connect` sends no packet;
/// it only makes the OS pick the outgoing interface. Fails on a machine with no route.
fn default_route_ip() -> Option<Ipv4Addr> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("192.0.2.1:9").ok()?; // TEST-NET-1, never routed
    match sock.local_addr().ok()?.ip() {
        std::net::IpAddr::V4(ip) if !ip.is_unspecified() => Some(ip),
        _ => None,
    }
}

fn looks_virtual(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    [
        "vethernet", "virtual", "vmware", "vbox", "docker", "wsl", "hyper-v", "tailscale", "zerotier", "vpn",
        "tap", "tun", "utun", "br-", "veth", "loopback",
    ]
    .iter()
    .any(|t| n.contains(t))
}

/// Usable IPv4 addresses, best guess first: the default route, then private ranges on
/// physical-looking adapters, then everything else that is up.
pub fn lan_ips() -> Vec<Ipv4Addr> {
    let route = default_route_ip();
    let mut found: Vec<(u8, Ipv4Addr)> = Vec::new();
    if let Ok(ifaces) = if_addrs::get_if_addrs() {
        for i in ifaces {
            let if_addrs::IfAddr::V4(v4) = &i.addr else { continue };
            let ip = v4.ip;
            if ip.is_loopback() || ip.is_link_local() || ip.is_unspecified() || ip.is_multicast() {
                continue;
            }
            if i.oper_status != if_addrs::IfOperStatus::Up || i.is_p2p {
                continue;
            }
            let rank = if Some(ip) == route {
                0
            } else if looks_virtual(&i.name) {
                3
            } else if ip.is_private() {
                1
            } else {
                2
            };
            found.push((rank, ip));
        }
    }
    if let Some(r) = route
        && !found.iter().any(|(_, ip)| *ip == r)
    {
        found.push((0, r));
    }
    found.sort_by_key(|(rank, _)| *rank);
    let mut out: Vec<Ipv4Addr> = Vec::new();
    for (_, ip) in found {
        if !out.contains(&ip) {
            out.push(ip);
        }
    }
    out
}
