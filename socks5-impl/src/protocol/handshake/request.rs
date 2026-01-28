#[cfg(feature = "tokio")]
use crate::protocol::AsyncStreamOperation;
use crate::protocol::{AuthMethod, StreamOperation, Version};
#[cfg(feature = "tokio")]
use tokio::io::{AsyncRead, AsyncReadExt};

/// SOCKS5 handshake request
///
/// ```plain
/// +-----+----------+----------+
/// | VER | NMETHODS | METHODS  |
/// +-----+----------+----------+
/// |  1  |    1     | 1 to 255 |
/// +-----+----------+----------|
/// ```
#[derive(Clone, Debug)]
pub struct Request {
    methods: Vec<AuthMethod>,
}

impl Request {
    pub fn new(methods: Vec<AuthMethod>) -> Self {
        Self { methods }
    }

    pub fn evaluate_method(&self, server_method: AuthMethod) -> bool {
        self.methods.contains(&server_method)
    }
}

impl StreamOperation for Request {
    fn retrieve_from_stream<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        let mut ver = [0; 1];
        r.read_exact(&mut ver)?;
        let ver = Version::try_from(ver[0])?;

        if ver != Version::V5 {
            let err = format!("Unsupported SOCKS version {0:#x}", u8::from(ver));
            return Err(std::io::Error::new(std::io::ErrorKind::Unsupported, err));
        }

        let mut mlen = [0; 1];
        r.read_exact(&mut mlen)?;
        let mlen = mlen[0];

        // 优化：对于少量方法（<=8），使用栈分配避免堆操作
        const STACK_BUF_SIZE: usize = 8;
        let methods = if mlen as usize <= STACK_BUF_SIZE {
            let mut buf = [0u8; STACK_BUF_SIZE];
            r.read_exact(&mut buf[..mlen as usize])?;
            buf[..mlen as usize].iter().map(|&b| AuthMethod::from(b)).collect()
        } else {
            let mut buf = vec![0; mlen as usize];
            r.read_exact(&mut buf)?;
            buf.into_iter().map(AuthMethod::from).collect()
        };

        Ok(Self { methods })
    }

    fn write_to_buf<B: bytes::BufMut>(&self, buf: &mut B) {
        buf.put_u8(Version::V5.into());
        buf.put_u8(self.methods.len() as u8);

        let methods = self.methods.iter().map(u8::from).collect::<Vec<u8>>();
        buf.put_slice(&methods);
    }

    fn len(&self) -> usize {
        2 + self.methods.len()
    }
}

#[cfg(feature = "tokio")]
#[async_trait::async_trait]
impl AsyncStreamOperation for Request {
    async fn retrieve_from_async_stream<R>(r: &mut R) -> std::io::Result<Self>
    where
        R: AsyncRead + Unpin + Send + ?Sized,
    {
        let ver = Version::try_from(r.read_u8().await?)?;

        if ver != Version::V5 {
            let err = format!("Unsupported SOCKS version {0:#x}", u8::from(ver));
            return Err(std::io::Error::new(std::io::ErrorKind::Unsupported, err));
        }

        let mlen = r.read_u8().await?;

        // 优化：对于少量方法（<=8），使用栈分配避免堆操作
        const STACK_BUF_SIZE: usize = 8;
        let methods = if mlen as usize <= STACK_BUF_SIZE {
            let mut buf = [0u8; STACK_BUF_SIZE];
            r.read_exact(&mut buf[..mlen as usize]).await?;
            buf[..mlen as usize].iter().map(|&b| AuthMethod::from(b)).collect()
        } else {
            let mut buf = vec![0; mlen as usize];
            r.read_exact(&mut buf).await?;
            buf.into_iter().map(AuthMethod::from).collect()
        };

        Ok(Self { methods })
    }
}
