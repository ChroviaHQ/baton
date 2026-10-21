use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::TcpStream;

use crate::server::Counters;

pub(crate) async fn tunnel(
    client: TcpStream,
    mut remote: TcpStream,
    counters: &Counters,
    prefix: &[u8],
) -> io::Result<()> {
    if !prefix.is_empty() {
        remote.write_all(prefix).await?;
        counters.up.fetch_add(prefix.len() as u64, Ordering::Relaxed);
    }
    let mut client = MeteredStream { inner: client, read: &counters.up, written: &counters.down };
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
