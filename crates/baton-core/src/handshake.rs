use std::io;
use std::net::SocketAddr;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::address::TargetAddr;
use crate::hop::{Hop, ProxyProtocol};

const MAX_RESPONSE_HEAD: usize = 8 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum HandshakeError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("proxy rejected the credentials")]
    AuthFailed,
    #[error("proxy requires credentials")]
    AuthRequired,
    #[error("HTTP {0}")]
    HttpStatus(u16),
    #[error("SOCKS5 reply {0:#04x}")]
    Socks5Reply(u8),
    #[error("proxy sent a malformed response")]
    Malformed,
    #[error("{0}")]
    InvalidInput(&'static str),
}

pub(crate) async fn connect_via<S>(
    stream: &mut S,
    hop: &Hop,
    target: &TargetAddr,
) -> Result<(), HandshakeError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    match hop.protocol {
        ProxyProtocol::Http => http_connect(stream, target, hop.credentials()).await,
        ProxyProtocol::Socks5 => socks5_connect(stream, target, hop.credentials()).await,
    }
}

async fn http_connect<S>(
    stream: &mut S,
    target: &TargetAddr,
    credentials: Option<(&str, &str)>,
) -> Result<(), HandshakeError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let authority = target.to_string();
    let mut request = format!("CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\n");
    if let Some((username, password)) = credentials {
        let token = STANDARD.encode(format!("{username}:{password}"));
        request.push_str(&format!("Proxy-Authorization: Basic {token}\r\n"));
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes()).await?;
    stream.flush().await?;

    let head = read_response_head(stream).await?;
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut response = httparse::Response::new(&mut headers);
    match response.parse(&head) {
        Ok(httparse::Status::Complete(_)) => {}
        _ => return Err(HandshakeError::Malformed),
    }
    match response.code {
        Some(code) if (200..300).contains(&code) => Ok(()),
        Some(407) if credentials.is_some() => Err(HandshakeError::AuthFailed),
        Some(407) => Err(HandshakeError::AuthRequired),
        Some(code) => Err(HandshakeError::HttpStatus(code)),
        None => Err(HandshakeError::Malformed),
    }
}

async fn read_response_head<S>(stream: &mut S) -> Result<Vec<u8>, HandshakeError>
where
    S: AsyncRead + Unpin,
{
    let mut head = Vec::with_capacity(256);
    loop {
        if head.len() >= MAX_RESPONSE_HEAD {
            return Err(HandshakeError::Malformed);
        }
        head.push(stream.read_u8().await?);
        if head.ends_with(b"\r\n\r\n") {
            return Ok(head);
        }
    }
}

async fn socks5_connect<S>(
    stream: &mut S,
    target: &TargetAddr,
    credentials: Option<(&str, &str)>,
) -> Result<(), HandshakeError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let greeting: &[u8] =
        if credentials.is_some() { &[0x05, 0x02, 0x00, 0x02] } else { &[0x05, 0x01, 0x00] };
    stream.write_all(greeting).await?;

    let mut choice = [0u8; 2];
    stream.read_exact(&mut choice).await?;
    if choice[0] != 0x05 {
        return Err(HandshakeError::Malformed);
    }
    match (choice[1], credentials) {
        (0x00, _) => {}
        (0x02, Some((username, password))) => {
            socks5_authenticate(stream, username, password).await?;
        }
        (0xff, None) => return Err(HandshakeError::AuthRequired),
        (0xff, Some(_)) => return Err(HandshakeError::AuthFailed),
        _ => return Err(HandshakeError::Malformed),
    }

    let mut request = vec![0x05, 0x01, 0x00];
    write_socks5_addr(&mut request, target)?;
    stream.write_all(&request).await?;

    let mut reply = [0u8; 4];
    stream.read_exact(&mut reply).await?;
    if reply[0] != 0x05 {
        return Err(HandshakeError::Malformed);
    }
    if reply[1] != 0x00 {
        return Err(HandshakeError::Socks5Reply(reply[1]));
    }
    let remaining = match reply[3] {
        0x01 => 4 + 2,
        0x04 => 16 + 2,
        0x03 => stream.read_u8().await? as usize + 2,
        _ => return Err(HandshakeError::Malformed),
    };
    let mut bound = vec![0u8; remaining];
    stream.read_exact(&mut bound).await?;
    Ok(())
}

async fn socks5_authenticate<S>(
    stream: &mut S,
    username: &str,
    password: &str,
) -> Result<(), HandshakeError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let username_len = u8::try_from(username.len())
        .map_err(|_| HandshakeError::InvalidInput("SOCKS5 username is too long"))?;
    let password_len = u8::try_from(password.len())
        .map_err(|_| HandshakeError::InvalidInput("SOCKS5 password is too long"))?;
    let mut request = Vec::with_capacity(3 + username.len() + password.len());
    request.push(0x01);
    request.push(username_len);
    request.extend_from_slice(username.as_bytes());
    request.push(password_len);
    request.extend_from_slice(password.as_bytes());
    stream.write_all(&request).await?;

    let mut status = [0u8; 2];
    stream.read_exact(&mut status).await?;
    if status[1] != 0x00 {
        return Err(HandshakeError::AuthFailed);
    }
    Ok(())
}

fn write_socks5_addr(buf: &mut Vec<u8>, target: &TargetAddr) -> Result<(), HandshakeError> {
    match target {
        TargetAddr::Ip(SocketAddr::V4(addr)) => {
            buf.push(0x01);
            buf.extend_from_slice(&addr.ip().octets());
        }
        TargetAddr::Ip(SocketAddr::V6(addr)) => {
            buf.push(0x04);
            buf.extend_from_slice(&addr.ip().octets());
        }
        TargetAddr::Domain(host, _) => {
            let len = u8::try_from(host.len())
                .map_err(|_| HandshakeError::InvalidInput("domain name is too long"))?;
            buf.push(0x03);
            buf.push(len);
            buf.extend_from_slice(host.as_bytes());
        }
    }
    buf.extend_from_slice(&target.port().to_be_bytes());
    Ok(())
}
