use std::io;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::{InboundError, connect_target};
use crate::address::TargetAddr;
use crate::dial::DialError;
use crate::handshake::HandshakeError;
use crate::server::ServerState;
use crate::tunnel::tunnel;

pub(super) async fn serve(
    mut client: TcpStream,
    state: Arc<ServerState>,
) -> Result<(), InboundError> {
    let mut header = [0u8; 2];
    client.read_exact(&mut header).await?;
    let mut methods = vec![0u8; header[1] as usize];
    client.read_exact(&mut methods).await?;
    if !methods.contains(&0x00) {
        client.write_all(&[0x05, 0xff]).await?;
        return Err(InboundError::Protocol("SOCKS5 client offered no usable method"));
    }
    client.write_all(&[0x05, 0x00]).await?;

    let mut request = [0u8; 4];
    client.read_exact(&mut request).await?;
    if request[0] != 0x05 {
        return Err(InboundError::Protocol("malformed SOCKS5 request"));
    }
    let target = match read_addr(&mut client, request[3]).await? {
        Some(target) => target,
        None => {
            reply(&mut client, 0x08).await?;
            return Err(InboundError::Protocol("unsupported SOCKS5 address type"));
        }
    };
    if request[1] != 0x01 {
        reply(&mut client, 0x07).await?;
        return Err(InboundError::Protocol("only SOCKS5 CONNECT is supported"));
    }

    match connect_target(&state, &target).await {
        Ok(remote) => {
            reply(&mut client, 0x00).await?;
            Ok(tunnel(client, remote, &state.counters, &[]).await?)
        }
        Err(source) => {
            reply(&mut client, dial_error_code(&source, state.hops.len())).await?;
            Err(InboundError::Dial { target, source })
        }
    }
}

async fn read_addr(client: &mut TcpStream, atyp: u8) -> Result<Option<TargetAddr>, InboundError> {
    let target = match atyp {
        0x01 => {
            let mut ip = [0u8; 4];
            client.read_exact(&mut ip).await?;
            let port = client.read_u16().await?;
            TargetAddr::Ip(SocketAddr::new(Ipv4Addr::from(ip).into(), port))
        }
        0x04 => {
            let mut ip = [0u8; 16];
            client.read_exact(&mut ip).await?;
            let port = client.read_u16().await?;
            TargetAddr::Ip(SocketAddr::new(Ipv6Addr::from(ip).into(), port))
        }
        0x03 => {
            let len = client.read_u8().await? as usize;
            let mut host = vec![0u8; len];
            client.read_exact(&mut host).await?;
            let port = client.read_u16().await?;
            let host = String::from_utf8(host)
                .map_err(|_| InboundError::Protocol("SOCKS5 domain is not UTF-8"))?;
            TargetAddr::new(&host, port)
        }
        _ => return Ok(None),
    };
    Ok(Some(target))
}

async fn reply(client: &mut TcpStream, code: u8) -> io::Result<()> {
    client.write_all(&[0x05, code, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await
}

fn dial_error_code(error: &DialError, hop_count: usize) -> u8 {
    match error {
        DialError::Handshake { hop, source: HandshakeError::Socks5Reply(code) }
            if *hop + 1 == hop_count =>
        {
            *code
        }
        DialError::TargetUnreachable(error) if error.kind() == io::ErrorKind::ConnectionRefused => {
            0x05
        }
        DialError::Timeout | DialError::TargetUnreachable(_) => 0x04,
        DialError::HopUnreachable(_) | DialError::Handshake { .. } | DialError::EmptyChain => 0x01,
    }
}
