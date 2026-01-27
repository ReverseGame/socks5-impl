use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::TcpStream;

/// A wrapper around `TcpStream` that performs async graceful shutdown on drop.
///
/// When this struct is dropped, it will spawn a background task to perform
/// a graceful TCP shutdown. This ensures proper connection termination without
/// blocking the current task.
///
/// # Performance Note
///
/// The async drop mechanism uses `tokio::spawn` to avoid blocking. This means:
/// - Drop is non-blocking and won't impact async task performance
/// - Graceful shutdown happens in the background
/// - If you need to ensure shutdown completes, call `shutdown()` explicitly before dropping
#[derive(Debug)]
pub struct Stream {
    // 使用 Option 以便在 Drop 中 take ownership
    pub(crate) stream: Option<TcpStream>,
}

impl Stream {
    /// 获取内部 TcpStream 的引用
    #[inline]
    fn get_stream(&self) -> &TcpStream {
        self.stream.as_ref().expect("Stream has been consumed")
    }

    /// 获取内部 TcpStream 的可变引用
    #[inline]
    fn get_stream_mut(&mut self) -> &mut TcpStream {
        self.stream.as_mut().expect("Stream has been consumed")
    }
}

impl Stream {
    #[inline]
    pub fn new(stream: TcpStream) -> Self {
        Self { stream: Some(stream) }
    }

    /// Causes the other peer to receive a read of length 0, indicating that no more data will be sent.
    /// This only closes the stream in one direction (graceful shutdown).
    ///
    /// # Note
    ///
    /// While `Stream` performs async shutdown on drop, calling this method explicitly ensures
    /// that the shutdown completes and any errors are reported. This is recommended for
    /// critical connections where you need to ensure proper closure.
    #[inline]
    pub async fn shutdown(&mut self) -> std::io::Result<()> {
        self.get_stream_mut().shutdown().await
    }

    /// Consumes the `Stream` and returns the inner `TcpStream`.
    ///
    /// This method extracts the underlying `TcpStream` without triggering the async drop
    /// behavior, giving you full control over the connection lifecycle.
    #[inline]
    pub fn into_inner(mut self) -> TcpStream {
        self.stream.take().expect("Stream has been consumed")
    }

    /// Returns the local address that this stream is bound to.
    #[inline]
    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.get_stream().local_addr()
    }

    /// Returns the remote address that this stream is connected to.
    #[inline]
    pub fn peer_addr(&self) -> std::io::Result<SocketAddr> {
        self.get_stream().peer_addr()
    }

    /// Reads the linger duration for this socket by getting the `SO_LINGER` option.
    ///
    /// For more information about this option, see [`set_linger`](crate::server::connection::Bind::set_linger).
    #[inline]
    pub fn linger(&self) -> std::io::Result<Option<Duration>> {
        self.get_stream().linger()
    }

    /// Gets the value of the `TCP_NODELAY` option on this socket.
    ///
    /// For more information about this option, see [`set_nodelay`](crate::server::connection::Bind::set_nodelay).
    #[inline]
    pub fn nodelay(&self) -> std::io::Result<bool> {
        self.get_stream().nodelay()
    }

    /// Sets the value of the `TCP_NODELAY` option on this socket.
    ///
    /// If set, this option disables the Nagle algorithm. This means that segments are always sent as soon as possible,
    /// even if there is only a small amount of data. When not set, data is buffered until there is a sufficient amount to send out,
    /// thereby avoiding the frequent sending of small packets.
    pub fn set_nodelay(&self, nodelay: bool) -> std::io::Result<()> {
        self.get_stream().set_nodelay(nodelay)
    }

    /// Gets the value of the `IP_TTL` option for this socket.
    ///
    /// For more information about this option, see [`set_ttl`](crate::server::connection::Bind::set_ttl).
    pub fn ttl(&self) -> std::io::Result<u32> {
        self.get_stream().ttl()
    }

    /// Sets the value for the `IP_TTL` option on this socket.
    ///
    /// This value sets the time-to-live field that is used in every packet sent from this socket.
    pub fn set_ttl(&self, ttl: u32) -> std::io::Result<()> {
        self.get_stream().set_ttl(ttl)
    }
}

impl Deref for Stream {
    type Target = TcpStream;

    fn deref(&self) -> &Self::Target {
        self.get_stream()
    }
}

impl DerefMut for Stream {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_stream_mut()
    }
}

// 实现 AsyncRead trait，将操作委托给内部的 TcpStream
impl AsyncRead for Stream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(self.get_stream_mut()).poll_read(cx, buf)
    }
}

// 实现 AsyncWrite trait，将操作委托给内部的 TcpStream
impl AsyncWrite for Stream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(self.get_stream_mut()).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(self.get_stream_mut()).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(self.get_stream_mut()).poll_shutdown(cx)
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(self.get_stream_mut()).poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.get_stream().is_write_vectored()
    }
}

// 实现异步 Drop，在后台执行 graceful shutdown
//
// 这个实现使用 tokio::spawn 来在后台异步执行 shutdown，避免阻塞当前任务。
// 相比之前使用 block_in_place 的版本，这种方式：
// 1. 不会阻塞异步任务执行
// 2. 不会占用线程池资源
// 3. 在高并发场景下性能更好
//
// 注意：由于 shutdown 在后台执行，如果需要确保 shutdown 完成，
// 建议在 drop 之前显式调用 shutdown() 方法。
#[cfg(not(test))]
impl Drop for Stream {
    fn drop(&mut self) {
        // 从 Option 中取出 TcpStream
        if let Some(stream) = self.stream.take() {
            // 尝试在当前 tokio runtime 中异步执行 shutdown
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    let mut stream = stream;
                    // 忽略错误，因为连接可能已经关闭
                    let _ = stream.shutdown().await;
                });
            }
            // 如果不在 tokio runtime 中，stream 会被直接 drop，
            // TCP 协议栈会处理连接关闭
        }
    }
}
