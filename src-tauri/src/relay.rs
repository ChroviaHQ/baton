use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::path::{Path, PathBuf};
use std::time::Duration;

use baton_core::system_proxy::SystemProxy;
use baton_core::{Hop, Server, ServerConfig, StatsSnapshot};
use serde::Serialize;

use crate::config::{AppConfig, UpstreamMode};
use crate::error::CommandError;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub config: AppConfig,
    pub running: bool,
    pub lan_ip: Option<String>,
}

pub struct Relay {
    config_path: PathBuf,
    config: AppConfig,
    server: Option<Server>,
    system_proxy: SystemProxy,
}

impl Relay {
    pub fn new(data_dir: &Path) -> Self {
        let config_path = data_dir.join("config.json");
        Self {
            config: AppConfig::load(&config_path),
            config_path,
            server: None,
            system_proxy: SystemProxy::new(data_dir.join("system-proxy-backup.json")),
        }
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn status(&self) -> Status {
        Status {
            config: self.config.clone(),
            running: self.server.is_some(),
            lan_ip: lan_ip().map(|ip| ip.to_string()),
        }
    }

    pub fn stats(&self) -> Option<StatsSnapshot> {
        self.server.as_ref().map(Server::stats)
    }

    pub fn recover_system_proxy(&self) {
        if self.system_proxy.is_applied() {
            tracing::warn!("restoring system proxy left over from an unclean exit");
            if let Err(error) = self.system_proxy.restore() {
                tracing::error!(%error, "cannot restore system proxy");
            }
        }
    }

    pub async fn start(&mut self) -> Result<(), CommandError> {
        if self.server.is_none() {
            validate(&self.config, false)?;
            let hops = self.resolve_hops(&self.config)?;
            self.server = Some(start_server(&self.config, hops).await?);
        }
        self.reconcile_system_proxy(false).await
    }

    pub async fn stop(&mut self) -> Result<(), CommandError> {
        let server = self.server.take();
        let result = self.reconcile_system_proxy(false).await;
        if let Some(server) = server {
            server.stop().await;
        }
        result
    }

    pub async fn apply_config(&mut self, config: AppConfig) -> Result<(), CommandError> {
        let port_changed = config.listen_port != self.config.listen_port;
        let restart = self.server.is_some()
            && (port_changed
                || config.allow_lan != self.config.allow_lan
                || config.upstream_mode != self.config.upstream_mode
                || config.upstream != self.config.upstream
                || config.landing != self.config.landing);
        validate(&config, !restart)?;
        let hops = restart.then(|| self.resolve_hops(&config)).transpose()?;
        config
            .save(&self.config_path)
            .map_err(|error| CommandError::new("config_save_failed", error))?;
        self.config = config;
        if let Some(hops) = hops {
            if let Some(server) = self.server.take() {
                server.stop().await;
            }
            match start_server(&self.config, hops).await {
                Ok(server) => self.server = Some(server),
                Err(error) => {
                    let _ = self.reconcile_system_proxy(false).await;
                    return Err(error);
                }
            }
        }
        self.reconcile_system_proxy(port_changed).await
    }

    pub fn detect_upstream(&self, listen_port: u16) -> Result<Hop, CommandError> {
        let hop = self
            .system_proxy
            .original_proxy()
            .map_err(|error| CommandError::new("system_proxy_read_failed", error))?
            .ok_or_else(|| CommandError::new("system_proxy_not_set", "no system proxy"))?;
        if is_loopback(&hop.host) && hop.port == listen_port {
            return Err(CommandError::new("system_proxy_is_baton", "system proxy points to Baton"));
        }
        Ok(hop)
    }

    fn resolve_hops(&self, config: &AppConfig) -> Result<Vec<Hop>, CommandError> {
        let upstream = match config.upstream_mode {
            UpstreamMode::Manual => config.upstream.clone(),
            UpstreamMode::System => self.detect_upstream(config.listen_port)?,
        };
        Ok(vec![upstream, config.landing.clone()])
    }

    pub fn shutdown_blocking(&mut self) {
        if let Err(error) = self.system_proxy.restore() {
            tracing::error!(%error, "cannot restore system proxy on exit");
        }
        self.server = None;
    }

    async fn reconcile_system_proxy(&self, port_changed: bool) -> Result<(), CommandError> {
        let desired = self.server.is_some() && self.config.use_as_system_proxy;
        let active = self.system_proxy.is_applied();
        let system_proxy = self.system_proxy.clone();
        let port = self.config.listen_port;
        let task = if desired && (!active || port_changed) {
            tokio::task::spawn_blocking(move || system_proxy.apply("127.0.0.1", port))
        } else if !desired && active {
            tokio::task::spawn_blocking(move || system_proxy.restore())
        } else {
            return Ok(());
        };
        task.await
            .map_err(|error| CommandError::new("system_proxy_failed", error))?
            .map_err(CommandError::from)
    }
}

fn validate(config: &AppConfig, allow_unfilled: bool) -> Result<(), CommandError> {
    let hops = [&config.upstream, &config.landing];
    for (index, hop) in hops.into_iter().enumerate() {
        let skipped = (index == 0 && config.upstream_mode == UpstreamMode::System)
            || (allow_unfilled && hop.host.trim().is_empty() && hop.port == 0);
        if !skipped {
            hop.validate().map_err(|error| CommandError::invalid_hop(index, error))?;
        }
    }
    if config.listen_port == 0 {
        return Err(CommandError::new("invalid_listen_port", "listen port is zero"));
    }
    Ok(())
}

fn is_loopback(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

fn lan_ip() -> Option<IpAddr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect((Ipv4Addr::new(192, 0, 2, 1), 9)).ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_loopback() && !ip.is_unspecified()).then_some(ip)
}

async fn start_server(config: &AppConfig, hops: Vec<Hop>) -> Result<Server, CommandError> {
    Server::start(ServerConfig {
        listen: SocketAddr::from((
            if config.allow_lan { Ipv4Addr::UNSPECIFIED } else { Ipv4Addr::LOCALHOST },
            config.listen_port,
        )),
        hops,
        connect_timeout: CONNECT_TIMEOUT,
    })
    .await
    .map_err(|error| match error.kind() {
        io::ErrorKind::AddrInUse => CommandError::new("port_in_use", error),
        _ => CommandError::new("bind_failed", error),
    })
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;

    use baton_core::ProxyProtocol;

    use super::*;

    fn free_port() -> u16 {
        TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("baton-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn configured(dir: &Path) -> Relay {
        let mut relay = Relay::new(dir);
        relay.config.landing = Hop {
            protocol: ProxyProtocol::Http,
            host: "proxy.example.com".into(),
            port: 8080,
            ..Hop::default()
        };
        relay.config.listen_port = free_port();
        relay
    }

    #[tokio::test]
    async fn start_requires_a_landing_host() {
        let dir = temp_dir("validate");
        let mut relay = Relay::new(&dir);
        let error = relay.start().await.unwrap_err();
        assert_eq!(error.code, "empty_host");
        assert_eq!(error.hop, Some(1));
        assert!(!relay.status().running);
    }

    #[tokio::test]
    async fn update_persists_and_rebinds_on_port_change() {
        let dir = temp_dir("update");
        let mut relay = configured(&dir);
        relay.start().await.unwrap();
        let old_port = relay.config.listen_port;

        let mut config = relay.config.clone();
        config.listen_port = free_port();
        relay.apply_config(config.clone()).await.unwrap();

        assert!(relay.status().running);
        assert_eq!(relay.server.as_ref().unwrap().local_addr().port(), config.listen_port);
        TcpListener::bind(("127.0.0.1", old_port)).unwrap();
        assert_eq!(AppConfig::load(&dir.join("config.json")), config);

        relay.stop().await.unwrap();
        assert!(!relay.status().running);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn port_in_use_is_reported() {
        let dir = temp_dir("busy");
        let mut relay = configured(&dir);
        let blocker = TcpListener::bind("127.0.0.1:0").unwrap();
        relay.config.listen_port = blocker.local_addr().unwrap().port();
        let error = relay.start().await.unwrap_err();
        assert_eq!(error.code, "port_in_use");
        assert!(!relay.status().running);
    }

    #[tokio::test]
    async fn saving_allows_an_unfilled_landing_but_start_does_not() {
        let dir = temp_dir("partial");
        let mut relay = Relay::new(&dir);
        let mut config = relay.config.clone();
        config.upstream.port = 1080;
        relay.apply_config(config.clone()).await.unwrap();
        assert_eq!(AppConfig::load(&dir.join("config.json")), config);

        config.landing.host = "proxy.example.com".into();
        let error = relay.apply_config(config).await.unwrap_err();
        assert_eq!(error.code, "invalid_port");
        assert_eq!(error.hop, Some(1));

        let error = relay.start().await.unwrap_err();
        assert_eq!(error.code, "empty_host");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn running_relay_rejects_an_incomplete_change() {
        let dir = temp_dir("running");
        let mut relay = configured(&dir);
        relay.start().await.unwrap();
        let mut config = relay.config.clone();
        config.landing = Hop::default();
        let error = relay.apply_config(config).await.unwrap_err();
        assert_eq!(error.code, "empty_host");
        assert!(relay.status().running);
        assert_eq!(relay.config.landing.host, "proxy.example.com");
        relay.stop().await.unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn allow_lan_rebinds_on_all_interfaces() {
        let dir = temp_dir("lan");
        let mut relay = configured(&dir);
        relay.start().await.unwrap();
        assert!(relay.server.as_ref().unwrap().local_addr().ip().is_loopback());

        let mut config = relay.config.clone();
        config.allow_lan = true;
        relay.apply_config(config).await.unwrap();
        assert!(relay.server.as_ref().unwrap().local_addr().ip().is_unspecified());

        relay.stop().await.unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn system_mode_skips_upstream_validation() {
        let dir = temp_dir("system-mode");
        let mut relay = Relay::new(&dir);
        let mut config = relay.config.clone();
        config.upstream_mode = UpstreamMode::System;
        config.upstream = Hop::default();
        config.upstream.host = "half-filled".into();
        relay.apply_config(config).await.unwrap();
        let error = relay.start().await.unwrap_err();
        assert_eq!(error.hop, Some(1));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn recognizes_loopback_hosts() {
        assert!(is_loopback("127.0.0.1"));
        assert!(is_loopback("localhost"));
        assert!(is_loopback("[::1]"));
        assert!(!is_loopback("192.168.1.2"));
    }
}
