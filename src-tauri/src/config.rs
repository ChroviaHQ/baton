use std::fs;
use std::io;
use std::path::Path;

use baton_core::{Hop, ProxyKind};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpstreamMode {
    #[default]
    Manual,
    System,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    pub upstream_mode: UpstreamMode,
    pub upstream: Hop,
    pub landing: Hop,
    pub listen_port: u16,
    pub allow_lan: bool,
    pub system_proxy: bool,
    pub auto_connect: bool,
    pub show_in_dock: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            upstream_mode: UpstreamMode::Manual,
            upstream: Hop {
                kind: ProxyKind::Socks5,
                host: "127.0.0.1".into(),
                port: 7897,
                ..Hop::default()
            },
            landing: Hop::default(),
            listen_port: 6868,
            allow_lan: false,
            system_proxy: false,
            auto_connect: false,
            show_in_dock: false,
        }
    }
}

impl AppConfig {
    pub fn load(path: &Path) -> Self {
        match fs::read(path) {
            Ok(data) => serde_json::from_slice(&data).unwrap_or_else(|error| {
                tracing::warn!(%error, "config is invalid; using defaults");
                Self::default()
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Self::default(),
            Err(error) => {
                tracing::warn!(%error, "cannot read config; using defaults");
                Self::default()
            }
        }
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
