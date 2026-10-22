use std::net::IpAddr;

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
    entries: Vec<String>,
    domains: Vec<String>,
    networks: Vec<(u128, u128)>,
}

impl BypassList {
    pub fn parse(extra: &[String]) -> Result<Self, String> {
        let entries: Vec<String> = BUILTIN_ENTRIES
            .iter()
            .map(|&entry| entry.to_owned())
            .chain(extra.iter().cloned())
            .collect();
        let mut domains = Vec::new();
        let mut networks = Vec::new();
        for entry in &entries {
            if let Some(network) = parse_network(entry) {
                networks.push(network);
            } else if let Some(domain) = parse_domain(entry) {
                domains.push(domain);
            } else {
                return Err(entry.clone());
            }
        }
        Ok(Self { entries, domains, networks })
    }

    pub(crate) fn entries(&self) -> &[String] {
        &self.entries
    }

    pub(crate) fn matches(&self, target: &TargetAddr) -> bool {
        match target {
            TargetAddr::Ip(addr) => {
                let bits = ip_bits(addr.ip().to_canonical());
                self.networks.iter().any(|&(network, mask)| bits & mask == network)
            }
            TargetAddr::Domain(host, _) => {
                let host = host.trim_end_matches('.').to_ascii_lowercase();
                self.domains.iter().any(|domain| {
                    host == *domain
                        || host
                            .strip_suffix(domain.as_str())
                            .is_some_and(|rest| rest.ends_with('.'))
                })
            }
        }
    }
}

fn parse_network(entry: &str) -> Option<(u128, u128)> {
    let (ip, len) = match entry.split_once('/') {
        Some((ip, len)) => (ip.parse::<IpAddr>().ok()?, Some(len.parse::<u32>().ok()?)),
        None => match TargetAddr::new(entry, 0) {
            TargetAddr::Ip(addr) => (addr.ip(), None),
            TargetAddr::Domain(..) => return None,
        },
    };
    let width = if ip.is_ipv4() { 32 } else { 128 };
    let len = len.unwrap_or(width);
    if len > width {
        return None;
    }
    let mask = u128::MAX.checked_shl(width - len).unwrap_or(0);
    Some((ip_bits(ip) & mask, mask))
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
