use std::fs;
use std::io;
use std::path::Path;

use baton_core::{Hop, ProxyProtocol};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpstreamMode {
    #[default]
    Manual,
    System,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub upstream_mode: UpstreamMode,
    pub upstream: Hop,
    pub landing: Hop,
    pub listen_port: u16,
    pub allow_lan: bool,
    pub use_as_system_proxy: bool,
    pub start_on_launch: bool,
    pub show_in_dock: bool,
    pub bypass: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            upstream_mode: UpstreamMode::Manual,
            upstream: Hop {
                protocol: ProxyProtocol::Socks5,
                host: "127.0.0.1".into(),
                port: 7897,
                ..Hop::default()
            },
            landing: Hop::default(),
            listen_port: 1080,
            allow_lan: false,
            use_as_system_proxy: false,
            start_on_launch: false,
            show_in_dock: false,
            bypass: Vec::new(),
        }
    }
}

impl AppConfig {
    pub fn load(path: &Path) -> Self {
        let loaded =
            fs::read(path).and_then(|data| serde_json::from_slice(&data).map_err(io::Error::from));
        loaded.unwrap_or_else(|error| {
            if error.kind() != io::ErrorKind::NotFound {
                tracing::warn!(%error, "cannot load config; using defaults");
            }
            Self::default()
        })
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temp = path.with_extension("json.tmp");
        fs::write(&temp, serde_json::to_vec_pretty(self)?)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&temp, fs::Permissions::from_mode(0o600))?;
        }
        fs::rename(temp, path)
    }
}
