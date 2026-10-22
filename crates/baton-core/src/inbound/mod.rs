use std::io;
use std::sync::Arc;

use tokio::net::TcpStream;

use crate::address::TargetAddr;
use crate::dial::{DialError, dial, dial_direct};
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

pub(crate) async fn serve(client: TcpStream, shared: Arc<ServerState>) -> Result<(), InboundError> {
    let _ = client.set_nodelay(true);
    let mut first = [0u8; 1];
    if client.peek(&mut first).await? == 0 {
        return Ok(());
    }
    if first[0] == 0x05 {
        socks5::serve(client, shared).await
    } else {
        http::serve(client, shared).await
    }
}

async fn connect_target(shared: &ServerState, target: &TargetAddr) -> Result<TcpStream, DialError> {
    let stream = async {
        if shared.bypass.matches(target) {
            dial_direct(target).await
        } else {
            dial(&shared.hops, target).await
        }
    };
    tokio::time::timeout(shared.connect_timeout, stream).await.map_err(|_| DialError::Timeout)?
}
