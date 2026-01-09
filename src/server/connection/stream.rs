use std::ops::{Deref, DerefMut};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

#[derive(Debug)]
pub struct Stream {
    pub stream: TcpStream
}

impl Stream {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream
        }
    }
}

impl Deref for Stream {
    type Target = TcpStream;

    fn deref(&self) -> &Self::Target {
        &self.stream
    }
}

impl DerefMut for Stream {

    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.stream
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { 
                let _ = self.stream.shutdown().await;
            });
        });
    }
}