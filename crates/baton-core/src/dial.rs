use std::io;
use std::time::Duration;

use tokio::net::TcpStream;

use crate::address::TargetAddr;
use crate::config::Hop;
use crate::handshake::{self, HandshakeError};

#[derive(Debug, thiserror::Error)]
pub enum DialError {
    #[error("no hops configured")]
    Empty,
    #[error("cannot reach hop 1: {0}")]
    Connect(#[source] io::Error),
    #[error("hop {} failed: {source}", .hop + 1)]
    Handshake {
        hop: usize,
        #[source]
        source: HandshakeError,
    },
    #[error("timed out while building the chain")]
    Timeout,
}

pub async fn dial(hops: &[Hop], target: &TargetAddr) -> Result<TcpStream, DialError> {
    let first = hops.first().ok_or(DialError::Empty)?;
    let mut stream = TcpStream::connect((first.host.trim(), first.port))
        .await
        .map_err(DialError::Connect)?;
    let _ = stream.set_nodelay(true);
    for (index, hop) in hops.iter().enumerate() {
        let next = hops
            .get(index + 1)
            .map_or_else(|| target.clone(), Hop::target);
        handshake::connect_via(&mut stream, hop, &next)
            .await
            .map_err(|source| DialError::Handshake { hop: index, source })?;
    }
    Ok(stream)
}

pub async fn dial_timeout(
    hops: &[Hop],
    target: &TargetAddr,
    timeout: Duration,
) -> Result<TcpStream, DialError> {
    tokio::time::timeout(timeout, dial(hops, target))
        .await
        .map_err(|_| DialError::Timeout)?
}
