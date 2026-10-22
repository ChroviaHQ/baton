use std::io;

use tokio::net::TcpStream;

use crate::address::TargetAddr;
use crate::handshake::{self, HandshakeError};
use crate::hop::Hop;

#[derive(Debug, thiserror::Error)]
pub enum DialError {
    #[error("no hops configured")]
    NoHops,
    #[error("cannot reach hop 1: {0}")]
    Unreachable(#[source] io::Error),
    #[error("hop {} failed: {source}", .hop + 1)]
    Handshake {
        hop: usize,
        #[source]
        source: HandshakeError,
    },
    #[error("timed out while building the chain")]
    Timeout,
    #[error("cannot reach the target directly: {0}")]
    DirectUnreachable(#[source] io::Error),
}

pub async fn dial(hops: &[Hop], target: &TargetAddr) -> Result<TcpStream, DialError> {
    let first = hops.first().ok_or(DialError::NoHops)?;
    let mut stream = connect(&first.address()).await.map_err(DialError::Unreachable)?;
    for (index, hop) in hops.iter().enumerate() {
        let next = hops.get(index + 1).map_or_else(|| target.clone(), Hop::address);
        handshake::connect_via(&mut stream, hop, &next)
            .await
            .map_err(|source| DialError::Handshake { hop: index, source })?;
    }
    Ok(stream)
}

pub async fn dial_direct(target: &TargetAddr) -> Result<TcpStream, DialError> {
    connect(target).await.map_err(DialError::DirectUnreachable)
}

async fn connect(target: &TargetAddr) -> io::Result<TcpStream> {
    let stream = match target {
        TargetAddr::Ip(addr) => TcpStream::connect(addr).await,
        TargetAddr::Domain(host, port) => TcpStream::connect((host.as_str(), *port)).await,
    }?;
    let _ = stream.set_nodelay(true);
    Ok(stream)
}
