use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::InboundError;
use crate::address::TargetAddr;
use crate::dial::{DialError, dial_timeout};
use crate::server::ServerState;
use crate::tunnel::tunnel;

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

pub(super) async fn serve(
    mut client: TcpStream,
    shared: Arc<ServerState>,
) -> Result<(), InboundError> {
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
                Ok(tunnel(client, remote, &shared.counters, leftover).await?)
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
        Ok(remote) => Ok(tunnel(client, remote, &shared.counters, &prefix).await?),
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
