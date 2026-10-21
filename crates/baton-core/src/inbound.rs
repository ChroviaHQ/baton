use std::io;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::TcpStream;

use crate::address::TargetAddr;
use crate::dial::{DialError, dial_timeout};
use crate::handshake::HandshakeError;
use crate::server::ServerState;

const MAX_REQUEST_HEAD: usize = 64 * 1024;
const HOP_BY_HOP: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authorization",
    "proxy-connection",
    "te",
    "trailer",
    "upgrade",
];

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
        serve_socks5(client, shared).await
    } else {
        serve_http(client, shared).await
    }
}

async fn serve_socks5(mut client: TcpStream, shared: Arc<ServerState>) -> Result<(), InboundError> {
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
    let target = match read_socks5_addr(&mut client, request[3]).await? {
        Some(target) => target,
        None => {
            socks5_reply(&mut client, 0x08).await?;
            return Err(InboundError::Protocol("unsupported SOCKS5 address type"));
        }
    };
    if request[1] != 0x01 {
        socks5_reply(&mut client, 0x07).await?;
        return Err(InboundError::Protocol("only SOCKS5 CONNECT is supported"));
    }

    match dial_timeout(&shared.hops, &target, shared.connect_timeout).await {
        Ok(remote) => {
            socks5_reply(&mut client, 0x00).await?;
            tunnel(client, remote, &shared, &[]).await
        }
        Err(source) => {
            socks5_reply(&mut client, socks5_code(&source, shared.hops.len())).await?;
            Err(InboundError::Dial { target, source })
        }
    }
}

async fn read_socks5_addr(
    client: &mut TcpStream,
    atyp: u8,
) -> Result<Option<TargetAddr>, InboundError> {
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
            TargetAddr::Domain(host, port)
        }
        _ => return Ok(None),
    };
    Ok(Some(target))
}

async fn socks5_reply(client: &mut TcpStream, code: u8) -> io::Result<()> {
    client.write_all(&[0x05, code, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await
}

fn socks5_code(error: &DialError, hop_count: usize) -> u8 {
    match error {
        DialError::Handshake { hop, source: HandshakeError::Socks5Reply(code) }
            if *hop + 1 == hop_count =>
        {
            *code
        }
        DialError::Timeout => 0x04,
        DialError::Unreachable(_) | DialError::Handshake { .. } | DialError::NoHops => 0x01,
    }
}

async fn serve_http(mut client: TcpStream, shared: Arc<ServerState>) -> Result<(), InboundError> {
    let mut buf = Vec::with_capacity(4096);
    let head_len = loop {
        if let Some(pos) = find_head_end(&buf) {
            break pos;
        }
        if buf.len() >= MAX_REQUEST_HEAD {
            respond(&mut client, "431 Request Header Fields Too Large").await?;
            return Err(InboundError::Protocol("HTTP request head is too large"));
        }
        let mut chunk = [0u8; 4096];
        let n = client.read(&mut chunk).await?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&chunk[..n]);
    };

    let mut headers = [httparse::EMPTY_HEADER; 128];
    let mut request = httparse::Request::new(&mut headers);
    if !matches!(request.parse(&buf[..head_len]), Ok(httparse::Status::Complete(_))) {
        respond(&mut client, "400 Bad Request").await?;
        return Err(InboundError::Protocol("malformed HTTP request"));
    }
    let method = request.method.unwrap_or_default();
    let path = request.path.unwrap_or_default();
    let leftover = &buf[head_len..];

    if method.eq_ignore_ascii_case("CONNECT") {
        let Some(target) = TargetAddr::parse_authority(path, 443) else {
            respond(&mut client, "400 Bad Request").await?;
            return Err(InboundError::Protocol("invalid CONNECT authority"));
        };
        return match dial_timeout(&shared.hops, &target, shared.connect_timeout).await {
            Ok(remote) => {
                client.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await?;
                tunnel(client, remote, &shared, leftover).await
            }
            Err(source) => {
                respond(&mut client, http_status(&source)).await?;
                Err(InboundError::Dial { target, source })
            }
        };
    }

    let Some((target, origin_path)) = split_absolute_uri(path) else {
        respond(&mut client, "400 Bad Request").await?;
        return Err(InboundError::Protocol("HTTP proxy request is not absolute-form"));
    };
    let mut forwarded =
        format!("{method} {origin_path} HTTP/1.{}\r\n", request.version.unwrap_or(1));
    let mut has_host = false;
    for header in request.headers.iter() {
        let name = header.name.to_ascii_lowercase();
        if HOP_BY_HOP.contains(&name.as_str()) {
            continue;
        }
        has_host |= name == "host";
        forwarded.push_str(header.name);
        forwarded.push_str(": ");
        forwarded.push_str(&String::from_utf8_lossy(header.value));
        forwarded.push_str("\r\n");
    }
    if !has_host {
        forwarded.push_str(&format!("Host: {}\r\n", host_header(&target)));
    }
    forwarded.push_str("Connection: close\r\n\r\n");

    let mut prefix = forwarded.into_bytes();
    prefix.extend_from_slice(leftover);
    match dial_timeout(&shared.hops, &target, shared.connect_timeout).await {
        Ok(remote) => tunnel(client, remote, &shared, &prefix).await,
        Err(source) => {
            respond(&mut client, http_status(&source)).await?;
            Err(InboundError::Dial { target, source })
        }
    }
}

fn find_head_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|pos| pos + 4)
}

fn split_absolute_uri(uri: &str) -> Option<(TargetAddr, String)> {
    let scheme_end = uri.find("://")?;
    if !uri[..scheme_end].eq_ignore_ascii_case("http") {
        return None;
    }
    let rest = &uri[scheme_end + 3..];
    let (authority, path) = match rest.find(['/', '?']) {
        Some(pos) => (&rest[..pos], &rest[pos..]),
        None => (rest, "/"),
    };
    let authority = authority.rsplit_once('@').map_or(authority, |(_, host)| host);
    let target = TargetAddr::parse_authority(authority, 80)?;
    let path = if path.starts_with('?') { format!("/{path}") } else { path.to_owned() };
    Some((target, path))
}

fn host_header(target: &TargetAddr) -> String {
    if target.port() == 80 {
        match target {
            TargetAddr::Ip(SocketAddr::V6(addr)) => format!("[{}]", addr.ip()),
            _ => target.host(),
        }
    } else {
        target.to_string()
    }
}

fn http_status(error: &DialError) -> &'static str {
    match error {
        DialError::Timeout => "504 Gateway Timeout",
        _ => "502 Bad Gateway",
    }
}

async fn respond(client: &mut TcpStream, status: &str) -> io::Result<()> {
    let response = format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    client.write_all(response.as_bytes()).await
}

async fn tunnel(
    client: TcpStream,
    mut remote: TcpStream,
    shared: &ServerState,
    prefix: &[u8],
) -> Result<(), InboundError> {
    if !prefix.is_empty() {
        remote.write_all(prefix).await?;
        shared.counters.up.fetch_add(prefix.len() as u64, Ordering::Relaxed);
    }
    let mut client =
        MeteredStream { inner: client, read: &shared.counters.up, written: &shared.counters.down };
    tokio::io::copy_bidirectional(&mut client, &mut remote).await?;
    Ok(())
}

struct MeteredStream<'a, S> {
    inner: S,
    read: &'a AtomicU64,
    written: &'a AtomicU64,
}

impl<S: AsyncRead + Unpin> AsyncRead for MeteredStream<'_, S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let before = buf.filled().len();
        let poll = Pin::new(&mut self.inner).poll_read(cx, buf);
        let read = buf.filled().len() - before;
        if read > 0 {
            self.read.fetch_add(read as u64, Ordering::Relaxed);
        }
        poll
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for MeteredStream<'_, S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let poll = Pin::new(&mut self.inner).poll_write(cx, buf);
        if let Poll::Ready(Ok(written)) = poll {
            self.written.fetch_add(written as u64, Ordering::Relaxed);
        }
        poll
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}
