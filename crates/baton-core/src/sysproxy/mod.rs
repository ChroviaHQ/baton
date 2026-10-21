use std::fs;
use std::io;
use std::path::PathBuf;

use crate::config::{Hop, ProxyKind};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(windows)]
use windows as platform;

pub use platform::Snapshot;

pub const BYPASS: &[&str] = &[
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
pub enum SysProxyError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("{command} failed: {message}")]
    Command { command: String, message: String },
    #[error("invalid backup: {0}")]
    Backup(#[from] serde_json::Error),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Endpoints {
    pub(crate) https: Option<(String, u16)>,
    pub(crate) socks: Option<(String, u16)>,
    pub(crate) http: Option<(String, u16)>,
}

impl Endpoints {
    pub(crate) fn into_hop(self) -> Option<Hop> {
        let (kind, (host, port)) = self
            .https
            .map(|endpoint| (ProxyKind::Http, endpoint))
            .or_else(|| self.socks.map(|endpoint| (ProxyKind::Socks5, endpoint)))
            .or_else(|| self.http.map(|endpoint| (ProxyKind::Http, endpoint)))?;
        Some(Hop {
            kind,
            host,
            port,
            ..Hop::default()
        })
    }
}

pub(crate) fn parse_endpoint(value: &str) -> Option<(String, u16)> {
    let value = value.trim();
    let value = value.split_once("://").map_or(value, |(_, rest)| rest);
    let value = value.trim_end_matches('/');
    let (host, port) = value.rsplit_once(':')?;
    let host = host.trim_start_matches('[').trim_end_matches(']');
    let port = port.parse().ok().filter(|port| *port != 0)?;
    (!host.is_empty()).then(|| (host.to_owned(), port))
}

#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn parse_windows_proxy_server(value: &str) -> Endpoints {
    if !value.contains('=') {
        let endpoint = parse_endpoint(value);
        return Endpoints {
            https: endpoint.clone(),
            http: endpoint,
            socks: None,
        };
    }
    let mut endpoints = Endpoints::default();
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

pub fn current() -> Result<Option<Hop>, SysProxyError> {
    platform::current()
}

pub fn snapshot() -> Result<Snapshot, SysProxyError> {
    platform::snapshot()
}

pub fn apply(host: &str, port: u16) -> Result<(), SysProxyError> {
    platform::apply(host, port, BYPASS)
}

pub fn restore(snapshot: &Snapshot) -> Result<(), SysProxyError> {
    platform::restore(snapshot)
}

#[derive(Clone, Debug)]
pub struct SystemProxy {
    backup_path: PathBuf,
}

impl SystemProxy {
    pub fn new(backup_path: PathBuf) -> Self {
        Self { backup_path }
    }

    pub fn is_enabled(&self) -> bool {
        self.backup_path.exists()
    }

    pub fn enable(&self, host: &str, port: u16) -> Result<(), SysProxyError> {
        if !self.is_enabled() {
            let snapshot = snapshot()?;
            if let Some(parent) = self.backup_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&self.backup_path, serde_json::to_vec_pretty(&snapshot)?)?;
        }
        apply(host, port)
    }

    pub fn original(&self) -> Result<Option<Hop>, SysProxyError> {
        match fs::read(&self.backup_path) {
            Ok(data) => {
                let snapshot: Snapshot = serde_json::from_slice(&data)?;
                Ok(platform::endpoints(&snapshot).into_hop())
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => current(),
            Err(error) => Err(error.into()),
        }
    }

    pub fn disable(&self) -> Result<(), SysProxyError> {
        let data = match fs::read(&self.backup_path) {
            Ok(data) => data,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        let snapshot: Snapshot = serde_json::from_slice(&data)?;
        restore(&snapshot)?;
        fs::remove_file(&self.backup_path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_endpoints() {
        assert_eq!(
            parse_endpoint("127.0.0.1:7890"),
            Some(("127.0.0.1".into(), 7890))
        );
        assert_eq!(
            parse_endpoint("http://proxy:8080/"),
            Some(("proxy".into(), 8080))
        );
        assert_eq!(parse_endpoint("[::1]:1080"), Some(("::1".into(), 1080)));
        assert_eq!(parse_endpoint("proxy"), None);
        assert_eq!(parse_endpoint("proxy:0"), None);
    }

    #[test]
    fn parses_windows_proxy_server() {
        let single = parse_windows_proxy_server("127.0.0.1:7890")
            .into_hop()
            .unwrap();
        assert_eq!((single.kind, single.port), (ProxyKind::Http, 7890));

        let socks_only = parse_windows_proxy_server("socks=127.0.0.1:1080")
            .into_hop()
            .unwrap();
        assert_eq!(
            (socks_only.kind, socks_only.port),
            (ProxyKind::Socks5, 1080)
        );

        let mixed = parse_windows_proxy_server("http=a:1;https=b:2;socks=c:3")
            .into_hop()
            .unwrap();
        assert_eq!((mixed.kind, mixed.host.as_str()), (ProxyKind::Http, "b"));
    }
}
