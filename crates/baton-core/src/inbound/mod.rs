use std::io;
use std::sync::Arc;

use tokio::net::TcpStream;

use crate::address::TargetAddr;
use crate::dial::{DialError, dial};
use crate::server::ServerState;

mod http;
mod socks5;

#[derive(Debug, thiserror::Error)]
pub(crate) enum InboundError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("{0}")]
    Protocol(&'static str),
    #[error("{target}: {source}")]
    Dial {
        target: TargetAddr,
        #[source]
        source: DialError,
    },
}

pub(crate) async fn serve(client: TcpStream, state: Arc<ServerState>) -> Result<(), InboundError> {
    let _ = client.set_nodelay(true);
    let mut first = [0u8; 1];
    if client.peek(&mut first).await? == 0 {
        return Ok(());
    }
    if first[0] == 0x05 {
        socks5::serve(client, state).await
    } else {
        http::serve(client, state).await
    }
}

async fn connect_target(state: &ServerState, target: &TargetAddr) -> Result<TcpStream, DialError> {
    let hops = if state.bypass.matches(target) { &[][..] } else { &state.hops[..] };
    tokio::time::timeout(state.connect_timeout, dial(hops, target))
        .await
        .map_err(|_| DialError::Timeout)?
}
