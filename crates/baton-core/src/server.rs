use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::Serialize;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::config::Hop;
use crate::inbound;

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub listen: SocketAddr,
    pub hops: Vec<Hop>,
    pub connect_timeout: Duration,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsSnapshot {
    pub active: u64,
    pub total: u64,
    pub failed: u64,
    pub up_bytes: u64,
    pub down_bytes: u64,
}

#[derive(Default)]
pub(crate) struct Stats {
    active: AtomicU64,
    total: AtomicU64,
    failed: AtomicU64,
    pub(crate) up: AtomicU64,
    pub(crate) down: AtomicU64,
}

pub(crate) struct Shared {
    pub(crate) hops: Vec<Hop>,
    pub(crate) connect_timeout: Duration,
    pub(crate) stats: Stats,
}

pub struct Server {
    local_addr: SocketAddr,
    shared: Arc<Shared>,
    cancel: CancellationToken,
    task: Option<JoinHandle<()>>,
}

impl Server {
    pub async fn start(config: ServerConfig) -> io::Result<Self> {
        let listener = TcpListener::bind(config.listen).await?;
        let local_addr = listener.local_addr()?;
        let shared = Arc::new(Shared {
            hops: config.hops,
            connect_timeout: config.connect_timeout,
            stats: Stats::default(),
        });
        let cancel = CancellationToken::new();
        let task = tokio::spawn(accept_loop(listener, shared.clone(), cancel.clone()));
        tracing::info!(%local_addr, "listening");
        Ok(Self {
            local_addr,
            shared,
            cancel,
            task: Some(task),
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn stats(&self) -> StatsSnapshot {
        let stats = &self.shared.stats;
        StatsSnapshot {
            active: stats.active.load(Ordering::Relaxed),
            total: stats.total.load(Ordering::Relaxed),
            failed: stats.failed.load(Ordering::Relaxed),
            up_bytes: stats.up.load(Ordering::Relaxed),
            down_bytes: stats.down.load(Ordering::Relaxed),
        }
    }

    pub async fn stop(mut self) {
        self.cancel.cancel();
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

async fn accept_loop(listener: TcpListener, shared: Arc<Shared>, cancel: CancellationToken) {
    loop {
        let accepted = tokio::select! {
            _ = cancel.cancelled() => return,
            accepted = listener.accept() => accepted,
        };
        let (client, peer) = match accepted {
            Ok(accepted) => accepted,
            Err(error) => {
                tracing::warn!(%error, "accept failed");
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        let shared = shared.clone();
        let cancel = cancel.child_token();
        tokio::spawn(async move {
            let stats = &shared.stats;
            stats.total.fetch_add(1, Ordering::Relaxed);
            stats.active.fetch_add(1, Ordering::Relaxed);
            let result = tokio::select! {
                _ = cancel.cancelled() => Ok(()),
                result = inbound::handle(client, shared.clone()) => result,
            };
            stats.active.fetch_sub(1, Ordering::Relaxed);
            if let Err(error) = result {
                if matches!(error, inbound::InboundError::Dial { .. }) {
                    stats.failed.fetch_add(1, Ordering::Relaxed);
                    tracing::warn!(%peer, %error, "connection failed");
                } else {
                    tracing::debug!(%peer, %error, "connection closed");
                }
            }
        });
    }
}
