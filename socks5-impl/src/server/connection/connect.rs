use crate::protocol::{Address, AsyncStreamOperation, Reply, Response};
use stream::Stream;
use tokio::net::tcp::{ReadHalf, WriteHalf};

/// Socks5 connection type `Connect`
///
/// This connection can be used as a regular async TCP stream after replying the client.
#[derive(Debug)]
pub struct Connect<S> {
    pub stream: Stream,
    _state: S,
}

impl<S: Default> Connect<S> {
    #[inline]
    pub(super) fn new(stream: Stream) -> Self {
        Self {
            stream,
            _state: S::default(),
        }
    }
}

#[derive(Debug, Default)]
pub struct NeedReply;

#[derive(Debug, Default)]
pub struct Ready;

impl Connect<NeedReply> {
    /// Reply to the client.
    #[inline]
    pub async fn reply(mut self, reply: Reply, addr: Address) -> std::io::Result<Connect<Ready>> {
        let resp = Response::new(reply, addr);
        resp.write_to_async_stream(&mut *self.stream).await?;
        Ok(Connect::<Ready>::new(self.stream))
    }
}

impl Connect<Ready> {
    /// Returns the read/write half of the stream.
    #[inline]
    pub fn split(&mut self) -> (ReadHalf<'_>, WriteHalf<'_>) {
        (*self.stream).split()
    }
}

impl<S> From<Connect<S>> for Stream {
    #[inline]
    fn from(conn: Connect<S>) -> Self {
        conn.stream
    }
}
