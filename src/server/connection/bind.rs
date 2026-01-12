use crate::protocol::{Address, AsyncStreamOperation, Reply, Response};
use std::{
    marker::PhantomData,
};
use tokio::{
    net::{
        tcp::{ReadHalf, WriteHalf},
    },
};
use crate::server::connection::stream::Stream;

/// Socks5 command type `Bind`
///
/// By [`wait_request`](crate::server::connection::Authenticated::wait_request)
/// on an [`Authenticated`](crate::server::connection::Authenticated) from SOCKS5 client,
/// you may get a `Bind<NeedFirstReply>`. After replying the client 2 times
/// using [`reply()`](crate::server::connection::Bind::reply),
/// you will get a `Bind<Ready>`, which can be used as a regular async TCP stream.
///
/// A `Bind<S>` can be converted to a regular tokio [`TcpStream`](https://docs.rs/tokio/latest/tokio/net/struct.TcpStream.html) by using the `From` trait.
#[derive(Debug)]
pub struct Bind<S> {
    pub stream: Stream,
    _state: PhantomData<S>,
}

/// Marker type indicating that the connection needs its first reply.
#[derive(Debug, Default)]
pub struct NeedFirstReply;

/// Marker type indicating that the connection needs its second reply.
#[derive(Debug, Default)]
pub struct NeedSecondReply;

/// Marker type indicating that the connection is ready to use as a regular TCP stream.
#[derive(Debug, Default)]
pub struct Ready;

impl Bind<NeedFirstReply> {
    #[inline]
    pub(super) fn new(stream: Stream) -> Self {
        Self {
            stream,
            _state: PhantomData,
        }
    }

    /// Reply to the SOCKS5 client with the given reply and address.
    ///
    /// If encountered an error while writing the reply, the error alongside the original `TcpStream` is returned.
    pub async fn reply(mut self, reply: Reply, addr: Address) -> std::io::Result<Bind<NeedSecondReply>> {
        let resp = Response::new(reply, addr);
        resp.write_to_async_stream(&mut self.stream.stream).await?;
        Ok(Bind::<NeedSecondReply>::new(self.stream))
    }
}

impl Bind<NeedSecondReply> {
    #[inline]
    fn new(stream: Stream) -> Self {
        Self {
            stream,
            _state: PhantomData,
        }
    }

    /// Reply to the SOCKS5 client with the given reply and address.
    ///
    /// If encountered an error while writing the reply, the error alongside the original `TcpStream` is returned.
    pub async fn reply(mut self, reply: Reply, addr: Address) -> Result<Bind<Ready>, (std::io::Error, Stream)> {
        let resp = Response::new(reply, addr);

        if let Err(err) = resp.write_to_async_stream(&mut self.stream.stream).await {
            return Err((err, self.stream));
        }

        Ok(Bind::<Ready>::new(self.stream))
    }
}

impl Bind<Ready> {
    #[inline]
    fn new(stream: Stream) -> Self {
        Self {
            stream,
            _state: PhantomData,
        }
    }

    /// Split the connection into a read and a write half.
    #[inline]
    pub fn split(&mut self) -> (ReadHalf<'_>, WriteHalf<'_>) {
        self.stream.split()
    }
}

impl<S> From<Bind<S>> for Stream {
    #[inline]
    fn from(conn: Bind<S>) -> Self {
        conn.stream
    }
}
