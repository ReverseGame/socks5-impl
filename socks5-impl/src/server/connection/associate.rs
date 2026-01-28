use crate::protocol::{Address, AsyncStreamOperation, Reply, Response, StreamOperation, UdpHeader};
use bytes::{Bytes, BytesMut};
use std::{
    net::SocketAddr,
    sync::atomic::{AtomicUsize, Ordering},
};
use stream::Stream;
use tokio::{
    io::AsyncReadExt,
    net::{ToSocketAddrs, UdpSocket},
};

/// Socks5 connection type `UdpAssociate`
#[derive(Debug)]
pub struct UdpAssociate<S> {
    pub stream: Stream,
    _state: S,
}

impl<S: Default> UdpAssociate<S> {
    #[inline]
    pub(super) fn new(stream: Stream) -> Self {
        Self {
            stream,
            _state: S::default(),
        }
    }

    /// Reply to the SOCKS5 client with the given reply and address.
    ///
    /// If encountered an error while writing the reply, the error alongside the original `TcpStream` is returned.
    pub async fn reply(mut self, reply: Reply, addr: Address) -> std::io::Result<UdpAssociate<Ready>> {
        let resp = Response::new(reply, addr);
        resp.write_to_async_stream(&mut *self.stream).await?;
        Ok(UdpAssociate::<Ready>::new(self.stream))
    }
}

#[derive(Debug, Default)]
pub struct NeedReply;

#[derive(Debug, Default)]
pub struct Ready;

impl UdpAssociate<Ready> {
    /// Wait until the client closes this TCP connection.
    ///
    /// Socks5 protocol defines that when the client closes the TCP connection used to send the associate command,
    /// the server should release the associated UDP socket.
    pub async fn wait_until_closed(&mut self) -> std::io::Result<()> {
        loop {
            match self.stream.read(&mut [0]).await {
                Ok(0) => break Ok(()),
                Ok(_) => {}
                Err(err) => break Err(err),
            }
        }
    }
}

impl<S> From<UdpAssociate<S>> for Stream {
    #[inline]
    fn from(conn: UdpAssociate<S>) -> Self {
        conn.stream
    }
}

/// This is a helper for managing the associated UDP socket.
///
/// It will add the socks5 UDP header to every UDP packet it sends, also try to parse the socks5 UDP header from any UDP packet received.
///
/// The receiving buffer size for each UDP packet can be set with [`set_recv_buffer_size()`](#method.set_recv_buffer_size),
/// and be read with [`get_max_packet_size()`](#method.get_recv_buffer_size).
///
/// You can create this struct by using [`AssociatedUdpSocket::from::<(UdpSocket, usize)>()`](#impl-From<UdpSocket>),
/// the first element of the tuple is the UDP socket, the second element is the receiving buffer size.
///
/// This struct can also be revert into a raw tokio UDP socket with [`UdpSocket::from::<AssociatedUdpSocket>()`](#impl-From<AssociatedUdpSocket>>.
///
/// [`AssociatedUdpSocket`] can be used as the associated UDP socket.
///
/// # Performance Note
///
/// This struct is aligned to 64 bytes (typical cache line size) to prevent false sharing
/// in multi-threaded scenarios where the atomic `buf_size` field is frequently accessed
/// concurrently with the socket operations.
#[derive(Debug)]
#[repr(align(64))]
pub struct AssociatedUdpSocket {
    socket: UdpSocket,
    buf_size: AtomicUsize,
}

impl AssociatedUdpSocket {
    /// Connects the UDP socket setting the default destination for send() and limiting packets that are read via recv from the address specified in addr.
    #[inline]
    pub async fn connect<A: ToSocketAddrs>(&self, addr: A) -> std::io::Result<()> {
        self.socket.connect(addr).await
    }

    /// Get the maximum UDP packet size, with socks5 UDP header included.
    pub fn get_max_packet_size(&self) -> usize {
        self.buf_size.load(Ordering::Relaxed)
    }

    /// Set the maximum UDP packet size, with socks5 UDP header included, for adjusting the receiving buffer size.
    pub fn set_max_packet_size(&self, size: usize) {
        self.buf_size.store(size, Ordering::Release);
    }

    /// Receives a socks5 UDP relay packet on the socket from the remote address to which it is connected.
    /// On success, returns the packet itself, the fragment number and the remote target address.
    ///
    /// The [`connect`](#method.connect) method will connect this socket to a remote address.
    /// This method will fail if the socket is not connected.
    pub async fn recv(&self) -> std::io::Result<(Bytes, u8, Address)> {
        let max_packet_size = self.buf_size.load(Ordering::Acquire);
        // 使用 BytesMut 避免初始化开销，并支持潜在的缓冲区复用
        let mut buf = BytesMut::zeroed(max_packet_size);

        loop {
            let len = self.socket.recv(&mut buf).await?;
            let pkt = buf.split_to(len).freeze();

            if let Ok(header) = UdpHeader::retrieve_from_async_stream(&mut pkt.as_ref()).await {
                let pkt = pkt.slice(header.len()..);
                return Ok((pkt, header.frag, header.address));
            }

            // 解析失败，重置缓冲区以便重试
            buf.clear();
            buf.resize(max_packet_size, 0);
        }
    }

    /// Receives a socks5 UDP relay packet on the socket from the any remote address.
    /// On success, returns the packet itself, the fragment number, the remote target address and the source address.
    pub async fn recv_from(&self) -> std::io::Result<(Bytes, u8, Address, SocketAddr)> {
        let max_packet_size = self.buf_size.load(Ordering::Acquire);
        // 使用 BytesMut 避免初始化开销，并支持潜在的缓冲区复用
        let mut buf = BytesMut::zeroed(max_packet_size);

        loop {
            let (len, src_addr) = self.socket.recv_from(&mut buf).await?;
            let pkt = buf.split_to(len).freeze();

            if let Ok(header) = UdpHeader::retrieve_from_async_stream(&mut pkt.as_ref()).await {
                let pkt = pkt.slice(header.len()..);
                return Ok((pkt, header.frag, header.address, src_addr));
            }

            // 解析失败，重置缓冲区以便重试
            buf.clear();
            buf.resize(max_packet_size, 0);
        }
    }

    /// Sends a UDP relay packet to the remote address to which it is connected. The socks5 UDP header will be added to the packet.
    pub async fn send<P: AsRef<[u8]>>(&self, pkt: P, frag: u8, from_addr: Address) -> std::io::Result<usize> {
        let header = UdpHeader::new(frag, from_addr);
        let mut buf = BytesMut::with_capacity(header.len() + pkt.as_ref().len());
        header.write_to_buf(&mut buf);
        buf.extend_from_slice(pkt.as_ref());

        self.socket.send(&buf).await.map(|len| len - header.len())
    }

    /// Sends a UDP relay packet to a specified remote address to which it is connected. The socks5 UDP header will be added to the packet.
    pub async fn send_to<P: AsRef<[u8]>>(&self, pkt: P, frag: u8, from_addr: Address, to_addr: SocketAddr) -> std::io::Result<usize> {
        let header = UdpHeader::new(frag, from_addr);
        let mut buf = BytesMut::with_capacity(header.len() + pkt.as_ref().len());
        header.write_to_buf(&mut buf);
        buf.extend_from_slice(pkt.as_ref());

        self.socket.send_to(&buf, to_addr).await.map(|len| len - header.len())
    }
}

impl From<(UdpSocket, usize)> for AssociatedUdpSocket {
    #[inline]
    fn from(from: (UdpSocket, usize)) -> Self {
        AssociatedUdpSocket {
            socket: from.0,
            buf_size: AtomicUsize::new(from.1),
        }
    }
}

impl From<AssociatedUdpSocket> for UdpSocket {
    #[inline]
    fn from(from: AssociatedUdpSocket) -> Self {
        from.socket
    }
}

impl AsRef<UdpSocket> for AssociatedUdpSocket {
    #[inline]
    fn as_ref(&self) -> &UdpSocket {
        &self.socket
    }
}

impl AsMut<UdpSocket> for AssociatedUdpSocket {
    #[inline]
    fn as_mut(&mut self) -> &mut UdpSocket {
        &mut self.socket
    }
}
