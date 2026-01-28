mod address;
mod command;
pub mod handshake;
mod reply;
mod request;
mod response;
mod udp;

pub use self::{
    address::{Address, AddressType},
    command::Command,
    handshake::{
        AuthMethod,
        password_method::{self, UserKey},
    },
    reply::Reply,
    request::Request,
    response::Response,
    udp::UdpHeader,
};
pub use bytes::BufMut;

use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

/// SOCKS protocol version, either 4 or 5
#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub enum Version {
    V4 = 4,
    #[default]
    V5 = 5,
}

impl TryFrom<u8> for Version {
    type Error = std::io::Error;

    fn try_from(value: u8) -> std::io::Result<Self> {
        match value {
            4 => Ok(Version::V4),
            5 => Ok(Version::V5),
            _ => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid version")),
        }
    }
}

impl From<Version> for u8 {
    fn from(v: Version) -> Self {
        v as u8
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v: u8 = (*self).into();
        write!(f, "{v}")
    }
}

/// SOCKS5 协议流操作 trait（统一序列化和异步 I/O）
#[async_trait::async_trait]
pub trait StreamOperation {
    /// 从异步流中读取并反序列化对象
    async fn retrieve_from_async_stream<R>(r: &mut R) -> std::io::Result<Self>
    where
        R: AsyncRead + Unpin + Send + ?Sized,
        Self: Sized;

    /// 将对象序列化到缓冲区
    fn write_to_buf<B: BufMut>(&self, buf: &mut B);

    /// 返回序列化后的字节长度
    fn len(&self) -> usize;

    /// 判断是否为空
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 将对象序列化并写入异步流（提供默认实现）
    async fn write_to_async_stream<W>(&self, w: &mut W) -> std::io::Result<()>
    where
        W: AsyncWrite + Unpin + Send + ?Sized,
    {
        let mut buf = bytes::BytesMut::with_capacity(self.len());
        self.write_to_buf(&mut buf);
        w.write_all(&buf).await
    }
}
