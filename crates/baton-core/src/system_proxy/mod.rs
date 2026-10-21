use std::fs;
use std::io;
use std::path::PathBuf;

use crate::hop::{Hop, ProxyProtocol};

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
            None => platform::current(),
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
