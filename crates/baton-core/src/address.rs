use std::fmt;
use std::net::{IpAddr, SocketAddr};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TargetAddr {
    Ip(SocketAddr),
    Domain(String, u16),
}

impl TargetAddr {
    pub fn new(host: &str, port: u16) -> Self {
        let bare = host.strip_prefix('[').and_then(|h| h.strip_suffix(']')).unwrap_or(host);
        match bare.parse::<IpAddr>() {
            Ok(ip) => Self::Ip(SocketAddr::new(ip, port)),
            Err(_) => Self::Domain(bare.to_owned(), port),
        }
    }

    pub fn parse_authority(authority: &str, default_port: u16) -> Option<Self> {
        let authority = authority.trim();
        if authority.is_empty() {
            return None;
        }
        if let Some(rest) = authority.strip_prefix('[') {
            let (host, tail) = rest.split_once(']')?;
            let port = match tail.strip_prefix(':') {
                Some(port) => port.parse().ok()?,
                None if tail.is_empty() => default_port,
                None => return None,
            };
            let ip = host.parse::<IpAddr>().ok()?;
            return Some(Self::Ip(SocketAddr::new(ip, port)));
        }
        match authority.rsplit_once(':') {
            Some((host, port)) if !host.contains(':') => {
                if host.is_empty() {
                    return None;
                }
                Some(Self::new(host, port.parse().ok()?))
            }
            Some(_) => None,
            None => Some(Self::new(authority, default_port)),
        }
    }

    pub fn port(&self) -> u16 {
        match self {
            Self::Ip(addr) => addr.port(),
            Self::Domain(_, port) => *port,
        }
    }
}

impl fmt::Display for TargetAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ip(addr) => write!(f, "{addr}"),
            Self::Domain(host, port) => write!(f, "{host}:{port}"),
        }
    }
}
