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

    pub fn host(&self) -> String {
        match self {
            Self::Ip(addr) => addr.ip().to_string(),
            Self::Domain(host, _) => host.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_authorities() {
        assert_eq!(
            TargetAddr::parse_authority("example.com:8443", 443),
            Some(TargetAddr::Domain("example.com".into(), 8443))
        );
        assert_eq!(
            TargetAddr::parse_authority("example.com", 80),
            Some(TargetAddr::Domain("example.com".into(), 80))
        );
        assert_eq!(
            TargetAddr::parse_authority("1.2.3.4:5", 80),
            Some(TargetAddr::Ip("1.2.3.4:5".parse().unwrap()))
        );
        assert_eq!(
            TargetAddr::parse_authority("[::1]:8080", 80),
            Some(TargetAddr::Ip("[::1]:8080".parse().unwrap()))
        );
        assert_eq!(
            TargetAddr::parse_authority("[::1]", 443),
            Some(TargetAddr::Ip("[::1]:443".parse().unwrap()))
        );
        assert_eq!(TargetAddr::parse_authority("::1:80", 80), None);
        assert_eq!(TargetAddr::parse_authority(":80", 80), None);
        assert_eq!(TargetAddr::parse_authority("host:http", 80), None);
    }

    #[test]
    fn displays_ipv6_with_brackets() {
        assert_eq!(TargetAddr::new("::1", 80).to_string(), "[::1]:80");
        assert_eq!(TargetAddr::new("[::1]", 80).to_string(), "[::1]:80");
    }
}
