use std::net::SocketAddr;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use baton_core::{Hop, ProxyKind, Server, ServerConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

#[derive(Clone, Copy)]
enum Mock {
    Http,
    Socks5,
}

#[derive(Clone)]
struct MockProxy {
    kind: Mock,
    credentials: Option<(&'static str, &'static str)>,
}

async fn spawn_mock(proxy: MockProxy, seen: mpsc::UnboundedSender<String>) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let proxy = proxy.clone();
            let seen = seen.clone();
            tokio::spawn(async move {
                let _ = match proxy.kind {
                    Mock::Http => mock_http(stream, proxy.credentials, seen).await,
                    Mock::Socks5 => mock_socks5(stream, proxy.credentials, seen).await,
                };
            });
        }
    });
    addr
}

async fn mock_http(
    mut stream: TcpStream,
    credentials: Option<(&str, &str)>,
    seen: mpsc::UnboundedSender<String>,
) -> std::io::Result<()> {
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        head.push(stream.read_u8().await?);
    }
    let head = String::from_utf8(head).unwrap();
    let authority = head.split_whitespace().nth(1).unwrap().to_owned();
    if let Some((username, password)) = credentials {
        let expected = format!(
            "Proxy-Authorization: Basic {}",
            STANDARD.encode(format!("{username}:{password}"))
        );
        if !head.contains(&expected) {
            stream
                .write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n")
                .await?;
            return Ok(());
        }
    }
    seen.send(format!("http {authority}")).unwrap();
    let mut upstream = TcpStream::connect(authority).await?;
    stream
        .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
        .await?;
    tokio::io::copy_bidirectional(&mut stream, &mut upstream).await?;
    Ok(())
}

async fn mock_socks5(
    mut stream: TcpStream,
    credentials: Option<(&str, &str)>,
    seen: mpsc::UnboundedSender<String>,
) -> std::io::Result<()> {
    let mut header = [0u8; 2];
    stream.read_exact(&mut header).await?;
    let mut methods = vec![0u8; header[1] as usize];
    stream.read_exact(&mut methods).await?;
    match credentials {
        Some((username, password)) => {
            if !methods.contains(&0x02) {
                stream.write_all(&[0x05, 0xff]).await?;
                return Ok(());
            }
            stream.write_all(&[0x05, 0x02]).await?;
            let _version = stream.read_u8().await?;
            let mut user = vec![0u8; stream.read_u8().await? as usize];
            stream.read_exact(&mut user).await?;
            let mut pass = vec![0u8; stream.read_u8().await? as usize];
            stream.read_exact(&mut pass).await?;
            if user != username.as_bytes() || pass != password.as_bytes() {
                stream.write_all(&[0x01, 0x01]).await?;
                return Ok(());
            }
            stream.write_all(&[0x01, 0x00]).await?;
        }
        None => stream.write_all(&[0x05, 0x00]).await?,
    }
    let mut request = [0u8; 4];
    stream.read_exact(&mut request).await?;
    let host = match request[3] {
        0x01 => {
            let mut ip = [0u8; 4];
            stream.read_exact(&mut ip).await?;
            std::net::Ipv4Addr::from(ip).to_string()
        }
        0x03 => {
            let mut host = vec![0u8; stream.read_u8().await? as usize];
            stream.read_exact(&mut host).await?;
            String::from_utf8(host).unwrap()
        }
        _ => unreachable!(),
    };
    let port = stream.read_u16().await?;
    seen.send(format!("socks5 {host}:{port}")).unwrap();
    let mut upstream = match TcpStream::connect((host.as_str(), port)).await {
        Ok(upstream) => upstream,
        Err(_) => {
            stream
                .write_all(&[0x05, 0x05, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
                .await?;
            return Ok(());
        }
    };
    stream
        .write_all(&[0x05, 0x00, 0x00, 0x03, 4, b'b', b'n', b'd', b'x', 0, 1])
        .await?;
    tokio::io::copy_bidirectional(&mut stream, &mut upstream).await?;
    Ok(())
}

async fn spawn_echo() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            tokio::spawn(async move {
                let (mut reader, mut writer) = stream.split();
                let _ = tokio::io::copy(&mut reader, &mut writer).await;
            });
        }
    });
    addr
}

async fn spawn_http_origin() -> (SocketAddr, mpsc::UnboundedReceiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut head = Vec::new();
                while !head.ends_with(b"\r\n\r\n") {
                    head.push(stream.read_u8().await.unwrap());
                }
                tx.send(String::from_utf8(head).unwrap()).unwrap();
                stream
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
                    .await
                    .unwrap();
            });
        }
    });
    (addr, rx)
}

fn hop(kind: ProxyKind, addr: SocketAddr, credentials: Option<(&str, &str)>) -> Hop {
    let (username, password) = credentials.unwrap_or_default();
    Hop {
        kind,
        host: addr.ip().to_string(),
        port: addr.port(),
        username: username.into(),
        password: password.into(),
    }
}

async fn start(hops: Vec<Hop>) -> Server {
    Server::start(ServerConfig {
        listen: "127.0.0.1:0".parse().unwrap(),
        hops,
        connect_timeout: Duration::from_secs(5),
    })
    .await
    .unwrap()
}

async fn socks5_client(proxy: SocketAddr, target: SocketAddr) -> (TcpStream, u8) {
    let mut stream = TcpStream::connect(proxy).await.unwrap();
    stream.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    let mut choice = [0u8; 2];
    stream.read_exact(&mut choice).await.unwrap();
    assert_eq!(choice, [0x05, 0x00]);
    let host = target.ip().to_string();
    let mut request = vec![0x05, 0x01, 0x00, 0x03, host.len() as u8];
    request.extend_from_slice(host.as_bytes());
    request.extend_from_slice(&target.port().to_be_bytes());
    stream.write_all(&request).await.unwrap();
    let mut reply = [0u8; 10];
    stream.read_exact(&mut reply).await.unwrap();
    (stream, reply[1])
}

async fn read_response_head(stream: &mut TcpStream) -> String {
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        head.push(stream.read_u8().await.unwrap());
    }
    String::from_utf8(head).unwrap()
}

#[tokio::test]
async fn socks5_inbound_through_http_then_socks5_with_auth() {
    let (seen_tx, mut seen) = mpsc::unbounded_channel();
    let echo = spawn_echo().await;
    let upstream = spawn_mock(
        MockProxy {
            kind: Mock::Http,
            credentials: None,
        },
        seen_tx.clone(),
    )
    .await;
    let landing = spawn_mock(
        MockProxy {
            kind: Mock::Socks5,
            credentials: Some(("alice", "s3cret")),
        },
        seen_tx,
    )
    .await;
    let server = start(vec![
        hop(ProxyKind::Http, upstream, None),
        hop(ProxyKind::Socks5, landing, Some(("alice", "s3cret"))),
    ])
    .await;

    let (mut stream, code) = socks5_client(server.local_addr(), echo).await;
    assert_eq!(code, 0x00);
    stream.write_all(b"relay").await.unwrap();
    let mut buf = [0u8; 5];
    stream.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"relay");

    assert_eq!(seen.recv().await.unwrap(), format!("http {landing}"));
    assert_eq!(seen.recv().await.unwrap(), format!("socks5 {echo}"));
    drop(stream);
    tokio::time::sleep(Duration::from_millis(50)).await;
    let stats = server.stats();
    assert_eq!(stats.total, 1);
    assert_eq!(stats.up_bytes, 5);
    assert_eq!(stats.down_bytes, 5);
    server.stop().await;
}

#[tokio::test]
async fn http_connect_inbound_through_socks5_then_http_with_auth() {
    let (seen_tx, mut seen) = mpsc::unbounded_channel();
    let echo = spawn_echo().await;
    let upstream = spawn_mock(
        MockProxy {
            kind: Mock::Socks5,
            credentials: None,
        },
        seen_tx.clone(),
    )
    .await;
    let landing = spawn_mock(
        MockProxy {
            kind: Mock::Http,
            credentials: Some(("bob", "pw")),
        },
        seen_tx,
    )
    .await;
    let server = start(vec![
        hop(ProxyKind::Socks5, upstream, None),
        hop(ProxyKind::Http, landing, Some(("bob", "pw"))),
    ])
    .await;

    let mut stream = TcpStream::connect(server.local_addr()).await.unwrap();
    stream
        .write_all(format!("CONNECT {echo} HTTP/1.1\r\nHost: {echo}\r\n\r\nearly").as_bytes())
        .await
        .unwrap();
    let head = read_response_head(&mut stream).await;
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    let mut buf = [0u8; 5];
    stream.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"early");

    assert_eq!(seen.recv().await.unwrap(), format!("socks5 {landing}"));
    assert_eq!(seen.recv().await.unwrap(), format!("http {echo}"));
    server.stop().await;
}

#[tokio::test]
async fn plain_http_request_is_rewritten_to_origin_form() {
    let (seen_tx, _seen) = mpsc::unbounded_channel();
    let (origin, mut requests) = spawn_http_origin().await;
    let upstream = spawn_mock(
        MockProxy {
            kind: Mock::Http,
            credentials: None,
        },
        seen_tx.clone(),
    )
    .await;
    let landing = spawn_mock(
        MockProxy {
            kind: Mock::Http,
            credentials: None,
        },
        seen_tx,
    )
    .await;
    let server = start(vec![
        hop(ProxyKind::Http, upstream, None),
        hop(ProxyKind::Http, landing, None),
    ])
    .await;

    let mut stream = TcpStream::connect(server.local_addr()).await.unwrap();
    stream
        .write_all(
            format!(
                "GET http://{origin}/path?q=1 HTTP/1.1\r\nHost: {origin}\r\nProxy-Connection: keep-alive\r\nProxy-Authorization: Basic eA==\r\nAccept: */*\r\n\r\n"
            )
            .as_bytes(),
        )
        .await
        .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(response.ends_with("ok"));

    let forwarded = requests.recv().await.unwrap();
    assert!(
        forwarded.starts_with("GET /path?q=1 HTTP/1.1\r\n"),
        "{forwarded}"
    );
    assert!(forwarded.contains(&format!("Host: {origin}\r\n")));
    assert!(forwarded.contains("Accept: */*\r\n"));
    assert!(forwarded.contains("Connection: close\r\n"));
    assert!(!forwarded.to_ascii_lowercase().contains("proxy-"));
    server.stop().await;
}

#[tokio::test]
async fn landing_auth_failure_is_reported_to_clients() {
    let (seen_tx, _seen) = mpsc::unbounded_channel();
    let echo = spawn_echo().await;
    let upstream = spawn_mock(
        MockProxy {
            kind: Mock::Http,
            credentials: None,
        },
        seen_tx.clone(),
    )
    .await;
    let landing = spawn_mock(
        MockProxy {
            kind: Mock::Http,
            credentials: Some(("bob", "pw")),
        },
        seen_tx,
    )
    .await;
    let server = start(vec![
        hop(ProxyKind::Http, upstream, None),
        hop(ProxyKind::Http, landing, Some(("bob", "wrong"))),
    ])
    .await;

    let mut stream = TcpStream::connect(server.local_addr()).await.unwrap();
    stream
        .write_all(format!("CONNECT {echo} HTTP/1.1\r\n\r\n").as_bytes())
        .await
        .unwrap();
    let head = read_response_head(&mut stream).await;
    assert!(head.starts_with("HTTP/1.1 502"), "{head}");

    let (_stream, code) = socks5_client(server.local_addr(), echo).await;
    assert_eq!(code, 0x01);
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(server.stats().failed, 2);
    server.stop().await;
}

#[tokio::test]
async fn unreachable_target_passes_socks5_reply_through() {
    let (seen_tx, _seen) = mpsc::unbounded_channel();
    let closed = TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap()
        .local_addr()
        .unwrap();
    let upstream = spawn_mock(
        MockProxy {
            kind: Mock::Socks5,
            credentials: None,
        },
        seen_tx.clone(),
    )
    .await;
    let landing = spawn_mock(
        MockProxy {
            kind: Mock::Socks5,
            credentials: None,
        },
        seen_tx,
    )
    .await;
    let server = start(vec![
        hop(ProxyKind::Socks5, upstream, None),
        hop(ProxyKind::Socks5, landing, None),
    ])
    .await;

    let (_stream, code) = socks5_client(server.local_addr(), closed).await;
    assert_eq!(code, 0x05);
    server.stop().await;
}

#[tokio::test]
async fn stop_releases_the_port() {
    let server = start(vec![hop(
        ProxyKind::Http,
        "127.0.0.1:1".parse().unwrap(),
        None,
    )])
    .await;
    let addr = server.local_addr();
    server.stop().await;
    TcpListener::bind(addr).await.unwrap();
}
