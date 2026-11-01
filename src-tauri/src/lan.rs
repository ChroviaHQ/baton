use std::net::{IpAddr, Ipv4Addr, UdpSocket};

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanAddress {
    pub interface: String,
    pub ip: Ipv4Addr,
}

pub fn lan_addresses() -> Vec<LanAddress> {
    let default_route_ip = default_route_ip();
    let mut addresses: Vec<LanAddress> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|interface| interface.is_oper_up() && !interface.is_p2p())
        .filter_map(|interface| match interface.ip() {
            IpAddr::V4(ip) if ip.is_private() => Some(LanAddress { interface: interface.name, ip }),
            _ => None,
        })
        .collect();
    addresses.sort_by_key(|address| Some(address.ip) != default_route_ip);
    addresses
}

fn default_route_ip() -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect((Ipv4Addr::new(192, 0, 2, 1), 9)).ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(ip) => Some(ip),
        IpAddr::V6(_) => None,
    }
}
