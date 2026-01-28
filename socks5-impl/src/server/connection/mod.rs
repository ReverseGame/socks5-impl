use self::{associate::UdpAssociate, bind::Bind, connect::Connect};
use crate::protocol::{self, Address, AsyncStreamOperation, AuthMethod, Command, handshake};
use crate::server::AuthAdaptor;
use std::time::Duration;
use stream::Stream;
use tokio::net::TcpStream;

pub mod associate;
pub mod bind;
pub mod connect;

/// An incoming connection. This may not be a valid socks5 connection. You need to call [`authenticate()`](#method.authenticate)
/// to perform the socks5 handshake. It will be converted to a proper socks5 connection after the handshake succeeds.
pub struct IncomingConnection<O> {
    stream: TcpStream,
    auth: AuthAdaptor<O>,
}

impl<O> IncomingConnection<O> {
    #[inline]
    pub fn new(stream: TcpStream, auth: AuthAdaptor<O>) -> Self {
        IncomingConnection { stream, auth }
    }
    /// Set a timeout for the SOCKS5 handshake.
    pub async fn authenticate_with_timeout(self, timeout: Duration) -> crate::Result<(Authenticated, O)> {
        tokio::time::timeout(timeout, self.authenticate())
            .await
            .map_err(|_| crate::Error::String("handshake timeout".into()))?
    }

    /// Perform a SOCKS5 authentication handshake using the given
    /// [`AuthExecutor`](crate::server::auth::AuthExecutor) adapter.
    ///
    /// If the handshake succeeds, an [`Authenticated`]
    /// alongs with the output of the [`AuthExecutor`](crate::server::auth::AuthExecutor) adapter is returned.
    /// Otherwise, the error and the original [`TcpStream`](https://docs.rs/tokio/latest/tokio/net/struct.TcpStream.html) is returned.
    ///
    /// Note that this method will not implicitly close the connection even if the handshake failed.
    pub async fn authenticate(mut self) -> crate::Result<(Authenticated, O)> {
        let request = handshake::Request::retrieve_from_async_stream(&mut self.stream).await?;
        if let Some(method) = self.evaluate_request(&request) {
            // Note: set_method is not called here because auth is behind Arc and requires &mut self
            // The default implementation does nothing anyway
            let response = handshake::Response::new(method);
            response.write_to_async_stream(&mut self.stream).await?;
            let output = self.auth.execute(&mut self.stream).await;
            Ok((Authenticated::new(Stream::new(self.stream)), output))
        } else {
            let response = handshake::Response::new(AuthMethod::NoAcceptableMethods);
            response.write_to_async_stream(&mut self.stream).await?;
            let err = "No available handshake method provided by client";
            Err(crate::Error::Io(std::io::Error::new(std::io::ErrorKind::Unsupported, err)))
        }
    }

    fn evaluate_request(&self, req: &handshake::Request) -> Option<AuthMethod> {
        let method = self.auth.auth_method();
        if req.evaluate_method(method) {
            Some(method)
        } else {
            Some(AuthMethod::NoAuth)
        }
    }
}

impl<O> std::fmt::Debug for IncomingConnection<O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IncomingConnection").field("stream", &self.stream).finish()
    }
}

impl<O> From<IncomingConnection<O>> for TcpStream {
    #[inline]
    fn from(conn: IncomingConnection<O>) -> Self {
        conn.stream
    }
}

/// A TCP stream that has been authenticated.
///
/// To get the command from the SOCKS5 client, use
/// [`wait_request`](crate::server::connection::Authenticated::wait_request).
///
/// It can also be converted back into a raw [`tokio::TcpStream`](https://docs.rs/tokio/latest/tokio/net/struct.TcpStream.html) with `From` trait.
pub struct Authenticated(Stream);

impl Authenticated {
    #[inline]
    fn new(stream: Stream) -> Self {
        Self(stream)
    }

    /// Waits the SOCKS5 client to send a request.
    ///
    /// This method will return a [`Command`] if the client sends a valid command.
    ///
    /// When encountering an error, the stream will be returned alongside the error.
    ///
    /// Note that this method will not implicitly close the connection even if the client sends an invalid request.
    pub async fn wait_request(mut self) -> crate::Result<ClientConnection> {
        let req = protocol::Request::retrieve_from_async_stream(&mut *self.0).await?;

        match req.command {
            Command::UdpAssociate => Ok(ClientConnection::UdpAssociate(
                UdpAssociate::<associate::NeedReply>::new(self.0),
                req.address,
            )),
            Command::Bind => Ok(ClientConnection::Bind(Bind::<bind::NeedFirstReply>::new(self.0), req.address)),
            Command::Connect => Ok(ClientConnection::Connect(Connect::<connect::NeedReply>::new(self.0), req.address)),
        }
    }
}

impl From<Authenticated> for Stream {
    #[inline]
    fn from(conn: Authenticated) -> Self {
        conn.0
    }
}

/// After the socks5 handshake succeeds, the connection may become:
///
/// - Associate
/// - Bind
/// - Connect
#[derive(Debug)]
pub enum ClientConnection {
    UdpAssociate(UdpAssociate<associate::NeedReply>, Address),
    Bind(Bind<bind::NeedFirstReply>, Address),
    Connect(Connect<connect::NeedReply>, Address),
}
