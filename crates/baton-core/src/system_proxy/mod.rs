use std::fs;
use std::io;
use std::path::PathBuf;

use crate::config::{Hop, ProxyProtocol};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(windows)]
use windows as platform;

use platform::Snapshot;

const BYPASS: &[&str] = &[
    "localhost",
    "127.0.0.1",
    "::1",
    "*.local",
    "10.0.0.0/8",
    "172.16.0.0/12",
    "192.168.0.0/16",
    "169.254.0.0/16",
];

#[derive(Debug, thiserror::Error)]
pub enum SystemProxyError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("{command} failed: {message}")]
    Command { command: String, message: String },
    #[error("invalid backup: {0}")]
    Backup(#[from] serde_json::Error),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ProxyEndpoints {
    pub(crate) https: Option<(String, u16)>,
    pub(crate) socks: Option<(String, u16)>,
    pub(crate) http: Option<(String, u16)>,
}

impl ProxyEndpoints {
    pub(crate) fn preferred_hop(self) -> Option<Hop> {
        let (protocol, (host, port)) = self
            .https
            .map(|endpoint| (ProxyProtocol::Http, endpoint))
            .or_else(|| self.socks.map(|endpoint| (ProxyProtocol::Socks5, endpoint)))
            .or_else(|| self.http.map(|endpoint| (ProxyProtocol::Http, endpoint)))?;
        Some(Hop { protocol, host, port, ..Hop::default() })
    }
}

#[cfg(any(windows, test))]
fn parse_endpoint(value: &str) -> Option<(String, u16)> {
    let value = value.trim();
    let value = value.split_once("://").map_or(value, |(_, rest)| rest);
    let value = value.trim_end_matches('/');
    let (host, port) = value.rsplit_once(':')?;
    let host = host.trim_start_matches('[').trim_end_matches(']');
    let port = port.parse().ok().filter(|port| *port != 0)?;
    (!host.is_empty()).then(|| (host.to_owned(), port))
}

#[cfg(any(windows, test))]
fn parse_windows_proxy_server(value: &str) -> ProxyEndpoints {
    if !value.contains('=') {
        let endpoint = parse_endpoint(value);
        return ProxyEndpoints { https: endpoint.clone(), http: endpoint, socks: None };
    }
    let mut endpoints = ProxyEndpoints::default();
    for entry in value.split(';') {
        let Some((scheme, address)) = entry.split_once('=') else {
            continue;
        };
        let endpoint = parse_endpoint(address);
        match scheme.trim().to_ascii_lowercase().as_str() {
            "https" => endpoints.https = endpoint,
            "http" => endpoints.http = endpoint,
            "socks" | "socks5" => endpoints.socks = endpoint,
            _ => {}
        }
    }
    endpoints
}

pub fn current() -> Result<Option<Hop>, SystemProxyError> {
    platform::current()
}

#[derive(Clone, Debug)]
pub struct SystemProxy {
    backup_path: PathBuf,
}

impl SystemProxy {
    pub fn new(backup_path: PathBuf) -> Self {
        Self { backup_path }
    }

    pub fn is_applied(&self) -> bool {
        self.backup_path.exists()
    }

    pub fn apply(&self, host: &str, port: u16) -> Result<(), SystemProxyError> {
        if !self.is_applied() {
            if let Some(parent) = self.backup_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let snapshot = platform::snapshot()?;
            fs::write(&self.backup_path, serde_json::to_vec_pretty(&snapshot)?)?;
        }
        platform::apply(host, port, BYPASS)
    }

    pub fn original_proxy(&self) -> Result<Option<Hop>, SystemProxyError> {
        match self.backup()? {
            Some(snapshot) => Ok(platform::endpoints(&snapshot).preferred_hop()),
            None => current(),
        }
    }

    pub fn restore(&self) -> Result<(), SystemProxyError> {
        if let Some(snapshot) = self.backup()? {
            platform::restore(&snapshot)?;
            fs::remove_file(&self.backup_path)?;
        }
        Ok(())
    }

    fn backup(&self) -> Result<Option<Snapshot>, SystemProxyError> {
        match fs::read(&self.backup_path) {
            Ok(data) => Ok(Some(serde_json::from_slice(&data)?)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_endpoints() {
        assert_eq!(parse_endpoint("127.0.0.1:7890"), Some(("127.0.0.1".into(), 7890)));
        assert_eq!(parse_endpoint("http://proxy:8080/"), Some(("proxy".into(), 8080)));
        assert_eq!(parse_endpoint("[::1]:1080"), Some(("::1".into(), 1080)));
        assert_eq!(parse_endpoint("proxy"), None);
        assert_eq!(parse_endpoint("proxy:0"), None);
    }

    #[test]
    fn parses_windows_proxy_server() {
        let single = parse_windows_proxy_server("127.0.0.1:7890").preferred_hop().unwrap();
        assert_eq!((single.protocol, single.port), (ProxyProtocol::Http, 7890));

        let socks_only =
            parse_windows_proxy_server("socks=127.0.0.1:1080").preferred_hop().unwrap();
        assert_eq!((socks_only.protocol, socks_only.port), (ProxyProtocol::Socks5, 1080));

        let mixed =
            parse_windows_proxy_server("http=a:1;https=b:2;socks=c:3").preferred_hop().unwrap();
        assert_eq!((mixed.protocol, mixed.host.as_str()), (ProxyProtocol::Http, "b"));
    }
}
