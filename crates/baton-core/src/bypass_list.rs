use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use crate::address::TargetAddr;

const BUILTIN_ENTRIES: &[&str] = &[
    "localhost",
    "127.0.0.0/8",
    "::1",
    "*.local",
    "10.0.0.0/8",
    "172.16.0.0/12",
    "192.168.0.0/16",
    "169.254.0.0/16",
];

#[derive(Debug)]
pub struct BypassList {
    domains: Vec<String>,
    networks: Vec<Network>,
}

impl BypassList {
    pub fn parse(extra: &[String]) -> Result<Self, String> {
        let mut domains: Vec<String> = Vec::new();
        let mut networks: Vec<Network> = Vec::new();
        for entry in BUILTIN_ENTRIES.iter().copied().chain(extra.iter().map(String::as_str)) {
            if let Some(network) = Network::parse(entry) {
                if !networks.iter().any(|known| known.contains(&network)) {
                    networks.retain(|known| !network.contains(known));
                    networks.push(network);
                }
            } else if let Some(domain) = parse_domain(entry) {
                if !domains.iter().any(|known| domain_covers(known, &domain)) {
                    domains.retain(|known| !domain_covers(&domain, known));
                    domains.push(domain);
                }
            } else {
                return Err(entry.to_owned());
            }
        }
        Ok(Self { domains, networks })
    }

    pub(crate) fn domains(&self) -> &[String] {
        &self.domains
    }

    pub(crate) fn networks(&self) -> &[Network] {
        &self.networks
    }

    pub(crate) fn matches(&self, target: &TargetAddr) -> bool {
        match target {
            TargetAddr::Ip(addr) => {
                let bits = ip_bits(addr.ip().to_canonical());
                self.networks.iter().any(|network| bits & network.mask == network.bits)
            }
            TargetAddr::Domain(host, _) => {
                let host = host.trim_end_matches('.').to_ascii_lowercase();
                self.domains.iter().any(|domain| domain_covers(domain, &host))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Network {
    ip: IpAddr,
    prefix_len: u32,
    bits: u128,
    mask: u128,
}

impl Network {
    fn parse(entry: &str) -> Option<Self> {
        let (ip, prefix_len) = match entry.split_once('/') {
            Some((ip, prefix_len)) => {
                (ip.parse::<IpAddr>().ok()?, Some(prefix_len.parse::<u32>().ok()?))
            }
            None => match TargetAddr::new(entry, 0) {
                TargetAddr::Ip(addr) => (addr.ip(), None),
                TargetAddr::Domain(..) => return None,
            },
        };
        let width = if ip.is_ipv4() { 32 } else { 128 };
        let prefix_len = prefix_len.unwrap_or(width);
        if prefix_len > width {
            return None;
        }
        let mask = u128::MAX.checked_shl(width - prefix_len).unwrap_or(0);
        let bits = ip_bits(ip) & mask;
        let ip = match ip {
            IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::from_bits(bits as u32)),
            IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::from_bits(bits)),
        };
        Some(Self { ip, prefix_len, bits, mask })
    }

    pub(crate) fn ip(&self) -> IpAddr {
        self.ip
    }

    pub(crate) fn prefix_len(&self) -> u32 {
        self.prefix_len
    }

    pub(crate) fn is_host(&self) -> bool {
        self.mask == u128::MAX
    }

    fn contains(&self, other: &Network) -> bool {
        self.ip.is_ipv4() == other.ip.is_ipv4()
            && other.mask & self.mask == self.mask
            && other.bits & self.mask == self.bits
    }
}

fn domain_covers(domain: &str, host: &str) -> bool {
    host == domain || host.strip_suffix(domain).is_some_and(|rest| rest.ends_with('.'))
}

fn parse_domain(entry: &str) -> Option<String> {
    let domain = entry
        .trim_start_matches("*.")
        .trim_start_matches('.')
        .trim_end_matches('.')
        .to_ascii_lowercase();
    let valid = !domain.is_empty()
        && domain.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.');
    valid.then_some(domain)
}

fn ip_bits(ip: IpAddr) -> u128 {
    match ip {
        IpAddr::V4(ip) => u128::from(ip.to_ipv6_mapped()),
        IpAddr::V6(ip) => u128::from(ip),
    }
}
